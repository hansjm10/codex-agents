use std::error::Error;
use std::fmt;
use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::codex::{
    CodexAdapterError, CodexSessionAdapter, CodexSessionOutcome, CodexSessionRequest,
    execute_assignment,
};
use crate::domain::{
    AgentEvent, AgentEventPayload, AgentResult, AgentResultStatus, ArtifactRef, Assignment,
    BlockerInfo, HarnessStatus, RunState, ValidationStatus,
};
use crate::harness::{
    BaselineHarnessRequest, BaselineHarnessRunner, GeneratedArtifact, HarnessRunError,
    ValidationCheck, ValidationExecutor,
};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgentRunRequest {
    pub run_id: String,
    pub assignment: Assignment,
    pub validation_checks: Vec<ValidationCheck>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgentRun {
    pub events: Vec<AgentEvent>,
    pub result: AgentResult,
    pub generated_artifacts: Vec<GeneratedArtifact>,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct AgentRuntime {
    harness: BaselineHarnessRunner,
}

impl AgentRuntime {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn execute<C, E>(
        &self,
        request: AgentRunRequest,
        codex_adapter: &mut C,
        validation_executor: &mut E,
    ) -> Result<AgentRun, AgentRuntimeError>
    where
        C: CodexSessionAdapter,
        E: ValidationExecutor,
    {
        validate_request(&request)?;

        let run_id = request.run_id.clone();
        let assignment_id = request.assignment.assignment_id.clone();
        let mut recorder = EventRecorder::new();

        recorder.push(
            RunState::Preparing,
            AgentEventPayload::RunStarted {
                run_id: run_id.clone(),
                assignment_id: assignment_id.clone(),
            },
        );
        recorder.transition(RunState::Preparing, RunState::Running);

        let codex_run = execute_assignment(
            codex_adapter,
            CodexSessionRequest {
                run_id: run_id.clone(),
                assignment: request.assignment.clone(),
            },
        )
        .map_err(AgentRuntimeError::from_codex)?;

        recorder.push(
            RunState::Running,
            AgentEventPayload::CodexSessionStarted {
                session_id: codex_run.session_id.clone(),
            },
        );

        let mut generated_artifacts = codex_run
            .artifacts
            .iter()
            .cloned()
            .map(|artifact| GeneratedArtifact {
                artifact: artifact.artifact,
                contents: artifact.contents,
            })
            .collect::<Vec<_>>();
        for artifact in &generated_artifacts {
            recorder.push(
                RunState::Running,
                AgentEventPayload::ArtifactRecorded {
                    artifact_id: artifact.artifact.artifact_id.clone(),
                },
            );
        }

        match codex_run.outcome {
            CodexSessionOutcome::Completed => {
                recorder.transition(RunState::Running, RunState::Validating);

                let harness_run = self
                    .harness
                    .run(
                        BaselineHarnessRequest {
                            run_id: run_id.clone(),
                            assignment_id: assignment_id.clone(),
                            checks: request.validation_checks,
                            artifact_refs: artifact_refs(&generated_artifacts),
                            codex_output_refs: codex_run.codex_output_refs,
                            log_refs: codex_run.log_refs,
                        },
                        validation_executor,
                    )
                    .map_err(AgentRuntimeError::from_harness)?;

                recorder.push(
                    RunState::Validating,
                    AgentEventPayload::ValidationCompleted {
                        status: validation_status(harness_run.harness_result().status),
                        failing_checks: harness_run.harness_result().failing_checks.clone(),
                    },
                );
                for artifact in &harness_run.generated_artifacts {
                    recorder.push(
                        RunState::Validating,
                        AgentEventPayload::ArtifactRecorded {
                            artifact_id: artifact.artifact.artifact_id.clone(),
                        },
                    );
                }

                let final_status = if harness_run.harness_result().status == HarnessStatus::Passed {
                    AgentResultStatus::Completed
                } else {
                    AgentResultStatus::Failed
                };
                let final_state = match final_status {
                    AgentResultStatus::Completed => RunState::Completed,
                    AgentResultStatus::Blocked => RunState::Blocked,
                    AgentResultStatus::Failed => RunState::Failed,
                    AgentResultStatus::Cancelled => RunState::Cancelled,
                };

                recorder.transition(RunState::Validating, final_state);
                recorder.push(
                    final_state,
                    AgentEventPayload::RunCompleted {
                        status: final_status,
                    },
                );

                let harness_result = harness_run.replay_record.harness_result;
                let mut observations = codex_run.observations;
                observations.push(harness_result.summary_for_next_agent.clone());

                generated_artifacts.extend(harness_run.generated_artifacts);

                Ok(AgentRun {
                    events: recorder.finish(),
                    result: AgentResult {
                        run_id,
                        assignment_id,
                        status: final_status,
                        final_state,
                        summary: completed_summary(&codex_run.summary, &harness_result),
                        observations,
                        final_message: codex_run.final_message,
                        blocker: None,
                        harness_result: Some(harness_result),
                    },
                    generated_artifacts,
                })
            }
            CodexSessionOutcome::Blocked => {
                let blocker = BlockerInfo {
                    code: "codex_blocked".to_string(),
                    message: codex_run.summary.clone(),
                    missing_capability: None,
                    resolution_hint: codex_run.observations.first().cloned(),
                };

                recorder.transition(RunState::Running, RunState::Blocked);
                recorder.push(
                    RunState::Blocked,
                    AgentEventPayload::Blocked {
                        blocker: blocker.clone(),
                    },
                );
                recorder.push(
                    RunState::Blocked,
                    AgentEventPayload::RunCompleted {
                        status: AgentResultStatus::Blocked,
                    },
                );

                Ok(AgentRun {
                    events: recorder.finish(),
                    result: AgentResult {
                        run_id,
                        assignment_id,
                        status: AgentResultStatus::Blocked,
                        final_state: RunState::Blocked,
                        summary: codex_run.summary,
                        observations: codex_run.observations,
                        final_message: codex_run.final_message,
                        blocker: Some(blocker),
                        harness_result: None,
                    },
                    generated_artifacts,
                })
            }
            CodexSessionOutcome::Failed => {
                recorder.transition(RunState::Running, RunState::Failed);
                recorder.push(
                    RunState::Failed,
                    AgentEventPayload::RunCompleted {
                        status: AgentResultStatus::Failed,
                    },
                );

                Ok(AgentRun {
                    events: recorder.finish(),
                    result: AgentResult {
                        run_id,
                        assignment_id,
                        status: AgentResultStatus::Failed,
                        final_state: RunState::Failed,
                        summary: codex_run.summary,
                        observations: codex_run.observations,
                        final_message: codex_run.final_message,
                        blocker: None,
                        harness_result: None,
                    },
                    generated_artifacts,
                })
            }
            CodexSessionOutcome::Cancelled => {
                recorder.transition(RunState::Running, RunState::Cancelled);
                recorder.push(
                    RunState::Cancelled,
                    AgentEventPayload::RunCompleted {
                        status: AgentResultStatus::Cancelled,
                    },
                );

                Ok(AgentRun {
                    events: recorder.finish(),
                    result: AgentResult {
                        run_id,
                        assignment_id,
                        status: AgentResultStatus::Cancelled,
                        final_state: RunState::Cancelled,
                        summary: codex_run.summary,
                        observations: codex_run.observations,
                        final_message: codex_run.final_message,
                        blocker: None,
                        harness_result: None,
                    },
                    generated_artifacts,
                })
            }
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgentRuntimeError {
    pub kind: AgentRuntimeErrorKind,
    pub message: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AgentRuntimeErrorKind {
    InvalidRequest,
    Codex,
    Harness,
}

impl AgentRuntimeError {
    fn from_codex(error: CodexAdapterError) -> Self {
        Self {
            kind: AgentRuntimeErrorKind::Codex,
            message: error.to_string(),
        }
    }

    fn from_harness(error: HarnessRunError) -> Self {
        Self {
            kind: AgentRuntimeErrorKind::Harness,
            message: error.to_string(),
        }
    }
}

impl fmt::Display for AgentRuntimeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "agent runtime error ({}): {}",
            self.kind.as_str(),
            self.message
        )
    }
}

impl Error for AgentRuntimeError {}

impl AgentRuntimeErrorKind {
    fn as_str(self) -> &'static str {
        match self {
            Self::InvalidRequest => "invalid_request",
            Self::Codex => "codex",
            Self::Harness => "harness",
        }
    }
}

fn validate_request(request: &AgentRunRequest) -> Result<(), AgentRuntimeError> {
    if request.run_id.trim().is_empty() {
        return Err(AgentRuntimeError {
            kind: AgentRuntimeErrorKind::InvalidRequest,
            message: "run_id must not be empty".to_string(),
        });
    }
    if request.validation_checks.is_empty() {
        return Err(AgentRuntimeError {
            kind: AgentRuntimeErrorKind::InvalidRequest,
            message: "at least one validation check is required".to_string(),
        });
    }

    validate_absolute_dir("repo_root", &request.assignment.repo_root)?;
    validate_absolute_dir("worktree_root", &request.assignment.worktree_root)?;

    Ok(())
}

fn validate_absolute_dir(field: &str, path: &str) -> Result<(), AgentRuntimeError> {
    let path = Path::new(path);
    if !path.is_absolute() {
        return Err(AgentRuntimeError {
            kind: AgentRuntimeErrorKind::InvalidRequest,
            message: format!("{field} must be an absolute path, got {}", path.display()),
        });
    }
    if !path.is_dir() {
        return Err(AgentRuntimeError {
            kind: AgentRuntimeErrorKind::InvalidRequest,
            message: format!(
                "{field} must point to an existing directory, got {}",
                path.display()
            ),
        });
    }

    Ok(())
}

fn artifact_refs(artifacts: &[GeneratedArtifact]) -> Vec<ArtifactRef> {
    artifacts
        .iter()
        .map(|artifact| artifact.artifact.clone())
        .collect()
}

fn validation_status(status: HarnessStatus) -> ValidationStatus {
    match status {
        HarnessStatus::Passed => ValidationStatus::Passed,
        HarnessStatus::Failed | HarnessStatus::Blocked => ValidationStatus::Failed,
    }
}

fn completed_summary(codex_summary: &str, harness_result: &crate::domain::HarnessResult) -> String {
    match harness_result.status {
        HarnessStatus::Passed => {
            format!("{codex_summary} Validation passed for the bounded assignment.")
        }
        HarnessStatus::Failed => format!(
            "{codex_summary} Validation failed for: {}.",
            harness_result.failing_checks.join(", ")
        ),
        HarnessStatus::Blocked => format!("{codex_summary} Harness validation is blocked."),
    }
}

#[derive(Clone, Debug, Default)]
struct EventRecorder {
    events: Vec<AgentEvent>,
    next_sequence: u64,
}

impl EventRecorder {
    fn new() -> Self {
        Self {
            events: Vec::new(),
            next_sequence: 1,
        }
    }

    fn push(&mut self, state: RunState, payload: AgentEventPayload) {
        let sequence = self.next_sequence;
        self.events.push(AgentEvent {
            sequence,
            timestamp: logical_timestamp(sequence),
            state,
            payload,
        });
        self.next_sequence += 1;
    }

    fn transition(&mut self, from: RunState, to: RunState) {
        self.push(to, AgentEventPayload::StateChanged { from, to });
    }

    fn finish(self) -> Vec<AgentEvent> {
        self.events
    }
}

fn logical_timestamp(sequence: u64) -> String {
    let seconds = sequence.saturating_sub(1);
    let hours = seconds / 3_600;
    let minutes = (seconds % 3_600) / 60;
    let seconds = seconds % 60;

    format!("1970-01-01T{hours:02}:{minutes:02}:{seconds:02}Z")
}
