use std::collections::VecDeque;

use codex_agents::harness::{
    BaselineHarnessRequest, BaselineHarnessRun, BaselineHarnessRunner, HarnessRunError,
    ValidationCheck, ValidationExecution, ValidationExecutor,
};
use codex_agents::{
    ArtifactEntrypointRole, ArtifactKind, ArtifactRef, CheckOutcome, CodexOutputFormat,
    CodexOutputRef, HarnessReplayRecord, HarnessStatus, LogRef, LogStream,
};

#[test]
fn baseline_runner_collates_passing_checks_into_one_replay_surface() {
    let request = sample_request();
    let mut executor = StubExecutor::new(vec![
        ValidationExecution {
            status: CheckOutcome::Passed,
            exit_code: Some(0),
            duration_ms: Some(12),
            stdout: "fmt ok\n".to_string(),
            stderr: String::new(),
        },
        ValidationExecution {
            status: CheckOutcome::Passed,
            exit_code: Some(0),
            duration_ms: Some(34),
            stdout: "test ok\n".to_string(),
            stderr: "warning: cached fixture\n".to_string(),
        },
    ]);

    let run = BaselineHarnessRunner
        .run(request, &mut executor)
        .expect("baseline harness run should succeed");

    assert_eq!(run.harness_result().status, HarnessStatus::Passed);
    assert!(run.harness_result().failing_checks.is_empty());
    assert_eq!(run.harness_result().check_results.len(), 2);
    assert_eq!(
        run.harness_result().check_results[0]
            .stdout_artifact_id
            .as_deref(),
        Some("check-01-cargo-fmt-check-stdout")
    );
    assert_eq!(
        run.harness_result().check_results[1]
            .stderr_artifact_id
            .as_deref(),
        Some("check-02-cargo-test-stderr")
    );
    assert_eq!(
        run.harness_result().codex_output_refs[0].artifact_id,
        "codex-md"
    );
    assert_eq!(
        run.harness_result().log_refs[0].artifact_id,
        "runtime-structured-log"
    );
    assert_eq!(
        run.harness_result().validation_result_refs[0].primary_artifact_id,
        "check-01-cargo-fmt-check-stdout"
    );
    assert_eq!(
        run.harness_result()
            .artifact_index
            .get("harness-report")
            .expect("report artifact should be indexed")
            .kind,
        ArtifactKind::ValidationReport
    );
    assert_eq!(
        run.harness_result().artifact_index.entrypoints[0].role,
        ArtifactEntrypointRole::StartHere
    );
    assert_eq!(
        run.harness_result().artifact_index.entrypoints[0].artifact_id,
        "check-01-cargo-fmt-check-stdout"
    );
    assert!(
        run.harness_result()
            .summary_for_next_agent
            .contains("harness-report")
    );

    let report = parse_report(&run);
    assert_eq!(report, run.replay_record);
}

#[test]
fn baseline_runner_preserves_failing_check_evidence_for_later_debugging() {
    let request = sample_request();
    let mut executor = StubExecutor::new(vec![
        ValidationExecution {
            status: CheckOutcome::Passed,
            exit_code: Some(0),
            duration_ms: Some(12),
            stdout: "fmt ok\n".to_string(),
            stderr: String::new(),
        },
        ValidationExecution {
            status: CheckOutcome::Failed,
            exit_code: Some(101),
            duration_ms: Some(34),
            stdout: "running 3 tests\n".to_string(),
            stderr: "test harness_runner::replay_surface ... FAILED\n".to_string(),
        },
    ]);

    let run = BaselineHarnessRunner
        .run(request, &mut executor)
        .expect("baseline harness run should succeed");

    assert_eq!(run.harness_result().status, HarnessStatus::Failed);
    assert_eq!(
        run.harness_result().failing_checks,
        vec!["cargo test".to_string()]
    );
    assert!(
        run.harness_result()
            .summary_for_next_agent
            .contains("check-02-cargo-test-stderr")
    );
    assert_eq!(
        run.harness_result().validation_result_refs[1].primary_artifact_id,
        "check-02-cargo-test-stderr"
    );
    assert_eq!(
        run.harness_result().artifact_index.entrypoints[0].artifact_id,
        "check-02-cargo-test-stderr"
    );
    assert_eq!(
        run.harness_result().artifact_index.entrypoints[0].role,
        ArtifactEntrypointRole::StartHere
    );
    assert!(
        run.harness_result()
            .artifact_index
            .entrypoints
            .iter()
            .any(|entrypoint| {
                entrypoint.role == ArtifactEntrypointRole::ReplayReport
                    && entrypoint.artifact_id == "harness-report"
            })
    );

    let stderr_artifact = run
        .generated_artifacts
        .iter()
        .find(|artifact| artifact.artifact.artifact_id == "check-02-cargo-test-stderr")
        .expect("failing stderr artifact should be generated");

    assert!(stderr_artifact.contents.contains("FAILED"));
}

#[test]
fn baseline_runner_is_deterministic_for_identical_inputs() {
    let request = sample_request();
    let executions = vec![
        ValidationExecution {
            status: CheckOutcome::Passed,
            exit_code: Some(0),
            duration_ms: Some(12),
            stdout: "fmt ok\n".to_string(),
            stderr: String::new(),
        },
        ValidationExecution {
            status: CheckOutcome::Failed,
            exit_code: Some(101),
            duration_ms: Some(34),
            stdout: "running 3 tests\n".to_string(),
            stderr: "test harness_runner::replay_surface ... FAILED\n".to_string(),
        },
    ];

    let first = BaselineHarnessRunner
        .run(request.clone(), &mut StubExecutor::new(executions.clone()))
        .expect("first baseline harness run should succeed");
    let second = BaselineHarnessRunner
        .run(request, &mut StubExecutor::new(executions))
        .expect("second baseline harness run should succeed");

    assert_eq!(first, second);
}

#[test]
fn baseline_runner_rejects_unindexed_codex_output_refs() {
    let mut request = sample_request();
    request.codex_output_refs = vec![CodexOutputRef {
        artifact_id: "missing".to_string(),
        format: CodexOutputFormat::Markdown,
        summary: Some("missing".to_string()),
        line_count: Some(1),
    }];

    let error = BaselineHarnessRunner
        .run(
            request,
            &mut StubExecutor::new(vec![ValidationExecution {
                status: CheckOutcome::Passed,
                exit_code: Some(0),
                duration_ms: Some(1),
                stdout: String::new(),
                stderr: String::new(),
            }]),
        )
        .expect_err("unknown artifact refs should be rejected");

    assert!(matches!(error, HarnessRunError::InvalidRequest(_)));
    assert!(error.to_string().contains("unknown artifact id"));
}

fn sample_request() -> BaselineHarnessRequest {
    BaselineHarnessRequest {
        run_id: "run-17".to_string(),
        assignment_id: "assignment-42".to_string(),
        checks: vec![
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
        artifact_refs: vec![
            ArtifactRef {
                artifact_id: "codex-md".to_string(),
                kind: ArtifactKind::CodexOutput,
                path: "runs/run-17/codex/final.md".to_string(),
                media_type: Some("text/markdown".to_string()),
                description: Some("Final Codex summary".to_string()),
                byte_length: Some(128),
            },
            ArtifactRef {
                artifact_id: "runtime-structured-log".to_string(),
                kind: ArtifactKind::Log,
                path: "runs/run-17/logs/runtime.jsonl".to_string(),
                media_type: Some("application/jsonl".to_string()),
                description: Some("Structured runtime log".to_string()),
                byte_length: Some(64),
            },
        ],
        codex_output_refs: vec![CodexOutputRef {
            artifact_id: "codex-md".to_string(),
            format: CodexOutputFormat::Markdown,
            summary: Some("Final Codex summary".to_string()),
            line_count: Some(6),
        }],
        log_refs: vec![LogRef {
            artifact_id: "runtime-structured-log".to_string(),
            stream: LogStream::Structured,
            line_count: Some(3),
        }],
    }
}

fn parse_report(run: &BaselineHarnessRun) -> HarnessReplayRecord {
    let report = run
        .generated_artifacts
        .iter()
        .find(|artifact| artifact.artifact.artifact_id == "harness-report")
        .expect("report artifact should be generated");

    serde_json::from_str(&report.contents).expect("report should deserialize")
}

#[derive(Clone, Debug)]
struct StubExecutor {
    executions: VecDeque<ValidationExecution>,
}

impl StubExecutor {
    fn new(executions: Vec<ValidationExecution>) -> Self {
        Self {
            executions: executions.into(),
        }
    }
}

impl ValidationExecutor for StubExecutor {
    fn execute(
        &mut self,
        _check: &ValidationCheck,
    ) -> Result<ValidationExecution, HarnessRunError> {
        self.executions
            .pop_front()
            .ok_or_else(|| HarnessRunError::InvalidRequest("missing stub execution".to_string()))
    }
}
