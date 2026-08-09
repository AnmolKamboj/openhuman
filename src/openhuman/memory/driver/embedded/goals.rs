//! [`MemoryGoals`] for the embedded driver — the agent's long-term goals
//! document.
//!
//! The smallest family in the contract, and the only one with **no type
//! conversion at all**: `tinycortex::memory::goals::types` is a `pub use` of
//! `tinycortex_api::goals` (see `vendor/tinycortex/src/memory/goals/mod.rs`),
//! so the [`GoalsDoc`] the host store reads and writes *is* the contract's
//! [`GoalsDoc`]. `goals_doc_is_the_contract_type` pins that — if the engine
//! ever forks the type, this file should stop compiling here rather than
//! somewhere confusing.
//!
//! ## Both directions go through the host store, not the engine
//!
//! `store::load` / `store::save` are the host's own thin wrappers over the
//! engine's goals store. They own the on-disk location
//! (`<workspace>/MEMORY_GOALS.md`) and the item/character caps, and going
//! through them keeps this driver from being a second place that knows either.
//!
//! ## `set_goals` takes ownership; `save` needs `&mut`
//!
//! `store::save` trims the document in place to `GOALS_MAX_ITEMS` /
//! `GOALS_FILE_MAX_CHARS`. The contract hands the document over by value and
//! returns `()`, so the trimmed copy is simply dropped — the caller's next
//! [`MemoryGoals::goals`] reads whatever was actually persisted, which is the
//! honest answer.
//!
//! ## Why nothing maps to [`MemoryError::Invalid`]
//!
//! The contract reserves `Invalid` for "a document the driver refuses (e.g.
//! over its own item cap)". The engine *does* have those rejections, but
//! `goals::store` flattens every engine error to `String` via `to_string()`, so
//! by the time it reaches this file nothing is machine-readable. String-matching
//! the message to recover the class would be worse than the honest
//! [`MemoryError::Other`]: it would silently reclassify on any wording change.
//! Making this typed needs `goals/store.rs` to stop flattening, which is a host
//! change outside this step.

use async_trait::async_trait;
use tinycortex_api::error::MemoryError;
use tinycortex_api::goals::GoalsDoc;
use tinycortex_api::provider::MemoryGoals;

use crate::openhuman::memory::goals::store;

use super::{host_error, EmbeddedMemoryProvider};

#[async_trait]
impl MemoryGoals for EmbeddedMemoryProvider {
    async fn goals(&self) -> Result<GoalsDoc, MemoryError> {
        log::debug!(
            "[memory:driver:embedded] goals workspace={}",
            self.workspace_dir().display()
        );
        // A missing `MEMORY_GOALS.md` maps to an empty document inside
        // `store::load`, so the contract's "no goals is not NotFound" rule
        // holds without anything here.
        store::load(self.workspace_dir())
            .await
            .map_err(|error| host_error("goals", error))
    }

    async fn set_goals(&self, goals: GoalsDoc) -> Result<(), MemoryError> {
        let mut doc = goals;
        log::debug!(
            "[memory:driver:embedded] set_goals workspace={} items={}",
            self.workspace_dir().display(),
            doc.items.len()
        );
        store::save(self.workspace_dir(), &mut doc)
            .await
            .map_err(|error| host_error("set_goals", error))
    }
}

#[cfg(test)]
#[path = "goals_tests.rs"]
mod tests;
