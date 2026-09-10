//! Lock registry for the JSONL conversation store.
//!
//! A root owns shared metadata (`threads.jsonl`) and many independent message
//! files. Keeping those synchronization scopes separate lets unrelated agent
//! sessions write their message files concurrently while preserving atomic
//! metadata appends and purge semantics.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, LazyLock, Weak};

use parking_lot::{Mutex, RwLock};

#[derive(Debug, Default)]
pub(super) struct StoreLocks {
    /// Ordinary operations take a read guard; purge takes the write guard.
    pub(super) lifecycle: RwLock<()>,
    /// Serializes reads and appends of the root's shared `threads.jsonl`.
    pub(super) metadata: Mutex<()>,
    threads: Mutex<HashMap<String, Arc<Mutex<()>>>>,
    mutation_generation: AtomicU64,
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

    pub(super) fn mutation_generation(&self) -> u64 {
        self.mutation_generation.load(Ordering::Acquire)
    }

    pub(super) fn record_mutation(&self) {
        self.mutation_generation.fetch_add(1, Ordering::AcqRel);
    }

    /// Call only while holding the lifecycle write guard.
    pub(super) fn remove_thread(&self, thread_id: &str) {
        self.threads.lock().remove(thread_id);
    }

    /// Call only while holding the lifecycle write guard.
    pub(super) fn clear_threads(&self) {
        self.threads.lock().clear();
    }

    #[cfg(test)]
    pub(super) fn thread_count(&self) -> usize {
        self.threads.lock().len()
    }
}

/// Separate `ConversationStore::new` calls for the same root must coordinate.
/// Weak entries avoid retaining one lock set for every temporary workspace a
/// long-running process has ever touched.
static ROOTS: LazyLock<Mutex<HashMap<PathBuf, Weak<StoreLocks>>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

pub(super) fn for_root(root: &Path) -> Arc<StoreLocks> {
    let root = normalized_root(root);
    let mut roots = ROOTS.lock();
    if let Some(existing) = roots.get(&root).and_then(Weak::upgrade) {
        return existing;
    }
    let locks = Arc::new(StoreLocks::default());
    roots.insert(root, Arc::downgrade(&locks));
    locks
}

/// Resolve aliases even before the conversation directory itself exists.
/// Canonicalizing the nearest existing ancestor handles symlinks and `..`;
/// the missing suffix is then appended without touching the filesystem.
pub(super) fn normalized_root(root: &Path) -> PathBuf {
    let absolute;
    let root = if root.is_absolute() {
        root
    } else {
        absolute = std::env::current_dir()
            .map(|cwd| cwd.join(root))
            .unwrap_or_else(|_| root.to_path_buf());
        &absolute
    };
    if let Ok(canonical) = root.canonicalize() {
        return canonical;
    }

    let mut suffix = Vec::new();
    let mut ancestor = root;
    loop {
        if let Ok(canonical) = ancestor.canonicalize() {
            return suffix
                .iter()
                .rev()
                .fold(canonical, |path, component| path.join(component));
        }
        let Some(name) = ancestor.file_name() else {
            return root.to_path_buf();
        };
        suffix.push(name.to_os_string());
        let Some(parent) = ancestor.parent() else {
            return root.to_path_buf();
        };
        ancestor = parent;
    }
}
