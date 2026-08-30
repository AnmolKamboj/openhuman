use super::*;

#[tokio::test]
async fn add_then_list_reflects_change() {
    let tmp = tempfile::tempdir().unwrap();
    let add = GoalsAddTool::new(tmp.path().to_path_buf());
    let res = add
        .execute(json!({ "text": "help ship the app" }))
        .await
        .unwrap();
    assert!(!res.is_error);

    let list = GoalsListTool::new(tmp.path().to_path_buf());
    let res = list.execute(json!({})).await.unwrap();
    assert!(res.text().contains("help ship the app"));
}

#[tokio::test]
async fn edit_and_delete_unknown_id_error() {
    let tmp = tempfile::tempdir().unwrap();
    let edit = GoalsEditTool::new(tmp.path().to_path_buf());
    let res = edit
        .execute(json!({ "id": "g9", "text": "x" }))
        .await
        .unwrap();
    assert!(res.is_error);

    let del = GoalsDeleteTool::new(tmp.path().to_path_buf());
    let res = del.execute(json!({ "id": "g9" })).await.unwrap();
    assert!(res.is_error);
}
