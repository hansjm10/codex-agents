use std::collections::BTreeMap;

use codex_agents::{
    AgentEvent, AgentEventPayload, AgentResult, AgentResultStatus, ArtifactIndex, ArtifactKind,
    ArtifactRef, Assignment, AssignmentConstraints, CheckOutcome, CheckResult, CodexOutputFormat,
    CodexOutputRef, HarnessReplayRecord, HarnessResult, HarnessStatus, LogRef, LogStream,
    NetworkPolicy, ParameterSchema, RunState, SandboxPolicy, SchemaFormat, SkillPackRef,
    SkillPackScope, TimeoutPolicy, ToolSpec, ValidationStatus, WorkItemRef,
};
use schemars::schema_for;

#[test]
fn assignment_round_trips_with_runtime_input_contracts() {
    let mut metadata = BTreeMap::new();
    metadata.insert("priority".to_string(), "high".to_string());

    let assignment = Assignment {
        assignment_id: "assignment-42".to_string(),
        work_item: Some(WorkItemRef {
            system: "linear".to_string(),
            id: "IDL-1135".to_string(),
            url: Some("https://linear.app/idle-game-engine/issue/IDL-1135".to_string()),
        }),
        objective: "Define the public contracts for a single bounded agent run.".to_string(),
        repo_root: "/workspace/repo".to_string(),
        worktree_root: "/workspace/repo".to_string(),
        constraints: AssignmentConstraints {
            max_runtime_seconds: Some(1_800),
            require_clean_worktree: true,
            network_policy: NetworkPolicy::Denied,
            sandbox_policy: SandboxPolicy::WorkspaceWrite,
        },
        allowed_tools: vec![ToolSpec {
            name: "cargo-test".to_string(),
            command: "cargo test".to_string(),
            args_schema: Some(ParameterSchema {
                format: SchemaFormat::JsonSchema,
                definition:
                    "{\"type\":\"object\",\"properties\":{\"package\":{\"type\":\"string\"}}}"
                        .to_string(),
            }),
            supports_json: false,
            timeout_policy: TimeoutPolicy {
                soft_timeout_seconds: Some(600),
                hard_timeout_seconds: 900,
            },
            usage_examples: vec!["cargo test".to_string()],
        }],
        skill_packs: vec![SkillPackRef {
            name: "linear".to_string(),
            path: ".codex/skills/linear/SKILL.md".to_string(),
            revision_hint: Some("main".to_string()),
            scope: SkillPackScope::Assignment,
        }],
        metadata,
    };

    let value = serde_json::to_value(&assignment).expect("assignment should serialize");
    let round_trip: Assignment =
        serde_json::from_value(value.clone()).expect("assignment should deserialize");

    assert_eq!(round_trip, assignment);
    assert_eq!(value["constraints"]["network_policy"], "denied");
    assert_eq!(value["allowed_tools"][0]["name"], "cargo-test");
    assert_eq!(value["skill_packs"][0]["scope"], "assignment");
}

#[test]
fn agent_event_serializes_as_tagged_union() {
    let event = AgentEvent {
        sequence: 3,
        timestamp: "2026-03-18T00:15:00Z".to_string(),
        state: RunState::Validating,
        payload: AgentEventPayload::ValidationCompleted {
            status: ValidationStatus::Failed,
            failing_checks: vec!["cargo test".to_string()],
        },
    };

    let value = serde_json::to_value(&event).expect("agent event should serialize");

    assert_eq!(value["event_type"], "validation_completed");
    assert_eq!(value["state"], "validating");
    assert_eq!(value["status"], "failed");
    assert_eq!(value["failing_checks"][0], "cargo test");
}

#[test]
fn harness_result_schema_exposes_required_evidence_surfaces() {
    let schema = schema_for!(HarnessResult);
    let root = schema
        .schema
        .object
        .as_ref()
        .expect("harness result object");

    assert!(root.properties.contains_key("check_results"));
    assert!(root.properties.contains_key("codex_output_refs"));
    assert!(root.properties.contains_key("log_refs"));
    assert!(root.properties.contains_key("artifact_index"));
    assert!(root.properties.contains_key("summary_for_next_agent"));
}

#[test]
fn artifact_index_schema_exposes_artifact_refs() {
    let schema = schema_for!(ArtifactIndex);
    let root = schema
        .schema
        .object
        .as_ref()
        .expect("artifact index object");

    assert!(root.properties.contains_key("artifact_refs"));
    assert!(root.properties.contains_key("groups"));
}

#[test]
fn agent_result_round_trips_with_harness_result() {
    let harness_result = sample_harness_result();
    let agent_result = AgentResult {
        run_id: "run-17".to_string(),
        assignment_id: "assignment-42".to_string(),
        status: AgentResultStatus::Completed,
        final_state: RunState::Completed,
        summary: "Contracts compiled and validation passed.".to_string(),
        observations: vec!["Contract surfaces are JSON-serializable.".to_string()],
        final_message: Some("IDL-1135 is ready for review.".to_string()),
        blocker: None,
        harness_result: Some(harness_result.clone()),
    };

    let value = serde_json::to_value(&agent_result).expect("agent result should serialize");
    let round_trip: AgentResult =
        serde_json::from_value(value.clone()).expect("agent result should deserialize");

    assert_eq!(round_trip, agent_result);
    assert_eq!(
        value["harness_result"]["codex_output_refs"][0]["artifact_id"],
        "codex-md"
    );
    assert_eq!(
        value["harness_result"]["check_results"][0]["status"],
        "passed"
    );
    assert_eq!(
        value["harness_result"]["artifact_index"]["artifact_refs"][0]["kind"],
        "codex_output"
    );
}

#[test]
fn artifact_index_groups_artifacts_by_kind_for_machine_readable_debugging() {
    let artifact_index = sample_harness_result().artifact_index;

    assert_eq!(
        artifact_index
            .get("stderr-log")
            .expect("stderr log artifact should be indexed")
            .path,
        "artifacts/logs/stderr.log"
    );

    let log_group = artifact_index
        .groups
        .iter()
        .find(|group| group.kind == ArtifactKind::Log)
        .expect("log group should exist");

    assert_eq!(log_group.artifact_ids, vec!["stderr-log".to_string()]);
}

fn sample_harness_result() -> HarnessResult {
    let artifact_index = ArtifactIndex::new(vec![
        ArtifactRef {
            artifact_id: "codex-md".to_string(),
            kind: ArtifactKind::CodexOutput,
            path: "artifacts/codex/final.md".to_string(),
            media_type: Some("text/markdown".to_string()),
            description: Some("Final Codex report".to_string()),
            byte_length: Some(1_024),
        },
        ArtifactRef {
            artifact_id: "stderr-log".to_string(),
            kind: ArtifactKind::Log,
            path: "artifacts/logs/stderr.log".to_string(),
            media_type: Some("text/plain".to_string()),
            description: Some("Captured stderr".to_string()),
            byte_length: Some(512),
        },
        ArtifactRef {
            artifact_id: "cargo-test-json".to_string(),
            kind: ArtifactKind::TestOutput,
            path: "artifacts/tests/cargo-test.json".to_string(),
            media_type: Some("application/json".to_string()),
            description: Some("cargo test JSON output".to_string()),
            byte_length: Some(2_048),
        },
    ]);

    HarnessResult {
        status: HarnessStatus::Passed,
        failing_checks: Vec::new(),
        check_results: vec![CheckResult {
            name: "cargo test".to_string(),
            status: CheckOutcome::Passed,
            command: vec!["cargo".to_string(), "test".to_string()],
            duration_ms: Some(14_250),
            exit_code: Some(0),
            stdout_artifact_id: Some("cargo-test-json".to_string()),
            stderr_artifact_id: Some("stderr-log".to_string()),
        }],
        codex_output_refs: vec![CodexOutputRef {
            artifact_id: "codex-md".to_string(),
            format: CodexOutputFormat::Markdown,
            summary: Some("Final completion message".to_string()),
            line_count: Some(24),
        }],
        log_refs: vec![LogRef {
            artifact_id: "stderr-log".to_string(),
            stream: LogStream::Stderr,
            line_count: Some(12),
        }],
        artifact_index,
        summary_for_next_agent: "Inspect `cargo-test-json` first if validation regresses."
            .to_string(),
    }
}

#[test]
fn harness_replay_record_round_trips_for_machine_readable_replay() {
    let replay_record = HarnessReplayRecord {
        run_id: "run-17".to_string(),
        assignment_id: "assignment-42".to_string(),
        harness_result: sample_harness_result(),
    };

    let value = serde_json::to_value(&replay_record).expect("replay record should serialize");
    let round_trip: HarnessReplayRecord =
        serde_json::from_value(value.clone()).expect("replay record should deserialize");

    assert_eq!(round_trip, replay_record);
    assert_eq!(value["harness_result"]["status"], "passed");
}
