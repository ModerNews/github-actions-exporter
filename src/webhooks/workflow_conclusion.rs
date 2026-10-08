use serde::Deserialize;

/// Snake_case on the wire — `timed_out`, `action_required`.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WorkflowConclusion {
    Success,
    Failure,
    Neutral,
    Cancelled,
    TimedOut,
    ActionRequired,
    Stale,
    Skipped,
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every variant, against the wire strings the schema enumerates.
    #[test]
    fn all_variants_match_their_wire_names() {
        let cases = [
            ("success", WorkflowConclusion::Success),
            ("failure", WorkflowConclusion::Failure),
            ("neutral", WorkflowConclusion::Neutral),
            ("cancelled", WorkflowConclusion::Cancelled),
            ("timed_out", WorkflowConclusion::TimedOut),
            ("action_required", WorkflowConclusion::ActionRequired),
            ("stale", WorkflowConclusion::Stale),
            ("skipped", WorkflowConclusion::Skipped),
        ];
        for (wire, expected) in &cases {
            let got: WorkflowConclusion =
                serde_json::from_value(serde_json::json!(wire)).expect(wire);
            assert_eq!(&got, expected, "{wire}");
        }
    }

    /// British spelling on the wire — `canceled` is not what GitHub sends.
    #[test]
    fn spelling_is_pinned() {
        assert!(
            serde_json::from_value::<WorkflowConclusion>(serde_json::json!("canceled")).is_err()
        );
    }
}
