//! `FacetCache` — thin wrapper over `user_profile_facets` for Phase 3.
//!
//! Provides typed read/write access to the facet table with class-aware helpers.
//! The stability detector uses this to persist the result of each rebuild cycle.
//! Prompt sections use [`FacetCache::list_active`] to read the ambient cache.

use crate::openhuman::agent::learning::candidate::FacetClass;
use crate::openhuman::memory::store::profile::{ProfileFacet, UserState};
use crate::openhuman::memory::store::ProfileStore;

/// Thin wrapper around the `user_profile` table.
///
/// A learning-side newtype over [`ProfileStore`], which owns the SQL. This
/// type exists because the class↔key vocabulary below (`FacetClass`) is agent
/// domain knowledge that must not move into the memory family; everything
/// else forwards straight to the store.
pub struct FacetCache {
    store: ProfileStore,
}

impl FacetCache {
    pub fn new(store: ProfileStore) -> Self {
        Self { store }
    }

    /// List all facets with `state = 'active'`, ordered by stability descending.
    pub fn list_active(&self) -> anyhow::Result<Vec<ProfileFacet>> {
        self.store.list_active()
    }

    /// List all facets (all states), ordered by stability descending.
    pub fn list_all(&self) -> anyhow::Result<Vec<ProfileFacet>> {
        self.store.list_all()
    }

    /// List active facets belonging to a specific class.
    ///
    /// Class is determined by the `key` prefix before the first `/`.
    pub fn list_by_class(&self, class: FacetClass) -> anyhow::Result<Vec<ProfileFacet>> {
        let prefix = format!("{}/", class_prefix(class));
        let all = self.list_active()?;
        Ok(all
            .into_iter()
            .filter(|f| f.key.starts_with(&prefix))
            .collect())
    }

    /// Fetch a single facet by its full key (e.g. `"style/verbosity"`).
    pub fn get(&self, key: &str) -> anyhow::Result<Option<ProfileFacet>> {
        self.store.get(key)
    }

    /// Upsert a fully-formed facet row (rebuild path).
    pub fn upsert(&self, facet: &ProfileFacet) -> anyhow::Result<()> {
        self.store.upsert_full(facet)
    }

    /// Override the `user_state` of a facet.
    ///
    /// Returns `Ok(true)` if a row was found and updated.
    pub fn set_user_state(&self, key: &str, user_state: UserState) -> anyhow::Result<bool> {
        self.store.set_user_state(key, user_state)
    }

    /// Delete a facet by key. Returns `true` if a row was removed.
    pub fn delete(&self, key: &str) -> anyhow::Result<bool> {
        self.store.delete(key)
    }

    /// Delete all `Dropped`-state facets whose stability is below `threshold`.
    ///
    /// Pinned facets are never deleted. Returns the number of rows removed.
    pub fn drop_below_threshold(&self, threshold: f64) -> anyhow::Result<usize> {
        self.store.drop_below_threshold(threshold)
    }
}

// ── Class ↔ key utilities ─────────────────────────────────────────────────────

/// Extract the [`FacetClass`] from a full key string (e.g. `"style/verbosity"` → `Style`).
///
/// Returns `None` for keys that don't have a recognised class prefix.
pub fn class_from_key(key: &str) -> Option<FacetClass> {
    let prefix = key.split('/').next()?;
    match prefix {
        "style" => Some(FacetClass::Style),
        "identity" => Some(FacetClass::Identity),
        "tooling" => Some(FacetClass::Tooling),
        "veto" => Some(FacetClass::Veto),
        "goal" => Some(FacetClass::Goal),
        "channel" => Some(FacetClass::Channel),
        _ => None,
    }
}

/// Build a full key from a class and a suffix (e.g. `(Style, "verbosity")` → `"style/verbosity"`).
pub fn key_with_class(class: FacetClass, suffix: &str) -> String {
    format!("{}/{suffix}", class_prefix(class))
}

/// Return the canonical key prefix for a [`FacetClass`].
pub fn class_prefix(class: FacetClass) -> &'static str {
    match class {
        FacetClass::Style => "style",
        FacetClass::Identity => "identity",
        FacetClass::Tooling => "tooling",
        FacetClass::Veto => "veto",
        FacetClass::Goal => "goal",
        FacetClass::Channel => "channel",
    }
}

// ── Facet state enum re-export (convenience for callers of this module) ───────

pub use crate::openhuman::memory::store::profile::{
    FacetState as CacheFacetState, UserState as CacheUserState,
};

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
#[path = "cache_tests.rs"]
mod tests;
