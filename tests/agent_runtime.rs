use std::collections::{BTreeMap, VecDeque};

use codex_agents::agent::AgentRuntime;
use codex_agents::codex::{
    CodexSessionAdapter, CodexSessionOutcome, CodexSessionRequest, CodexSessionResponse,
};
use codex_agents::harness::{
    HarnessRunError, ValidationCheck, ValidationExecution, ValidationExecutor,
};
use codex_agents::{
    AgentEventPayload, AgentResultStatus, ArtifactEntrypointRole, Assignment,
    AssignmentConstraints, CheckOutcome, CodexOutputFormat, NetworkPolicy, RunState, SandboxPolicy,
    SkillPackScope, TimeoutPolicy, ToolSpec, WorkItemRef,
};
use tempfile::tempdir;

#[test]
fn runtime_executes_a_bounded_assignment_through_codex_and_harness() {
    let repo_root = tempdir().expect("repo root tempdir should exist");
    let worktree_root = tempdir().expect("worktree tempdir should exist");
    let request = codex_agents::AgentRunRequest {
        run_id: "run-17".to_string(),
        assignment: sample_assignment(
            repo_root.path().display().to_string(),
            worktree_root.path().display().to_string(),
        ),
        validation_checks: vec![
            ValidationCheck {
                name: "cargo fmt --check".to_string(),
                command: vec![
                    "cargo".to_string(),
                    "fmt".to_string(),
                    "--check".to_string(),
                ],
            },
            ValidationCheck {
                name: "cargo test".to_string(),
                command: vec!["cargo".to_string(), "test".to_string()],
            },
        ],
    };
    let mut codex = StubCodexAdapter::new(CodexSessionResponse {
        session_id: "session-123".to_string(),
        outcome: CodexSessionOutcome::Completed,
        items: vec![
            codex_agents::CodexResponseItem::Output {
                label: "final".to_string(),
                format: CodexOutputFormat::Markdown,
                content: "Integrated Codex worker execution.\n".to_string(),
                summary: Some("Final worker summary".to_string()),
                is_final: true,
            },
            codex_agents::CodexResponseItem::Log {
                stream: codex_agents::LogStream::Structured,
                content: "{\"event\":\"session_started\"}\n".to_string(),
            },
            codex_agents::CodexResponseItem::Observation {
                message: "Normalized Codex outputs into harness artifacts.".to_string(),
            },
        ],
    });
    let mut validation = StubValidationExecutor::new(vec![
        ValidationExecution {
            status: CheckOutcome::Passed,
            exit_code: Some(0),
            duration_ms: Some(10),
            stdout: "fmt ok\n".to_string(),
            stderr: String::new(),
        },
        ValidationExecution {
            status: CheckOutcome::Passed,
            exit_code: Some(0),
            duration_ms: Some(20),
            stdout: "test ok\n".to_string(),
            stderr: String::new(),
        },
    ]);

    let run = AgentRuntime::new()
        .execute(request, &mut codex, &mut validation)
        .expect("runtime execution should succeed");

    assert_eq!(run.result.status, AgentResultStatus::Completed);
    assert_eq!(run.result.final_state, RunState::Completed);
    assert_eq!(
        run.result.final_message.as_deref(),
        Some("Integrated Codex worker execution.\n")
    );
    assert!(
        run.result
            .summary
            .contains("Validation passed for the bounded assignment")
    );
    assert!(
        run.result
            .observations
            .iter()
            .any(|note| note.contains("Normalized Codex outputs"))
    );

    let harness_result = run
        .result
        .harness_result
        .as_ref()
        .expect("completed runs should include harness results");
    assert_eq!(harness_result.codex_output_refs.len(), 1);
    assert_eq!(harness_result.log_refs.len(), 1);
    assert!(
        harness_result
            .artifact_index
            .get("codex-output-01-final")
            .is_some()
    );
    assert!(
        harness_result
            .artifact_index
            .entrypoints
            .iter()
            .any(|entrypoint| {
                entrypoint.role == ArtifactEntrypointRole::CodexOutput
                    && entrypoint.artifact_id == "codex-output-01-final"
            })
    );
    assert!(
        harness_result
            .artifact_index
            .get("harness-report")
            .is_some()
    );

    assert!(matches!(
        &run.events[0].payload,
        AgentEventPayload::RunStarted { run_id, assignment_id }
            if run_id == "run-17" && assignment_id == "assignment-42"
    ));
    assert!(run.events.iter().any(|event| matches!(
        event.payload,
        AgentEventPayload::CodexSessionStarted { ref session_id } if session_id == "session-123"
    )));
    assert!(
        run.events
            .iter()
            .any(|event| matches!(event.payload, AgentEventPayload::ValidationCompleted { .. }))
    );
    assert!(matches!(
        run.events.last().expect("final event").payload,
        AgentEventPayload::RunCompleted {
            status: AgentResultStatus::Completed
        }
    ));

    assert!(run.generated_artifacts.iter().any(|artifact| {
        artifact.artifact.artifact_id == "codex-output-01-final"
            && artifact
                .contents
                .contains("Integrated Codex worker execution")
    }));
    assert!(
        run.generated_artifacts
            .iter()
            .any(|artifact| artifact.artifact.artifact_id == "harness-report")
    );
}

#[test]
fn runtime_surfaces_blocked_codex_runs_without_invoking_harness() {
    let repo_root = tempdir().expect("repo root tempdir should exist");
    let worktree_root = tempdir().expect("worktree tempdir should exist");
    let request = codex_agents::AgentRunRequest {
        run_id: "run-18".to_string(),
        assignment: sample_assignment(
            repo_root.path().display().to_string(),
            worktree_root.path().display().to_string(),
        ),
        validation_checks: vec![ValidationCheck {
            name: "cargo test".to_string(),
            command: vec!["cargo".to_string(), "test".to_string()],
        }],
    };
    let mut codex = StubCodexAdapter::new(CodexSessionResponse {
        session_id: "session-456".to_string(),
        outcome: CodexSessionOutcome::Blocked,
        items: vec![codex_agents::CodexResponseItem::Observation {
            message: "Missing external approval.".to_string(),
        }],
    });
    let mut validation = StubValidationExecutor::new(Vec::new());

    let run = AgentRuntime::new()
        .execute(request, &mut codex, &mut validation)
        .expect("blocked execution should still return a run result");

    assert_eq!(run.result.status, AgentResultStatus::Blocked);
    assert_eq!(run.result.final_state, RunState::Blocked);
    assert!(run.result.harness_result.is_none());
    assert_eq!(
        run.result
            .blocker
            .as_ref()
            .expect("blocked runs should include blocker info")
            .resolution_hint
            .as_deref(),
        Some("Missing external approval.")
    );
    assert!(
        run.events
            .iter()
            .any(|event| matches!(event.payload, AgentEventPayload::Blocked { .. }))
    );
}

fn sample_assignment(repo_root: String, worktree_root: String) -> Assignment {
    Assignment {
        assignment_id: "assignment-42".to_string(),
        work_item: Some(WorkItemRef {
            system: "linear".to_string(),
            id: "IDL-1139".to_string(),
            url: Some(
                "https://linear.app/idle-game-engine/issue/IDL-1139/integrate-codex-worker-execution-behind-the-harness-boundary"
                    .to_string(),
            ),
        }),
        objective: "Integrate the Codex-backed worker behind the harness boundary.".to_string(),
        repo_root,
        worktree_root,
        constraints: AssignmentConstraints {
            max_runtime_seconds: Some(1_200),
            require_clean_worktree: false,
            network_policy: NetworkPolicy::Allowed,
            sandbox_policy: SandboxPolicy::DangerFullAccess,
        },
        allowed_tools: vec![ToolSpec {
            name: "cargo".to_string(),
            command: "cargo".to_string(),
            args_schema: None,
            supports_json: false,
            timeout_policy: TimeoutPolicy {
                soft_timeout_seconds: Some(300),
                hard_timeout_seconds: 600,
            },
            usage_examples: vec!["cargo test".to_string()],
        }],
        skill_packs: vec![codex_agents::SkillPackRef {
            name: "linear".to_string(),
            path: ".codex/skills/linear/SKILL.md".to_string(),
            revision_hint: Some("main".to_string()),
            scope: SkillPackScope::Assignment,
        }],
        metadata: BTreeMap::new(),
    }
}

#[derive(Clone, Debug)]
struct StubCodexAdapter {
    response: Option<CodexSessionResponse>,
}

impl StubCodexAdapter {
    fn new(response: CodexSessionResponse) -> Self {
        Self {
            response: Some(response),
        }
    }
}

impl CodexSessionAdapter for StubCodexAdapter {
    fn execute(
        &mut self,
        _request: CodexSessionRequest,
    ) -> Result<CodexSessionResponse, codex_agents::CodexAdapterError> {
        self.response.take().ok_or(codex_agents::CodexAdapterError {
            kind: codex_agents::CodexAdapterErrorKind::ExecutionFailed,
            message: "stub response was already consumed".to_string(),
        })
    }
}

#[derive(Clone, Debug)]
struct StubValidationExecutor {
    executions: VecDeque<ValidationExecution>,
}

impl StubValidationExecutor {
    fn new(executions: Vec<ValidationExecution>) -> Self {
        Self {
            executions: executions.into(),
        }
    }
}

impl ValidationExecutor for StubValidationExecutor {
    fn execute(
        &mut self,
        _check: &ValidationCheck,
    ) -> Result<ValidationExecution, HarnessRunError> {
        self.executions
            .pop_front()
            .ok_or_else(|| HarnessRunError::InvalidRequest("missing stub execution".to_string()))
    }
}
