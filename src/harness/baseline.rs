use std::collections::BTreeSet;
use std::error::Error;
use std::fmt;

use serde::{Deserialize, Serialize};

use crate::domain::{
    ArtifactIndex, ArtifactKind, ArtifactRef, CheckOutcome, CheckResult, CodexOutputRef,
    HarnessReplayRecord, HarnessResult, HarnessStatus, LogRef,
};

const REPORT_ARTIFACT_ID: &str = "harness-report";
const REPORT_MEDIA_TYPE: &str = "application/json";
const LOG_MEDIA_TYPE: &str = "text/plain; charset=utf-8";

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct BaselineHarnessRequest {
    pub run_id: String,
    pub assignment_id: String,
    pub checks: Vec<ValidationCheck>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub artifact_refs: Vec<ArtifactRef>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub codex_output_refs: Vec<CodexOutputRef>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub log_refs: Vec<LogRef>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ValidationCheck {
    pub name: String,
    pub command: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ValidationExecution {
    pub status: CheckOutcome,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub exit_code: Option<i32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub duration_ms: Option<u64>,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub stdout: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub stderr: String,
}

pub trait ValidationExecutor {
    fn execute(&mut self, check: &ValidationCheck) -> Result<ValidationExecution, HarnessRunError>;
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct GeneratedArtifact {
    pub artifact: ArtifactRef,
    pub contents: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct BaselineHarnessRun {
    pub replay_record: HarnessReplayRecord,
    pub generated_artifacts: Vec<GeneratedArtifact>,
}

impl BaselineHarnessRun {
    pub fn harness_result(&self) -> &HarnessResult {
        &self.replay_record.harness_result
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum HarnessRunError {
    InvalidRequest(String),
    ExecutionFailed { check_name: String, message: String },
    Serialization(String),
}

impl fmt::Display for HarnessRunError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidRequest(message) => write!(f, "invalid harness request: {message}"),
            Self::ExecutionFailed {
                check_name,
                message,
            } => write!(
                f,
                "validation check `{check_name}` failed to execute: {message}"
            ),
            Self::Serialization(message) => {
                write!(f, "failed to serialize harness replay record: {message}")
            }
        }
    }
}

impl Error for HarnessRunError {}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct BaselineHarnessRunner;

impl BaselineHarnessRunner {
    pub fn run<E>(
        &self,
        request: BaselineHarnessRequest,
        executor: &mut E,
    ) -> Result<BaselineHarnessRun, HarnessRunError>
    where
        E: ValidationExecutor,
    {
        request.validate()?;

        let mut artifact_ids = request
            .artifact_refs
            .iter()
            .map(|artifact| artifact.artifact_id.clone())
            .collect::<BTreeSet<_>>();

        for codex_output in &request.codex_output_refs {
            ensure_known_artifact(
                &artifact_ids,
                "codex_output_refs",
                &codex_output.artifact_id,
            )?;
        }

        for log_ref in &request.log_refs {
            ensure_known_artifact(&artifact_ids, "log_refs", &log_ref.artifact_id)?;
        }

        let mut check_results = Vec::with_capacity(request.checks.len());
        let mut generated_artifacts = Vec::with_capacity(request.checks.len() * 2 + 1);
        let mut generated_refs = Vec::with_capacity(request.checks.len() * 2 + 1);
        let mut failing_checks = Vec::new();

        for (index, check) in request.checks.iter().enumerate() {
            let execution = executor.execute(check).map_err(|error| match error {
                HarnessRunError::ExecutionFailed { .. } => error,
                other => HarnessRunError::ExecutionFailed {
                    check_name: check.name.clone(),
                    message: other.to_string(),
                },
            })?;

            if execution.status == CheckOutcome::Failed {
                failing_checks.push(check.name.clone());
            }

            let artifact_stem = format!("{:02}-{}", index + 1, slugify(&check.name));
            let stdout = generated_log_artifact(
                &request.run_id,
                &artifact_stem,
                "stdout",
                &format!("Stdout for validation check `{}`", check.name),
                execution.stdout.clone(),
            );
            let stderr = generated_log_artifact(
                &request.run_id,
                &artifact_stem,
                "stderr",
                &format!("Stderr for validation check `{}`", check.name),
                execution.stderr.clone(),
            );

            for artifact in [stdout, stderr] {
                if !artifact_ids.insert(artifact.artifact.artifact_id.clone()) {
                    return Err(HarnessRunError::InvalidRequest(format!(
                        "generated artifact id `{}` collides with an existing artifact",
                        artifact.artifact.artifact_id
                    )));
                }

                generated_refs.push(artifact.artifact.clone());
                generated_artifacts.push(artifact);
            }

            check_results.push(CheckResult {
                name: check.name.clone(),
                status: execution.status,
                command: check.command.clone(),
                duration_ms: execution.duration_ms,
                exit_code: execution.exit_code,
                stdout_artifact_id: Some(format!("check-{artifact_stem}-stdout")),
                stderr_artifact_id: Some(format!("check-{artifact_stem}-stderr")),
            });
        }

        let report_artifact = ArtifactRef {
            artifact_id: REPORT_ARTIFACT_ID.to_string(),
            kind: ArtifactKind::ValidationReport,
            path: format!("runs/{}/harness/report.json", request.run_id),
            media_type: Some(REPORT_MEDIA_TYPE.to_string()),
            description: Some("Replay-ready harness validation report".to_string()),
            byte_length: None,
        };

        if !artifact_ids.insert(report_artifact.artifact_id.clone()) {
            return Err(HarnessRunError::InvalidRequest(format!(
                "artifact id `{REPORT_ARTIFACT_ID}` is reserved for the harness report"
            )));
        }

        generated_refs.push(report_artifact.clone());

        let status = if failing_checks.is_empty() {
            HarnessStatus::Passed
        } else {
            HarnessStatus::Failed
        };

        let artifact_index = ArtifactIndex::new(
            request
                .artifact_refs
                .iter()
                .cloned()
                .chain(generated_refs)
                .collect(),
        );
        let summary_for_next_agent = build_summary(
            status,
            &request.checks,
            &failing_checks,
            &check_results,
            REPORT_ARTIFACT_ID,
        );
        let harness_result = HarnessResult {
            status,
            failing_checks,
            check_results,
            codex_output_refs: request.codex_output_refs,
            log_refs: request.log_refs,
            artifact_index,
            summary_for_next_agent,
        };
        let replay_record = HarnessReplayRecord {
            run_id: request.run_id.clone(),
            assignment_id: request.assignment_id,
            harness_result,
        };
        let report_contents = serde_json::to_string_pretty(&replay_record)
            .map_err(|error| HarnessRunError::Serialization(error.to_string()))?;

        generated_artifacts.push(GeneratedArtifact {
            artifact: ArtifactRef {
                byte_length: Some(report_contents.len() as u64),
                ..report_artifact
            },
            contents: report_contents,
        });

        Ok(BaselineHarnessRun {
            replay_record,
            generated_artifacts,
        })
    }
}

impl BaselineHarnessRequest {
    fn validate(&self) -> Result<(), HarnessRunError> {
        if self.run_id.trim().is_empty() {
            return Err(HarnessRunError::InvalidRequest(
                "run_id must not be empty".to_string(),
            ));
        }

        if self.assignment_id.trim().is_empty() {
            return Err(HarnessRunError::InvalidRequest(
                "assignment_id must not be empty".to_string(),
            ));
        }

        if self.checks.is_empty() {
            return Err(HarnessRunError::InvalidRequest(
                "at least one validation check is required".to_string(),
            ));
        }

        let mut artifact_ids = BTreeSet::new();
        for artifact in &self.artifact_refs {
            if artifact.artifact_id.trim().is_empty() {
                return Err(HarnessRunError::InvalidRequest(
                    "artifact_refs must use non-empty artifact ids".to_string(),
                ));
            }

            if !artifact_ids.insert(artifact.artifact_id.clone()) {
                return Err(HarnessRunError::InvalidRequest(format!(
                    "duplicate artifact id `{}` in artifact_refs",
                    artifact.artifact_id
                )));
            }
        }

        let mut check_names = BTreeSet::new();
        for check in &self.checks {
            if check.name.trim().is_empty() {
                return Err(HarnessRunError::InvalidRequest(
                    "validation checks must have names".to_string(),
                ));
            }

            if check.command.is_empty() || check.command.iter().any(|part| part.trim().is_empty()) {
                return Err(HarnessRunError::InvalidRequest(format!(
                    "validation check `{}` must define a non-empty command",
                    check.name
                )));
            }

            if !check_names.insert(check.name.clone()) {
                return Err(HarnessRunError::InvalidRequest(format!(
                    "duplicate validation check name `{}`",
                    check.name
                )));
            }
        }

        Ok(())
    }
}

fn ensure_known_artifact(
    artifact_ids: &BTreeSet<String>,
    collection: &str,
    artifact_id: &str,
) -> Result<(), HarnessRunError> {
    if artifact_ids.contains(artifact_id) {
        Ok(())
    } else {
        Err(HarnessRunError::InvalidRequest(format!(
            "`{collection}` references unknown artifact id `{artifact_id}`"
        )))
    }
}

fn generated_log_artifact(
    run_id: &str,
    artifact_stem: &str,
    stream: &str,
    description: &str,
    contents: String,
) -> GeneratedArtifact {
    let artifact_id = format!("check-{artifact_stem}-{stream}");

    GeneratedArtifact {
        artifact: ArtifactRef {
            artifact_id,
            kind: ArtifactKind::Log,
            path: format!("runs/{run_id}/harness/checks/{artifact_stem}/{stream}.log"),
            media_type: Some(LOG_MEDIA_TYPE.to_string()),
            description: Some(description.to_string()),
            byte_length: Some(contents.len() as u64),
        },
        contents,
    }
}

fn build_summary(
    status: HarnessStatus,
    checks: &[ValidationCheck],
    failing_checks: &[String],
    check_results: &[CheckResult],
    report_artifact_id: &str,
) -> String {
    match status {
        HarnessStatus::Passed => format!(
            "Validation passed for {} checks. Start with `{report_artifact_id}` to replay the harness surface if this regresses.",
            checks.len()
        ),
        HarnessStatus::Failed => {
            let first_failure = failing_checks
                .first()
                .and_then(|name| {
                    check_results
                        .iter()
                        .find(|result| &result.name == name)
                        .and_then(|result| result.stderr_artifact_id.as_deref())
                })
                .unwrap_or(report_artifact_id);

            format!(
                "Validation failed for {} of {} checks: {}. Inspect `{first_failure}` first, then `{report_artifact_id}` for the replay-ready report.",
                failing_checks.len(),
                checks.len(),
                failing_checks.join(", ")
            )
        }
        HarnessStatus::Blocked => {
            format!("Harness execution is blocked. Inspect `{report_artifact_id}` for details.")
        }
    }
}

fn slugify(input: &str) -> String {
    let mut slug = String::new();
    let mut last_was_separator = false;

    for ch in input.chars() {
        if ch.is_ascii_alphanumeric() {
            slug.push(ch.to_ascii_lowercase());
            last_was_separator = false;
        } else if !last_was_separator {
            slug.push('-');
            last_was_separator = true;
        }
    }

    let trimmed = slug.trim_matches('-');
    if trimmed.is_empty() {
        "check".to_string()
    } else {
        trimmed.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::slugify;

    #[test]
    fn slugify_normalizes_for_stable_paths() {
        assert_eq!(slugify("cargo fmt --check"), "cargo-fmt-check");
        assert_eq!(slugify("  "), "check");
    }
}
