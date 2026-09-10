use super::*;

#[test]
fn one_hundred_agent_threads_append_without_loss_or_corruption() {
    use std::sync::{Arc, Barrier};

    let temp = TempDir::new().unwrap();
    let store = ConversationStore::new(temp.path().to_path_buf());
    let created_at = "2026-09-10T00:00:00Z".to_string();

    for index in 0..100 {
        store
            .ensure_thread(CreateConversationThread {
                parent_thread_id: None,
                id: format!("agent-{index}"),
                title: format!("Agent {index}"),
                created_at: created_at.clone(),
                labels: None,
                personality_id: None,
            })
            .unwrap();
    }

    let barrier = Arc::new(Barrier::new(101));
    let mut writers = Vec::with_capacity(100);
    for index in 0..100 {
        let store = store.clone();
        let barrier = Arc::clone(&barrier);
        let created_at = created_at.clone();
        writers.push(std::thread::spawn(move || {
            barrier.wait();
            store.append_message(
                &format!("agent-{index}"),
                ConversationMessage {
                    id: format!("message-{index}"),
                    content: format!("reply from agent {index}"),
                    message_type: "text".to_string(),
                    extra_metadata: serde_json::json!({}),
                    sender: "assistant".to_string(),
                    created_at,
                },
            )
        }));
    }
    barrier.wait();

    for writer in writers {
        writer.join().expect("writer panicked").expect("append");
    }

    let threads = store.list_threads().unwrap();
    assert_eq!(threads.len(), 100);
    assert!(threads.iter().all(|thread| thread.message_count == 1));
    for index in 0..100 {
        let messages = store.get_messages(&format!("agent-{index}")).unwrap();
        assert_eq!(messages.len(), 1);
        assert_eq!(messages[0].id, format!("message-{index}"));
    }
}
