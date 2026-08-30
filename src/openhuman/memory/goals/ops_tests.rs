use super::*;

#[tokio::test]
async fn list_add_edit_delete_flow() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();

    // Starts empty.
    let listed = list(dir).await.unwrap();
    assert!(listed.value.is_empty());

    // Add returns an id and the updated list.
    let added = add(dir, "ship the desktop app").await.unwrap();
    let id = added.value.id.clone();
    assert_eq!(added.value.goals.items.len(), 1);

    // Edit by id.
    let edited = edit(dir, &id, "ship the app to all platforms")
        .await
        .unwrap();
    assert_eq!(edited.value.items[0].text, "ship the app to all platforms");

    // Delete by id leaves the list empty.
    let deleted = delete(dir, &id).await.unwrap();
    assert!(deleted.value.is_empty());

    // Unknown id is an error.
    assert!(edit(dir, "nope", "x").await.is_err());
    assert!(delete(dir, "nope").await.is_err());
}
