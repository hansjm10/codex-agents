use std::error::Error;
use std::fmt;

use serde::{Deserialize, Serialize};

use crate::domain::{
    ArtifactKind, ArtifactRef, Assignment, CodexOutputFormat, CodexOutputRef, LogRef, LogStream,
};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CodexSessionRequest {
    pub run_id: String,
    pub assignment: Assignment,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CodexSessionOutcome {
    Completed,
    Blocked,
    Failed,
    Cancelled,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CodexSessionResponse {
    pub session_id: String,
    pub outcome: CodexSessionOutcome,
    pub items: Vec<CodexResponseItem>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum CodexResponseItem {
    Output {
        label: String,
        format: CodexOutputFormat,
        content: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        summary: Option<String>,
        #[serde(default)]
        is_final: bool,
    },
    Log {
        stream: LogStream,
        content: String,
    },
    Observation {
        message: String,
    },
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CapturedArtifact {
    pub artifact: ArtifactRef,
    pub contents: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct NormalizedCodexRun {
    pub session_id: String,
    pub outcome: CodexSessionOutcome,
    pub summary: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub final_message: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub observations: Vec<String>,
    pub artifacts: Vec<CapturedArtifact>,
    pub codex_output_refs: Vec<CodexOutputRef>,
    pub log_refs: Vec<LogRef>,
}

pub trait CodexSessionAdapter {
    fn execute(
        &mut self,
        request: CodexSessionRequest,
    ) -> Result<CodexSessionResponse, CodexAdapterError>;
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CodexAdapterError {
    pub kind: CodexAdapterErrorKind,
    pub message: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CodexAdapterErrorKind {
    InvalidRequest,
    MissingSessionId,
    ExecutionFailed,
}

impl fmt::Display for CodexAdapterError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "codex adapter error ({}): {}",
            self.kind.as_str(),
            self.message
        )
    }
}

impl Error for CodexAdapterError {}

impl CodexAdapterErrorKind {
    fn as_str(self) -> &'static str {
        match self {
            Self::InvalidRequest => "invalid_request",
            Self::MissingSessionId => "missing_session_id",
            Self::ExecutionFailed => "execution_failed",
        }
    }
}

pub fn execute_assignment<A>(
    adapter: &mut A,
    request: CodexSessionRequest,
) -> Result<NormalizedCodexRun, CodexAdapterError>
where
    A: CodexSessionAdapter,
{
    if request.run_id.trim().is_empty() {
        return Err(CodexAdapterError {
            kind: CodexAdapterErrorKind::InvalidRequest,
            message: "run_id must not be empty".to_string(),
        });
    }

    let run_id = request.run_id.clone();
    let response = adapter.execute(request)?;
    normalize_session_response(&run_id, response)
}

pub fn normalize_session_response(
    run_id: &str,
    response: CodexSessionResponse,
) -> Result<NormalizedCodexRun, CodexAdapterError> {
    if run_id.trim().is_empty() {
        return Err(CodexAdapterError {
            kind: CodexAdapterErrorKind::InvalidRequest,
            message: "run_id must not be empty".to_string(),
        });
    }
    if response.session_id.trim().is_empty() {
        return Err(CodexAdapterError {
            kind: CodexAdapterErrorKind::MissingSessionId,
            message: "codex session response is missing a session id".to_string(),
        });
    }

    let mut output_index = 0_u64;
    let mut log_index = 0_u64;
    let mut artifacts = Vec::new();
    let mut codex_output_refs = Vec::new();
    let mut log_refs = Vec::new();
    let mut observations = Vec::new();
    let mut final_message = None;
    let mut fallback_summary = None;

    for item in response.items {
        match item {
            CodexResponseItem::Output {
                label,
                format,
                content,
                summary,
                is_final,
            } => {
                output_index += 1;
                let slug = slugify(&label);
                let artifact_id = format!("codex-output-{output_index:02}-{slug}");
                let artifact = ArtifactRef {
                    artifact_id: artifact_id.clone(),
                    kind: ArtifactKind::CodexOutput,
                    path: format!(
                        "runs/{run_id}/codex/output/{output_index:02}-{slug}.{}",
                        file_extension(format)
                    ),
                    media_type: Some(media_type(format).to_string()),
                    description: Some(summary.clone().unwrap_or_else(|| {
                        format!("Codex {} output `{label}`", format_label(format))
                    })),
                    byte_length: Some(content.len() as u64),
                };
                let line_count = line_count(&content);

                if is_final && final_message.is_none() {
                    final_message = Some(content.clone());
                }
                if fallback_summary.is_none() {
                    fallback_summary = summary.clone().or_else(|| first_non_empty_line(&content));
                }

                codex_output_refs.push(CodexOutputRef {
                    artifact_id: artifact_id.clone(),
                    format,
                    summary,
                    line_count: Some(line_count),
                });
                artifacts.push(CapturedArtifact {
                    artifact,
                    contents: content,
                });
            }
            CodexResponseItem::Log { stream, content } => {
                log_index += 1;
                let stream_name = log_stream_name(stream);
                let artifact_id = format!("codex-log-{log_index:02}-{stream_name}");
                log_refs.push(LogRef {
                    artifact_id: artifact_id.clone(),
                    stream,
                    line_count: Some(line_count(&content)),
                });
                artifacts.push(CapturedArtifact {
                    artifact: ArtifactRef {
                        artifact_id,
                        kind: ArtifactKind::Log,
                        path: format!("runs/{run_id}/codex/logs/{log_index:02}-{stream_name}.log"),
                        media_type: Some("text/plain; charset=utf-8".to_string()),
                        description: Some(format!("Captured Codex {stream_name} log.")),
                        byte_length: Some(content.len() as u64),
                    },
                    contents: content,
                });
            }
            CodexResponseItem::Observation { message } => {
                if fallback_summary.is_none() {
                    fallback_summary = first_non_empty_line(&message);
                }
                observations.push(message);
            }
        }
    }

    let summary = final_message
        .as_ref()
        .and_then(|message| first_non_empty_line(message))
        .or(fallback_summary)
        .unwrap_or_else(|| outcome_summary(response.outcome).to_string());

    Ok(NormalizedCodexRun {
        session_id: response.session_id,
        outcome: response.outcome,
        summary,
        final_message,
        observations,
        artifacts,
        codex_output_refs,
        log_refs,
    })
}

fn file_extension(format: CodexOutputFormat) -> &'static str {
    match format {
        CodexOutputFormat::Markdown => "md",
        CodexOutputFormat::Json => "json",
        CodexOutputFormat::PlainText => "txt",
        CodexOutputFormat::Diff => "diff",
    }
}

fn media_type(format: CodexOutputFormat) -> &'static str {
    match format {
        CodexOutputFormat::Markdown => "text/markdown; charset=utf-8",
        CodexOutputFormat::Json => "application/json",
        CodexOutputFormat::PlainText => "text/plain; charset=utf-8",
        CodexOutputFormat::Diff => "text/x-diff; charset=utf-8",
    }
}

fn format_label(format: CodexOutputFormat) -> &'static str {
    match format {
        CodexOutputFormat::Markdown => "markdown",
        CodexOutputFormat::Json => "json",
        CodexOutputFormat::PlainText => "plain text",
        CodexOutputFormat::Diff => "diff",
    }
}

fn line_count(contents: &str) -> u64 {
    if contents.is_empty() {
        0
    } else {
        contents.lines().count() as u64
    }
}

fn log_stream_name(stream: LogStream) -> &'static str {
    match stream {
        LogStream::Stdout => "stdout",
        LogStream::Stderr => "stderr",
        LogStream::Structured => "structured",
    }
}

fn outcome_summary(outcome: CodexSessionOutcome) -> &'static str {
    match outcome {
        CodexSessionOutcome::Completed => "Codex completed without emitting a final message.",
        CodexSessionOutcome::Blocked => "Codex reported a blocked run.",
        CodexSessionOutcome::Failed => "Codex reported a failed run.",
        CodexSessionOutcome::Cancelled => "Codex reported a cancelled run.",
    }
}

fn first_non_empty_line(contents: &str) -> Option<String> {
    contents.lines().find_map(|line| {
        let trimmed = line.trim();
        (!trimmed.is_empty()).then(|| trimmed.to_string())
    })
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
        "output".to_string()
    } else {
        trimmed.to_string()
    }
}
