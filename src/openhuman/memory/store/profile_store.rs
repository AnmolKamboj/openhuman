//! `ProfileStore` — the only typed door onto the `user_profile` table.
//!
//! Before this type existed, `MemoryClient::profile_conn()` handed a raw
//! `Arc<Mutex<rusqlite::Connection>>` to three domains outside the memory
//! family (`agent/learning/*`, `memory/sync/composio/providers/profile.rs`),
//! two of which wrote SQL inline at the call site. Every SQL statement against
//! profile/facet rows now lives either here or in
//! [`super::namespace_store::profile`], both inside `crate::openhuman::memory`;
//! callers outside the family hold this handle and never a `Connection`.
//!
//! **This is not a guard win.** The profile/facet tables have no capability
//! family in the `tinycortex_api` contract, so reads and writes through this
//! type still run beneath [`crate::openhuman::memory::guard::MemoryGuard`]'s
//! seven policy steps: no tier check, no source-scope predicate, no taint
//! stamping, no redaction, no budget, no audit event. What changed is the shape
//! of the door — raw SQLite reachable from three domains became one typed store
//! whose confinement the compiler enforces.

use parking_lot::Mutex;
use rusqlite::{params, Connection};
use std::sync::Arc;

use super::namespace_store::profile::{self, FacetType, ProfileFacet, UserState};

/// Typed access to the `user_profile` table.
///
/// Cheap to clone — it is an `Arc` over the same connection `MemoryClient`
/// owns, so clones share one lock.
#[derive(Clone)]
pub struct ProfileStore {
    conn: Arc<Mutex<Connection>>,
}

impl ProfileStore {
    /// The single production construction site is
    /// [`super::MemoryClient::profile_store`].
    pub(in crate::openhuman::memory) fn from_conn(conn: Arc<Mutex<Connection>>) -> Self {
        Self { conn }
    }

    /// Test-only: build a store over a caller-owned in-memory database.
    ///
    /// Not a hole — the caller already holds the `Connection`, so this hands
    /// out nothing a [`super::MemoryClient`] owns. Confinement is about not
    /// *extracting* the client's connection, and `profile_conn()` stays
    /// `pub(in crate::openhuman::memory)`.
    ///
    /// Deliberately **not** `#[cfg(test)]`: integration tests under `tests/`
    /// link the lib compiled without `cfg(test)`, so a test-gated constructor
    /// is invisible to them — which is exactly how
    /// `tests/learning_phase4_integration_test.rs` was left uncompilable when
    /// `FacetCache::new` changed shape. `#[doc(hidden)]` keeps it off the
    /// public docs without hiding it from the linker.
    #[doc(hidden)]
    pub fn for_tests(conn: Arc<Mutex<Connection>>) -> Self {
        Self { conn }
    }

    // ── Facet-cache surface ───────────────────────────────────────────────

    /// List all facets with `state = 'active'`, ordered by stability descending.
    pub fn list_active(&self) -> anyhow::Result<Vec<ProfileFacet>> {
        profile::profile_select_active(&self.conn)
    }

    /// List all facets (all states), ordered by stability descending.
    pub fn list_all(&self) -> anyhow::Result<Vec<ProfileFacet>> {
        profile::profile_select_all(&self.conn)
    }

    /// Fetch a single facet by its full key (e.g. `"style/verbosity"`).
    pub fn get(&self, key: &str) -> anyhow::Result<Option<ProfileFacet>> {
        profile::profile_get_by_key(&self.conn, key)
    }

    /// Upsert a fully-formed facet row (rebuild path).
    pub fn upsert_full(&self, facet: &ProfileFacet) -> anyhow::Result<()> {
        profile::profile_upsert_full(&self.conn, facet)
    }

    /// Override the `user_state` of a facet. `Ok(true)` if a row was updated.
    pub fn set_user_state(&self, key: &str, user_state: UserState) -> anyhow::Result<bool> {
        profile::profile_set_user_state(&self.conn, key, user_state)
    }

    /// Delete a facet by key. Returns `true` if a row was removed.
    pub fn delete(&self, key: &str) -> anyhow::Result<bool> {
        profile::profile_delete_by_key(&self.conn, key)
    }

    /// Delete all `Dropped`-state facets whose stability is below `threshold`.
    pub fn drop_below_threshold(&self, threshold: f64) -> anyhow::Result<usize> {
        profile::profile_delete_below_threshold(&self.conn, threshold)
    }

    // ── Provider-identity surface ─────────────────────────────────────────

    /// Confidence-aware upsert of one provider-sourced facet row.
    #[allow(clippy::too_many_arguments)]
    pub fn upsert_provider_facet(
        &self,
        facet_id: &str,
        facet_type: &FacetType,
        key: &str,
        value: &str,
        confidence: f64,
        segment_id: Option<&str>,
        now: f64,
    ) -> anyhow::Result<()> {
        profile::profile_upsert(
            &self.conn, facet_id, facet_type, key, value, confidence, segment_id, now,
        )
    }

    /// Load every facet of `facet_type`, ordered by evidence count descending.
    pub fn facets_by_type(&self, facet_type: &FacetType) -> anyhow::Result<Vec<ProfileFacet>> {
        profile::profile_facets_by_type(&self.conn, facet_type)
    }

    /// True if any [`FacetType::Workflow`] (`"skill"`) row's key matches
    /// `key_pattern` (a SQL `LIKE` pattern) with exactly `canonical_value`.
    ///
    /// Encapsulates the two hand-rolled `SELECT 1 … LIKE` queries the composio
    /// provider used to write inline. Deliberately infallible: the callers are
    /// "is this row the user?" predicates whose only sane answer on a database
    /// error is "no", which is what the raw `.is_ok()` gave before.
    pub fn skill_identity_matches(&self, key_pattern: &str, canonical_value: &str) -> bool {
        let conn = self.conn.lock();
        let matched = conn
            .query_row(
                "SELECT 1 FROM user_profile
                  WHERE facet_type = ?1
                    AND key LIKE ?2
                    AND value = ?3
                  LIMIT 1",
                params![FacetType::Workflow.as_str(), key_pattern, canonical_value],
                |_| Ok(()),
            )
            .is_ok();
        // Facet values are user PII (emails, phone numbers, handles) — log the
        // pattern and the verdict, never the value.
        tracing::debug!(
            pattern = %key_pattern,
            matched,
            "[memory::profile_store] skill_identity_matches"
        );
        matched
    }

    /// Delete exactly one row by `facet_id`. `Ok(true)` if a row was removed.
    pub fn delete_by_facet_id(&self, facet_id: &str) -> anyhow::Result<bool> {
        let conn = self.conn.lock();
        let removed = conn.execute(
            "DELETE FROM user_profile WHERE facet_id = ?1",
            params![facet_id],
        )?;
        tracing::debug!(
            facet_id = %facet_id,
            removed,
            "[memory::profile_store] delete_by_facet_id"
        );
        Ok(removed > 0)
    }
}

#[cfg(test)]
#[path = "profile_store_tests.rs"]
mod tests;
