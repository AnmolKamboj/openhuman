use std::sync::atomic::{AtomicBool, Ordering};

use super::*;

#[tokio::test]
async fn static_token_connection_clears_identity_state_after_disconnect() {
    let manager = SocketManager::new();
    manager
        .connect("http://127.0.0.1:1", "opaque-token")
        .await
        .unwrap();
    let cleared = AtomicBool::new(false);
    connect_static_using(&manager, "http://127.0.0.1:1", "replacement", || {
        assert_eq!(
            manager.get_state().status,
            crate::openhuman::platform::socket::types::ConnectionStatus::Disconnected
        );
        cleared.store(true, Ordering::SeqCst);
    })
    .await
    .unwrap();
    assert!(cleared.load(Ordering::SeqCst));
}

// ── Redundant-connect suppression (#6181) ──────────────────────────
use std::sync::atomic::AtomicUsize;
use std::sync::Arc;

use crate::openhuman::platform::socket::token_provider::static_token_provider;
use crate::openhuman::platform::socket::types::ConnectionStatus;

const TOKEN_A: &str = "session-token-a";
const TOKEN_B: &str = "session-token-b";

/// EIO v4 mock that counts accepts and completes enough of the handshake for
/// `SocketManager` to reach `Connected`, then idles. Accepting in a loop is the
/// point: a duplicate connect shows up as a second accept.
async fn spawn_accept_counting_eio_server() -> (Arc<AtomicUsize>, std::net::SocketAddr) {
    use futures_util::{SinkExt, StreamExt};
    use tokio::net::TcpListener;
    use tokio_tungstenite::{accept_async, tungstenite::Message as WsMessage};

    let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind");
    let addr = listener.local_addr().expect("addr");
    let accepts = Arc::new(AtomicUsize::new(0));
    let counter = Arc::clone(&accepts);
    tokio::spawn(async move {
        while let Ok((stream, _)) = listener.accept().await {
            counter.fetch_add(1, Ordering::SeqCst);
            tokio::spawn(async move {
                let Ok(ws) = accept_async(stream).await else {
                    return;
                };
                let (mut write, mut read) = ws.split();
                // Long ping intervals so nothing reconnects on its own mid-test.
                let open = r#"0{"sid":"mock-eio-sid","upgrades":[],"pingInterval":30000,"pingTimeout":30000}"#;
                let _ = write.send(WsMessage::Text(open.into())).await;
                let _ = read.next().await; // client SIO CONNECT
                let _ = write
                    .send(WsMessage::Text(r#"40{"sid":"mock-sio-sid"}"#.into()))
                    .await;
                while let Some(Ok(_)) = read.next().await {}
            });
        }
    });
    (accepts, addr)
}

async fn wait_for_connected(manager: &SocketManager) {
    for _ in 0..200 {
        if manager.is_connected() {
            return;
        }
        tokio::time::sleep(std::time::Duration::from_millis(25)).await;
    }
    panic!("socket never reached Connected");
}

/// Wait out the window in which a duplicate connect would reach the server.
///
/// `connect_with_provider` returns as soon as the background loop is spawned, so
/// reading the accept counter straight afterwards would pass even when a second
/// session is on its way — a vacuous assertion. Returns early the moment a
/// second accept does show up, so the fix's happy path is the only one that pays
/// the full wait.
async fn settle_for_a_second_accept(accepts: &AtomicUsize) {
    for _ in 0..40 {
        if accepts.load(Ordering::SeqCst) > 1 {
            return;
        }
        tokio::time::sleep(std::time::Duration::from_millis(25)).await;
    }
}

/// Stand-in for the core's bootstrap auto-connect (`spawn_socket_auto_connect`).
async fn bootstrap_auto_connect(manager: &SocketManager, url: &str, token: &str) {
    manager
        .connect_with_provider(url, static_token_provider(token.to_string()))
        .await
        .unwrap();
    wait_for_connected(manager).await;
}

/// #6181: on a cold start the core auto-connects and the renderer's
/// `socket_connect_with_session` RPC follows a couple of seconds later with the
/// same backend URL and the same stored token. The second one must reuse the
/// live socket rather than tear it down and open a fresh EIO session.
#[tokio::test]
async fn a_redundant_session_connect_reuses_the_live_socket() {
    let (accepts, addr) = spawn_accept_counting_eio_server().await;
    let url = format!("http://{addr}");
    let manager = SocketManager::new();

    bootstrap_auto_connect(&manager, &url, TOKEN_A).await;

    let rebound = AtomicBool::new(false);
    let state = connect_with_session_using(
        &manager,
        &url,
        TOKEN_A,
        static_token_provider(TOKEN_A.to_string()),
        || rebound.store(true, Ordering::SeqCst),
    )
    .await
    .unwrap();

    settle_for_a_second_accept(&accepts).await;
    assert_eq!(
        accepts.load(Ordering::SeqCst),
        1,
        "a redundant connect for an identical identity opened a duplicate EIO session"
    );
    assert!(
        !rebound.load(Ordering::SeqCst),
        "identity rebind ran for a session that was already live"
    );
    // The caller also gets the live socket's state back, not the `Connecting`
    // of a handshake that has only just been kicked off.
    assert_eq!(state.status, ConnectionStatus::Connected);
}

/// The other half of the guard: an account switch keeps the same backend URL but
/// arrives with a different token, and must still disconnect, rebind the
/// identity-bound workflow plane, and reconnect.
#[tokio::test]
async fn a_session_connect_with_a_different_token_still_rebinds() {
    let (accepts, addr) = spawn_accept_counting_eio_server().await;
    let url = format!("http://{addr}");
    let manager = SocketManager::new();

    bootstrap_auto_connect(&manager, &url, TOKEN_A).await;

    let rebound = AtomicBool::new(false);
    connect_with_session_using(
        &manager,
        &url,
        TOKEN_B,
        static_token_provider(TOKEN_B.to_string()),
        || rebound.store(true, Ordering::SeqCst),
    )
    .await
    .unwrap();
    wait_for_connected(&manager).await;

    assert!(
        rebound.load(Ordering::SeqCst),
        "a new session token must still rebind the identity-bound workflow plane"
    );
    assert_eq!(
        accepts.load(Ordering::SeqCst),
        2,
        "a new session token must open a fresh EIO session"
    );
}

/// `is_live_for` is identity-scoped, not merely "am I connected": a different
/// backend URL is a different identity, and a disconnect clears the record so a
/// later connect is never skipped against a dead socket.
#[tokio::test]
async fn is_live_for_is_scoped_to_url_and_token_and_cleared_on_disconnect() {
    let (_accepts, addr) = spawn_accept_counting_eio_server().await;
    let url = format!("http://{addr}");
    let manager = SocketManager::new();

    bootstrap_auto_connect(&manager, &url, TOKEN_A).await;

    assert!(manager.is_live_for(&url, TOKEN_A));
    assert!(!manager.is_live_for(&url, TOKEN_B));
    assert!(!manager.is_live_for("http://127.0.0.1:1", TOKEN_A));

    manager.disconnect().await.unwrap();
    assert!(!manager.is_live_for(&url, TOKEN_A));
}
