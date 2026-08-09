//! Tests for [`ProfileStore`].
//!
//! The two interesting methods are the ones that replaced hand-rolled SQL in
//! `memory/sync/composio/providers/profile.rs`. A subtly wrong reimplementation
//! of `skill_identity_matches` makes the entity matcher stop recognising the
//! user, which degrades silently rather than erroring — so the oracle here is
//! the literal SQL that was replaced, executed against the same connection,
//! rather than my reading of it.

use super::*;
use crate::openhuman::memory::store::profile::PROFILE_INIT_SQL;

fn seeded_store() -> ProfileStore {
    let conn = Connection::open_in_memory().unwrap();
    conn.execute_batch(PROFILE_INIT_SQL).unwrap();
    let store = ProfileStore::for_tests(Arc::new(Mutex::new(conn)));

    let rows = [
        (
            "skill-gmail-default-email",
            "skill:gmail:default:email",
            "user@example.com",
        ),
        (
            "skill-slack-c123-handle",
            "skill:slack:c123:handle",
            "userhandle",
        ),
        (
            "skill-slack-c123-email",
            "skill:slack:c123:email",
            "work@example.com",
        ),
    ];
    for (facet_id, key, value) in rows {
        store
            .upsert_provider_facet(
                facet_id,
                &FacetType::Workflow,
                key,
                value,
                0.9,
                None,
                1000.0,
            )
            .unwrap();
    }
    store
}

/// The exact query string from the pre-refactor
/// `is_self_identity` / `is_self_identity_any_toolkit`, run here so the
/// assertion compares against the code that was replaced.
fn legacy_like_query(store: &ProfileStore, key_pattern: &str, canonical: &str) -> bool {
    let conn = store.conn.lock();
    conn.query_row(
        "SELECT 1 FROM user_profile
          WHERE facet_type = 'skill'
            AND key LIKE ?1
            AND value = ?2
          LIMIT 1",
        params![key_pattern, canonical],
        |_| Ok(()),
    )
    .is_ok()
}

#[test]
fn skill_identity_matches_agrees_with_the_legacy_like_query() {
    let store = seeded_store();
    let cases = [
        ("skill:gmail:%:email", "user@example.com"), // exact toolkit hit
        ("skill:slack:%:email", "user@example.com"), // wrong toolkit
        ("skill:%:%:email", "user@example.com"),     // cross-toolkit hit
        ("skill:%:%:email", "other@example.com"),    // value miss
        ("skill:%:%:phone", "user@example.com"),     // kind miss
        ("skill:gmail:%:handle", ""),                // empty value
        ("skill:slack:%:handle", "userhandle"),      // second toolkit hit
    ];
    for (pattern, value) in cases {
        let legacy = legacy_like_query(&store, pattern, value);
        assert_eq!(
            store.skill_identity_matches(pattern, value),
            legacy,
            "divergence for pattern={pattern:?} value={value:?}"
        );
    }
    // Non-vacuity: at least one case must actually be a hit, or the loop above
    // would pass with a method that always returns false.
    assert!(store.skill_identity_matches("skill:%:%:email", "user@example.com"));
}

#[test]
fn delete_by_facet_id_removes_exactly_one_row() {
    let store = seeded_store();
    assert_eq!(store.facets_by_type(&FacetType::Workflow).unwrap().len(), 3);

    assert!(store.delete_by_facet_id("skill-slack-c123-email").unwrap());

    let survivors = store.facets_by_type(&FacetType::Workflow).unwrap();
    let ids: Vec<&str> = survivors.iter().map(|f| f.facet_id.as_str()).collect();
    assert_eq!(survivors.len(), 2, "deleted more than one row: {ids:?}");
    assert!(ids.contains(&"skill-gmail-default-email"), "{ids:?}");
    assert!(ids.contains(&"skill-slack-c123-handle"), "{ids:?}");

    assert!(
        !store.delete_by_facet_id("skill-does-not-exist").unwrap(),
        "deleting an unknown facet_id must report false"
    );
}

#[test]
fn facet_cache_surface_round_trips_through_the_store() {
    let store = seeded_store();
    let facet = store.get("skill:gmail:default:email").unwrap();
    assert_eq!(facet.map(|f| f.value).as_deref(), Some("user@example.com"));
    assert_eq!(store.list_all().unwrap().len(), 3);
    assert!(store.delete("skill:gmail:default:email").unwrap());
    assert_eq!(store.list_all().unwrap().len(), 2);
}
