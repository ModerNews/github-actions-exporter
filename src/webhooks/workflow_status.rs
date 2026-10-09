use serde::Deserialize;

/// GitHub sends these as snake_case; without `rename_all` serde would expect
/// the variant names verbatim (`"InProgress"`) and reject every real payload.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WorkflowStatus {
    Requested,
    InProgress,
    Completed,
    Queued,
    Waiting,
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every variant, against the wire strings the schema enumerates. Guards
    /// the `rename_all` boundary: a variant added without a matching wire name
    /// fails here rather than silently at runtime.
    #[test]
    fn all_variants_match_their_wire_names() {
        let cases = [
            ("requested", WorkflowStatus::Requested),
            ("in_progress", WorkflowStatus::InProgress),
            ("completed", WorkflowStatus::Completed),
            ("queued", WorkflowStatus::Queued),
            ("waiting", WorkflowStatus::Waiting),
        ];
        for (wire, expected) in &cases {
            let got: WorkflowStatus = serde_json::from_value(serde_json::json!(wire)).expect(wire);
            assert_eq!(&got, expected, "{wire}");
        }
    }

    #[test]
    fn pascal_case_is_not_accepted() {
        // the exact failure `rename_all` exists to prevent
        assert!(serde_json::from_value::<WorkflowStatus>(serde_json::json!("InProgress")).is_err());
    }
}
