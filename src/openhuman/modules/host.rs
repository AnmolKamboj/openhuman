//! The module host: a broker, a connection, and the loaded-module table.
//!
//! # Why modules get their own bus
//!
//! The core already runs a bus — `core::bus::BUS`, a `OnceBus<DomainEvent>` — but
//! it cannot be reused here. `OnceBus::init_in_process` constructs its `Broker`
//! internally and never hands it out, and `ModuleHost::new` needs a `Broker` to
//! attach loaded modules to. Rather than change tinybus to expose it, modules run
//! on a second in-process broker of their own.
//!
//! That is a real limitation and worth naming: a module on this bus cannot
//! publish a `DomainEvent`, so it can serve requests but cannot participate in
//! the core's event flow. For a codec that is exactly right — a document writer
//! has nothing to say to the subconscious. A module that did need to emit events
//! would need `OnceBus` to share its broker first.
//!
//! # What loading a module means
//!
//! `dlopen` runs code before anything can inspect it. tinybus's ABI descriptor,
//! manifest and digest gates decide whether a module is *admitted*, not whether
//! it is *safe*: once loaded it can read and write this process's memory, and a
//! native fault in it takes the core down. It is first-party code that happens to
//! ship separately. tinybus also never unloads a library, so a module that fails
//! is failed until restart — which is why [`super::ops`] caches failures instead
//! of retrying.
//!
//! Everything here is created once and lives for the process. There is no
//! shutdown path because there is nothing a shutdown could reclaim.

use std::sync::OnceLock;

use tinybus::broker::Broker;
use tinybus::module::ModuleHost;
use tinybus::transport::memory::MemoryBus;
use tinybus::{Connection, Proxy};

/// The module bus, built once on first use.
static RUNTIME: OnceLock<ModuleRuntime> = OnceLock::new();

/// The broker, the host's own connection to it, and the module loader.
pub struct ModuleRuntime {
    /// The loader. Owns every admitted module for the process lifetime.
    host: ModuleHost,
    /// This process's client connection, used to call into loaded modules.
    connection: Connection,
}

impl ModuleRuntime {
    /// The loader.
    #[must_use]
    pub fn host(&self) -> &ModuleHost {
        &self.host
    }

    /// The connection modules are called over.
    #[must_use]
    pub fn connection(&self) -> &Connection {
        &self.connection
    }

    /// A proxy for one object on a loaded module.
    ///
    /// # Errors
    ///
    /// Returns an error if `bus_name` or `object_path` is not well formed, which
    /// for a registry entry means the table is wrong rather than the module.
    pub fn proxy(&self, bus_name: &str, object_path: &str) -> tinybus::Result<Proxy> {
        self.connection.proxy(bus_name, object_path, bus_name)
    }
}

/// The process-wide module runtime, standing it up on first use.
///
/// # Errors
///
/// Returns an error if the broker's in-memory transport cannot be connected,
/// which in practice means the tokio runtime is shutting down.
///
/// # Panics
///
/// Does not panic: a lost initialisation race reuses the winner's runtime.
pub async fn runtime() -> tinybus::Result<&'static ModuleRuntime> {
    if let Some(existing) = RUNTIME.get() {
        return Ok(existing);
    }

    let transport = MemoryBus::new();
    let broker = Broker::new();
    // The broker task is deliberately not retained. It lives as long as the
    // process, and holding the handle would only offer an abort that must never
    // be called: a module whose transport disappears faults, and a faulted
    // module cannot be reloaded without a restart.
    broker.spawn(transport.clone());

    // Strict admission. tinybus otherwise only warns when a module's toolchain
    // differs from the host's, and a rustc mismatch across a `cdylib` boundary
    // is the kind of thing that works until it corrupts something. A refusal at
    // load time is a clear message; the alternative is a field-only crash.
    let host = ModuleHost::new(broker).strict(true);
    let connection = Connection::connect(transport.connect().await?).await?;

    let runtime = ModuleRuntime { host, connection };
    // A concurrent caller may have won the race. Its runtime is equivalent, so
    // take the winner's and let ours drop.
    Ok(RUNTIME.get_or_init(|| runtime))
}

/// Whether the module runtime has been stood up.
///
/// Lets status reporting answer without starting a broker as a side effect of
/// being asked a question.
#[must_use]
pub fn is_started() -> bool {
    RUNTIME.get().is_some()
}

#[cfg(test)]
mod tests {
    use super::{is_started, runtime};

    #[tokio::test]
    async fn the_runtime_is_a_singleton() {
        let first = runtime().await.expect("runtime should start");
        assert!(is_started());
        let second = runtime().await.expect("runtime should be reused");
        assert!(
            std::ptr::eq(first, second),
            "runtime() handed out two different runtimes"
        );
    }

    #[tokio::test]
    async fn a_proxy_for_an_unloaded_module_is_constructible_but_unanswered() {
        // Building a proxy is a local operation — it validates names and nothing
        // else. Nothing has claimed the name, so the call fails rather than
        // hanging, which is what makes `ensure_loaded` worth having.
        let runtime = runtime().await.expect("runtime should start");
        let proxy = runtime
            .proxy(
                "ai.tinyhumans.tinydocs.Documents",
                "/ai/tinyhumans/tinydocs/Documents",
            )
            .expect("registry names should be well formed");
        let result: tinybus::Result<serde_json::Value> = proxy.call("GenerateDocx", ()).await;
        assert!(result.is_err(), "an unloaded module should not answer");
    }

    #[tokio::test]
    async fn a_malformed_name_is_rejected_locally() {
        let runtime = runtime().await.expect("runtime should start");
        assert!(runtime.proxy("not a bus name", "/nope").is_err());
    }
}
