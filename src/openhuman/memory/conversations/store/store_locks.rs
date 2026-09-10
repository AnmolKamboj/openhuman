//! Lock registry for the JSONL conversation store.
//!
//! A root owns shared metadata (`threads.jsonl`) and many independent message
//! files. Keeping those synchronization scopes separate lets unrelated agent
//! sessions write their message files concurrently while preserving atomic
//! metadata appends and purge semantics.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, LazyLock, Weak};

use parking_lot::{Mutex, RwLock};

#[derive(Debug, Default)]
pub(super) struct StoreLocks {
    /// Ordinary operations take a read guard; purge takes the write guard.
    pub(super) lifecycle: RwLock<()>,
    /// Serializes reads and appends of the root's shared `threads.jsonl`.
    pub(super) metadata: Mutex<()>,
    threads: Mutex<HashMap<String, Arc<Mutex<()>>>>,
}

impl StoreLocks {
    pub(super) fn thread(&self, thread_id: &str) -> Arc<Mutex<()>> {
        let mut locks = self.threads.lock();
        Arc::clone(
            locks
                .entry(thread_id.to_string())
                .or_insert_with(|| Arc::new(Mutex::new(()))),
        )
    }
}

/// Separate `ConversationStore::new` calls for the same root must coordinate.
/// Weak entries avoid retaining one lock set for every temporary workspace a
/// long-running process has ever touched.
static ROOTS: LazyLock<Mutex<HashMap<PathBuf, Weak<StoreLocks>>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

pub(super) fn for_root(root: &Path) -> Arc<StoreLocks> {
    let mut roots = ROOTS.lock();
    if let Some(existing) = roots.get(root).and_then(Weak::upgrade) {
        return existing;
    }
    let locks = Arc::new(StoreLocks::default());
    roots.insert(root.to_path_buf(), Arc::downgrade(&locks));
    locks
}
