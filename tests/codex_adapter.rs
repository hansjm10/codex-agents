use codex_agents::{
    ArtifactKind, CodexAdapterErrorKind, CodexOutputFormat, CodexResponseItem, CodexSessionOutcome,
    CodexSessionResponse, LogStream, normalize_session_response,
};

#[test]
fn codex_session_normalization_builds_harness_facing_refs() {
    let normalized = normalize_session_response(
        "run-17",
        CodexSessionResponse {
            session_id: "session-123".to_string(),
            outcome: CodexSessionOutcome::Completed,
            items: vec![
                CodexResponseItem::Output {
                    label: "analysis".to_string(),
                    format: CodexOutputFormat::Markdown,
                    content: "# Plan\n- inspect the harness boundary\n".to_string(),
                    summary: Some("Analysis notes".to_string()),
                    is_final: false,
                },
                CodexResponseItem::Log {
                    stream: LogStream::Structured,
                    content: "{\"event\":\"tool_invoked\"}\n".to_string(),
                },
                CodexResponseItem::Output {
                    label: "final".to_string(),
                    format: CodexOutputFormat::Markdown,
                    content: "Integrated the worker through the harness boundary.\n".to_string(),
                    summary: Some("Final completion summary".to_string()),
                    is_final: true,
                },
                CodexResponseItem::Observation {
                    message: "Kept Codex response details inside src/codex.".to_string(),
                },
            ],
        },
    )
    .expect("normalization should succeed");

    assert_eq!(normalized.session_id, "session-123");
    assert_eq!(
        normalized.summary,
        "Integrated the worker through the harness boundary."
    );
    assert_eq!(
        normalized.final_message.as_deref(),
        Some("Integrated the worker through the harness boundary.\n")
    );
    assert_eq!(
        normalized.observations,
        vec!["Kept Codex response details inside src/codex.".to_string()]
    );

    assert_eq!(normalized.codex_output_refs.len(), 2);
    assert_eq!(
        normalized.codex_output_refs[0].artifact_id,
        "codex-output-01-analysis"
    );
    assert_eq!(
        normalized.codex_output_refs[1].artifact_id,
        "codex-output-02-final"
    );
    assert_eq!(normalized.log_refs.len(), 1);
    assert_eq!(
        normalized.log_refs[0].artifact_id,
        "codex-log-01-structured"
    );

    assert_eq!(normalized.artifacts.len(), 3);
    assert_eq!(
        normalized.artifacts[0].artifact.kind,
        ArtifactKind::CodexOutput
    );
    assert_eq!(
        normalized.artifacts[0].artifact.path,
        "runs/run-17/codex/output/01-analysis.md"
    );
    assert_eq!(normalized.artifacts[1].artifact.kind, ArtifactKind::Log);
    assert_eq!(
        normalized.artifacts[1].artifact.path,
        "runs/run-17/codex/logs/01-structured.log"
    );
}

#[test]
fn codex_session_normalization_rejects_missing_session_ids() {
    let error = normalize_session_response(
        "run-17",
        CodexSessionResponse {
            session_id: String::new(),
            outcome: CodexSessionOutcome::Failed,
            items: Vec::new(),
        },
    )
    .expect_err("missing session ids should be rejected");

    assert_eq!(error.kind, CodexAdapterErrorKind::MissingSessionId);
    assert!(error.message.contains("missing a session id"));
}
