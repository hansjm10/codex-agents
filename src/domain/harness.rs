use std::collections::BTreeMap;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum HarnessStatus {
    Passed,
    Failed,
    Blocked,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct HarnessResult {
    pub status: HarnessStatus,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub failing_checks: Vec<String>,
    pub check_results: Vec<CheckResult>,
    pub codex_output_refs: Vec<CodexOutputRef>,
    pub log_refs: Vec<LogRef>,
    pub artifact_index: ArtifactIndex,
    pub summary_for_next_agent: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct CheckResult {
    pub name: String,
    pub status: CheckOutcome,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub command: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub duration_ms: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub exit_code: Option<i32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stdout_artifact_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stderr_artifact_id: Option<String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum CheckOutcome {
    Passed,
    Failed,
    Skipped,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct CodexOutputRef {
    pub artifact_id: String,
    pub format: CodexOutputFormat,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub summary: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub line_count: Option<u64>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum CodexOutputFormat {
    Markdown,
    Json,
    PlainText,
    Diff,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct LogRef {
    pub artifact_id: String,
    pub stream: LogStream,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub line_count: Option<u64>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum LogStream {
    Stdout,
    Stderr,
    Structured,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct ArtifactIndex {
    pub artifact_refs: Vec<ArtifactRef>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub groups: Vec<ArtifactGroup>,
}

impl ArtifactIndex {
    pub fn new(artifact_refs: Vec<ArtifactRef>) -> Self {
        let mut grouped_ids: BTreeMap<ArtifactKind, Vec<String>> = BTreeMap::new();

        for artifact in &artifact_refs {
            grouped_ids
                .entry(artifact.kind)
                .or_default()
                .push(artifact.artifact_id.clone());
        }

        let groups = grouped_ids
            .into_iter()
            .map(|(kind, artifact_ids)| ArtifactGroup { kind, artifact_ids })
            .collect();

        Self {
            artifact_refs,
            groups,
        }
    }

    pub fn get(&self, artifact_id: &str) -> Option<&ArtifactRef> {
        self.artifact_refs
            .iter()
            .find(|artifact| artifact.artifact_id == artifact_id)
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct ArtifactGroup {
    pub kind: ArtifactKind,
    pub artifact_ids: Vec<String>,
}

#[derive(
    Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, JsonSchema,
)]
#[serde(rename_all = "snake_case")]
pub enum ArtifactKind {
    CodexOutput,
    Log,
    TestOutput,
    ValidationReport,
    Patch,
    Other,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct ArtifactRef {
    pub artifact_id: String,
    pub kind: ArtifactKind,
    pub path: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub media_type: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub byte_length: Option<u64>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct HarnessReplayRecord {
    pub run_id: String,
    pub assignment_id: String,
    pub harness_result: HarnessResult,
}
