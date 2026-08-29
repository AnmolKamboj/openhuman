//! Domain RPC handlers for people. Adapter handlers in `schemas.rs`
//! parse params and delegate here.
//!
//! # These take the driver's people family, not a store
//!
//! They used to take `&PeopleStore` and reach the engine in-process. The store
//! lives behind the loaded module now, so each handler takes
//! `&dyn MemoryPeople` — the guarded family off the bound driver — and the
//! ranking, scoring and address-book work happens engine-side.
//!
//! What stays here is the **wire shape**: these payloads are a published RPC
//! surface (`people.*`) and the field names below are a compatibility surface,
//! so the JSON is assembled here rather than serialising contract types
//! directly. `schemas_tests` pins it.

use serde_json::{json, Value};

use crate::openhuman::memory::api::provider::{MemoryPeople, PersonHandle, PersonRecord};
use crate::rpc::RpcOutcome;

/// Render one person plus their score into the published `people.*` shape.
fn person_json(
    person: &PersonRecord,
    score: &crate::openhuman::memory::api::provider::PersonScore,
) -> Value {
    let handles: Vec<Value> = person
        .handles
        .iter()
        .map(|handle| {
            let (kind, value) = match handle {
                PersonHandle::IMessage(v) => ("imessage", v),
                PersonHandle::Email(v) => ("email", v),
                PersonHandle::DisplayName(v) => ("display_name", v),
            };
            json!({ "kind": kind, "value": value })
        })
        .collect();
    json!({
        "person_id": person.id,
        "display_name": person.display_name,
        "primary_email": person.primary_email,
        "primary_phone": person.primary_phone,
        "handles": handles,
        "score": score.score,
        "components": {
            "recency": score.recency,
            "frequency": score.frequency,
            "reciprocity": score.reciprocity,
            "depth": score.depth,
        },
        "interaction_count": score.interaction_count,
    })
}

/// List people ranked by composite score, highest first.
///
/// The ranking is the driver's — this no longer sorts. The engine holds the
/// interactions the score is computed from, so ranking host-side would mean
/// fetching every person's history across the bus to re-derive an order the
/// driver already produced.
pub async fn handle_list(
    people: &dyn MemoryPeople,
    limit: usize,
) -> Result<RpcOutcome<Value>, String> {
    let limit = limit.clamp(1, 500);
    let ranked = people
        .list_people(Some(limit))
        .await
        .map_err(|e| format!("list: {e}"))?;
    let people_json: Vec<Value> = ranked
        .iter()
        .map(|entry| person_json(&entry.person, &entry.score))
        .collect();
    Ok(RpcOutcome::new(json!({ "people": people_json }), vec![]))
}

/// Resolve a handle to a person id. Mints on first sight when
/// `create_if_missing` is true.
pub async fn handle_resolve(
    people: &dyn MemoryPeople,
    handle: PersonHandle,
    create_if_missing: bool,
) -> Result<RpcOutcome<Value>, String> {
    let resolved = people
        .resolve_handle(&handle, create_if_missing)
        .await
        .map_err(|e| format!("resolve: {e}"))?;
    Ok(RpcOutcome::new(
        json!({
            "person_id": resolved.as_ref().map(|r| r.id.clone()),
            "created": resolved.as_ref().is_some_and(|r| r.created),
        }),
        vec![],
    ))
}

/// Seed the people store from the system address book (CNContactStore on
/// macOS). Triggers the TCC Contacts permission prompt if not yet granted.
///
/// # `permission_denied` is always `false` now, and that is a real change
///
/// The contract deliberately reports a host without an address book — or
/// without permission to read it — as `seeded: 0` rather than as a distinct
/// error, because both mean the same thing to a caller and the alternative
/// leaks a platform detail into an engine-neutral contract. The field is kept
/// so the published shape does not change, but it can no longer become `true`.
/// Surfacing "grant Contacts access" needs a host-side permission probe, not a
/// memory-driver error.
pub async fn handle_refresh_address_book(
    people: &dyn MemoryPeople,
) -> Result<RpcOutcome<Value>, String> {
    let outcome = people
        .seed_from_address_book()
        .await
        .map_err(|e| format!("address_book: {e}"))?;
    log::debug!(
        "[people::rpc] refresh_address_book ok: seeded={} skipped={}",
        outcome.seeded,
        outcome.skipped
    );
    Ok(RpcOutcome::new(
        json!({
            "seeded": outcome.seeded,
            "skipped": outcome.skipped,
            "permission_denied": false,
        }),
        vec![],
    ))
}

/// Return the component-broken-down score for one person.
pub async fn handle_score(
    people: &dyn MemoryPeople,
    person_id: &str,
) -> Result<RpcOutcome<Value>, String> {
    let score = people
        .score_person(person_id)
        .await
        .map_err(|e| format!("score: {e}"))?
        .ok_or_else(|| format!("person not found: {person_id}"))?;
    Ok(RpcOutcome::new(
        json!({
            "person_id": person_id,
            "score": score.score,
            "components": {
                "recency": score.recency,
                "frequency": score.frequency,
                "reciprocity": score.reciprocity,
                "depth": score.depth,
            },
            "interaction_count": score.interaction_count,
        }),
        vec![],
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::openhuman::memory::api::error::MemoryError;
    use crate::openhuman::memory::api::provider::{
        AddressBookSeedOutcome, PersonInteraction, PersonRecord, PersonScore, RankedPerson,
        ResolvedPerson,
    };
    use async_trait::async_trait;

    /// A people family that answers with canned values.
    ///
    /// These tests cover what stayed **host-side** after the module port: the
    /// published `people.*` JSON shape, and that the driver's ordering is
    /// passed through rather than re-derived. Ranking and scoring themselves
    /// moved into the engine and are tested there — asserting them here would
    /// only re-test the fake.
    struct FakePeople {
        ranked: Vec<RankedPerson>,
        resolved: Option<ResolvedPerson>,
    }

    fn person(id: &str, name: &str) -> PersonRecord {
        PersonRecord {
            id: id.to_string(),
            display_name: Some(name.to_string()),
            primary_email: Some(format!("{name}@x.z").to_lowercase()),
            primary_phone: None,
            handles: vec![PersonHandle::Email(format!("{name}@x.z").to_lowercase())],
            created_at: "2026-01-01T00:00:00+00:00".into(),
            updated_at: "2026-01-01T00:00:00+00:00".into(),
        }
    }

    fn scored(score: f32, interactions: usize) -> PersonScore {
        PersonScore {
            recency: score,
            frequency: score,
            reciprocity: score,
            depth: score,
            score,
            interaction_count: interactions,
        }
    }

    #[async_trait]
    impl MemoryPeople for FakePeople {
        async fn list_people(
            &self,
            _limit: Option<usize>,
        ) -> Result<Vec<RankedPerson>, MemoryError> {
            Ok(self.ranked.clone())
        }
        async fn get_person(&self, _id: &str) -> Result<Option<PersonRecord>, MemoryError> {
            Ok(None)
        }
        async fn resolve_handle(
            &self,
            _handle: &PersonHandle,
            _create_if_missing: bool,
        ) -> Result<Option<ResolvedPerson>, MemoryError> {
            Ok(self.resolved.clone())
        }
        async fn add_handle_alias(
            &self,
            _id: &str,
            _handle: &PersonHandle,
        ) -> Result<(), MemoryError> {
            Ok(())
        }
        async fn score_person(&self, _id: &str) -> Result<Option<PersonScore>, MemoryError> {
            Ok(Some(scored(0.5, 7)))
        }
        async fn record_interaction(
            &self,
            _interaction: &PersonInteraction,
        ) -> Result<(), MemoryError> {
            Ok(())
        }
        async fn seed_from_address_book(&self) -> Result<AddressBookSeedOutcome, MemoryError> {
            Ok(AddressBookSeedOutcome {
                seeded: 3,
                skipped: 1,
            })
        }
    }

    #[tokio::test]
    async fn list_preserves_the_drivers_order_and_published_shape() {
        let people = FakePeople {
            ranked: vec![
                RankedPerson {
                    person: person("id-a", "Alice"),
                    score: scored(0.9, 10),
                },
                RankedPerson {
                    person: person("id-b", "Bob"),
                    score: scored(0.1, 1),
                },
            ],
            resolved: None,
        };
        let outcome = handle_list(&people, 10).await.unwrap();
        let arr = outcome.value["people"].as_array().unwrap();
        assert_eq!(arr.len(), 2);
        // Order is the driver's, not re-sorted here.
        assert_eq!(arr[0]["display_name"], "Alice");
        assert_eq!(arr[1]["display_name"], "Bob");
        // The published field set, which is a compatibility surface.
        assert_eq!(arr[0]["person_id"], "id-a");
        assert_eq!(arr[0]["interaction_count"], 10);
        // Compared with tolerance: the contract's score components are `f32`
        // and JSON numbers are `f64`, so 0.9f32 widens to 0.8999999761581421.
        // An exact assertion here would pin a widening artefact, not behaviour.
        let recency = arr[0]["components"]["recency"].as_f64().unwrap();
        assert!(
            (recency - 0.9).abs() < 1e-6,
            "recency component should round-trip: {recency}"
        );
        assert_eq!(arr[0]["handles"][0]["kind"], "email");
    }

    #[tokio::test]
    async fn list_does_not_re_sort_what_the_driver_returned() {
        // Deliberately out of score order: the driver is the ranking authority,
        // so a host-side sort would silently override it.
        let people = FakePeople {
            ranked: vec![
                RankedPerson {
                    person: person("id-low", "Low"),
                    score: scored(0.1, 1),
                },
                RankedPerson {
                    person: person("id-high", "High"),
                    score: scored(0.9, 9),
                },
            ],
            resolved: None,
        };
        let outcome = handle_list(&people, 10).await.unwrap();
        let arr = outcome.value["people"].as_array().unwrap();
        assert_eq!(arr[0]["display_name"], "Low");
        assert_eq!(arr[1]["display_name"], "High");
    }

    #[tokio::test]
    async fn resolve_without_create_returns_null_for_unknown() {
        let people = FakePeople {
            ranked: vec![],
            resolved: None,
        };
        let outcome = handle_resolve(&people, PersonHandle::Email("x@y.z".into()), false)
            .await
            .unwrap();
        assert!(outcome.value["person_id"].is_null());
        assert_eq!(outcome.value["created"], false);
    }

    #[tokio::test]
    async fn resolve_reports_whether_the_person_was_minted() {
        let people = FakePeople {
            ranked: vec![],
            resolved: Some(ResolvedPerson {
                id: "id-new".into(),
                created: true,
            }),
        };
        let outcome = handle_resolve(&people, PersonHandle::Email("x@y.z".into()), true)
            .await
            .unwrap();
        assert_eq!(outcome.value["person_id"], "id-new");
        assert_eq!(outcome.value["created"], true);
    }

    #[tokio::test]
    async fn score_carries_the_interaction_count_alongside_the_components() {
        let people = FakePeople {
            ranked: vec![],
            resolved: None,
        };
        let outcome = handle_score(&people, "id-a").await.unwrap();
        assert_eq!(outcome.value["person_id"], "id-a");
        assert_eq!(outcome.value["interaction_count"], 7);
        // 0.5 is exactly representable in both f32 and f64, so this one can be
        // compared directly.
        assert_eq!(outcome.value["components"]["depth"], 0.5);
    }

    /// `permission_denied` is now always `false` — see the handler docs.
    #[tokio::test]
    async fn refresh_address_book_reports_counts_and_never_a_permission_denial() {
        let people = FakePeople {
            ranked: vec![],
            resolved: None,
        };
        let outcome = handle_refresh_address_book(&people).await.unwrap();
        assert_eq!(outcome.value["seeded"], 3);
        assert_eq!(outcome.value["skipped"], 1);
        assert_eq!(outcome.value["permission_denied"], false);
    }
}
