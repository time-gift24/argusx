use prompt_lab_core::CheckResult;

#[test]
fn check_result_allows_nullable_check_item_id() {
    let v = serde_json::json!({
      "id": 1,
      "context_type": "sop",
      "context_key": "sop:SOP-1",
      "check_item_id": null,
      "source_type": "manual",
      "operator_id": "u1",
      "result": {"ok": true},
      "is_pass": true,
      "created_at": 1730000000000i64
    });
    let parsed: CheckResult = serde_json::from_value(v).unwrap();
    assert!(parsed.check_item_id.is_none());
}
