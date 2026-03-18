use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    io::{self, Read},
    path::{Path, PathBuf},
    process::{Command, Stdio},
    thread,
    time::{Duration, Instant},
};

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::domain::{ParameterSchema, TimeoutPolicy, ToolExecutionOutcome, ToolSpec};

pub const TOOL_MANIFEST_RELATIVE_PATH: &str = "manifests/tools.json";

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct ToolManifestCatalog {
    pub version: u32,
    pub tools: Vec<ToolManifest>,
}

impl ToolManifestCatalog {
    pub fn load_from_repo(repo_root: &Path) -> Result<Self, ToolManifestLoadError> {
        Self::load_from_path(&repo_root.join(TOOL_MANIFEST_RELATIVE_PATH))
    }

    pub fn load_from_path(path: &Path) -> Result<Self, ToolManifestLoadError> {
        let contents = fs::read_to_string(path).map_err(|error| ToolManifestLoadError {
            kind: ToolManifestLoadErrorKind::Io,
            message: format!(
                "failed to read tool manifest at {}: {error}",
                path.display()
            ),
        })?;
        let catalog: Self =
            serde_json::from_str(&contents).map_err(|error| ToolManifestLoadError {
                kind: ToolManifestLoadErrorKind::InvalidJson,
                message: format!(
                    "failed to parse tool manifest at {}: {error}",
                    path.display()
                ),
            })?;
        catalog.validate()
    }

    pub fn get(&self, name: &str) -> Option<&ToolManifest> {
        self.tools.iter().find(|tool| tool.name == name)
    }

    pub fn tool_names(&self) -> Vec<String> {
        self.tools.iter().map(|tool| tool.name.clone()).collect()
    }

    fn validate(self) -> Result<Self, ToolManifestLoadError> {
        let mut seen = BTreeSet::new();

        for tool in &self.tools {
            if tool.name.trim().is_empty() {
                return Err(ToolManifestLoadError {
                    kind: ToolManifestLoadErrorKind::InvalidManifest,
                    message: "tool manifest entries must have a non-empty name".to_string(),
                });
            }
            if !seen.insert(tool.name.clone()) {
                return Err(ToolManifestLoadError {
                    kind: ToolManifestLoadErrorKind::DuplicateToolName,
                    message: format!("tool manifest contains duplicate tool `{}`", tool.name),
                });
            }
            if tool.timeout_policy.hard_timeout_seconds == 0 {
                return Err(ToolManifestLoadError {
                    kind: ToolManifestLoadErrorKind::InvalidManifest,
                    message: format!("tool `{}` must declare a non-zero hard timeout", tool.name),
                });
            }
        }

        Ok(self)
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct ToolManifest {
    pub name: String,
    pub description: String,
    pub executable: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub default_args: Vec<String>,
    pub state: ToolManifestState,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub args_schema: Option<ParameterSchema>,
    pub supports_json: bool,
    pub timeout_policy: TimeoutPolicy,
    pub inherit_parent_env: bool,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub allowed_env: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub usage_examples: Vec<String>,
}

impl ToolManifest {
    pub fn as_tool_spec(&self) -> ToolSpec {
        ToolSpec {
            name: self.name.clone(),
            command: self.executable.clone(),
            args_schema: self.args_schema.clone(),
            supports_json: self.supports_json,
            timeout_policy: self.timeout_policy.clone(),
            usage_examples: self.usage_examples.clone(),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ToolManifestState {
    Stable,
    Deprecated,
    Experimental,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ToolInvocation {
    pub tool_name: String,
    pub cwd: PathBuf,
    pub args: Vec<String>,
    pub env: BTreeMap<String, String>,
    pub timeout_override_seconds: Option<u64>,
    pub capture_json: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct ToolExecutionResult {
    pub tool_name: String,
    pub command: Vec<String>,
    pub cwd: String,
    pub outcome: ToolExecutionOutcome,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub exit_code: Option<i32>,
    pub duration_ms: u64,
    pub stdout: String,
    pub stderr: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub json_output: Option<Value>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct ToolExecutionError {
    pub kind: ToolExecutionErrorKind,
    pub message: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ToolExecutionErrorKind {
    UnknownTool,
    WorkingDirectoryNotAbsolute,
    WorkingDirectoryNotFound,
    EnvironmentVariableNotAllowed,
    TimeoutOutOfPolicy,
    JsonOutputNotSupported,
    InvalidJsonOutput,
    SpawnFailed,
    WaitFailed,
    OutputReadFailed,
}

#[derive(Clone, Debug)]
pub struct ToolRunner {
    catalog: ToolManifestCatalog,
}

impl ToolRunner {
    pub fn new(catalog: ToolManifestCatalog) -> Self {
        Self { catalog }
    }

    pub fn run(
        &self,
        invocation: ToolInvocation,
    ) -> Result<ToolExecutionResult, ToolExecutionError> {
        let manifest =
            self.catalog
                .get(&invocation.tool_name)
                .ok_or_else(|| ToolExecutionError {
                    kind: ToolExecutionErrorKind::UnknownTool,
                    message: format!(
                        "tool `{}` is not present in the manifest",
                        invocation.tool_name
                    ),
                })?;

        if !invocation.cwd.is_absolute() {
            return Err(ToolExecutionError {
                kind: ToolExecutionErrorKind::WorkingDirectoryNotAbsolute,
                message: format!(
                    "tool `{}` requires an absolute working directory, got {}",
                    invocation.tool_name,
                    invocation.cwd.display()
                ),
            });
        }
        if !invocation.cwd.is_dir() {
            return Err(ToolExecutionError {
                kind: ToolExecutionErrorKind::WorkingDirectoryNotFound,
                message: format!(
                    "tool `{}` working directory does not exist: {}",
                    invocation.tool_name,
                    invocation.cwd.display()
                ),
            });
        }
        for key in invocation.env.keys() {
            if !manifest.allowed_env.iter().any(|allowed| allowed == key) {
                return Err(ToolExecutionError {
                    kind: ToolExecutionErrorKind::EnvironmentVariableNotAllowed,
                    message: format!(
                        "tool `{}` does not allow overriding environment variable `{key}`",
                        invocation.tool_name
                    ),
                });
            }
        }

        let timeout_seconds = invocation
            .timeout_override_seconds
            .unwrap_or(manifest.timeout_policy.hard_timeout_seconds);
        if timeout_seconds == 0 || timeout_seconds > manifest.timeout_policy.hard_timeout_seconds {
            return Err(ToolExecutionError {
                kind: ToolExecutionErrorKind::TimeoutOutOfPolicy,
                message: format!(
                    "tool `{}` timeout {}s exceeds hard policy {}s",
                    invocation.tool_name,
                    timeout_seconds,
                    manifest.timeout_policy.hard_timeout_seconds
                ),
            });
        }
        if invocation.capture_json && !manifest.supports_json {
            return Err(ToolExecutionError {
                kind: ToolExecutionErrorKind::JsonOutputNotSupported,
                message: format!(
                    "tool `{}` does not declare JSON output support",
                    invocation.tool_name
                ),
            });
        }

        let command_line = build_command_line(manifest, &invocation.args);
        let mut command = Command::new(&manifest.executable);
        command.args(&manifest.default_args);
        command.args(&invocation.args);
        command.current_dir(&invocation.cwd);
        if !manifest.inherit_parent_env {
            command.env_clear();
        }
        command.envs(&invocation.env);
        command.stdout(Stdio::piped());
        command.stderr(Stdio::piped());

        let start = Instant::now();
        let mut child = command.spawn().map_err(|error| ToolExecutionError {
            kind: ToolExecutionErrorKind::SpawnFailed,
            message: format!("failed to spawn tool `{}`: {error}", invocation.tool_name),
        })?;
        let stdout_reader = spawn_reader(child.stdout.take());
        let stderr_reader = spawn_reader(child.stderr.take());

        let timeout = Duration::from_secs(timeout_seconds);
        let mut timed_out = false;
        let status = loop {
            match child.try_wait() {
                Ok(Some(status)) => break Some(status),
                Ok(None) => {
                    if start.elapsed() >= timeout {
                        timed_out = true;
                        kill_child(&mut child, &invocation.tool_name)?;
                        child.wait().map_err(|error| ToolExecutionError {
                            kind: ToolExecutionErrorKind::WaitFailed,
                            message: format!(
                                "failed to wait for timed out tool `{}`: {error}",
                                invocation.tool_name
                            ),
                        })?;
                        break None;
                    }
                    thread::sleep(Duration::from_millis(10));
                }
                Err(error) => {
                    return Err(ToolExecutionError {
                        kind: ToolExecutionErrorKind::WaitFailed,
                        message: format!(
                            "failed to wait for tool `{}`: {error}",
                            invocation.tool_name
                        ),
                    });
                }
            }
        };

        let stdout = join_reader(stdout_reader, &invocation.tool_name)?;
        let stderr = join_reader(stderr_reader, &invocation.tool_name)?;
        let duration_ms = start.elapsed().as_millis().min(u64::MAX as u128) as u64;
        let outcome = if timed_out {
            ToolExecutionOutcome::TimedOut
        } else if status.and_then(|status| status.code()).unwrap_or_default() == 0 {
            ToolExecutionOutcome::Succeeded
        } else {
            ToolExecutionOutcome::Failed
        };
        let json_output = if invocation.capture_json && outcome == ToolExecutionOutcome::Succeeded {
            Some(
                serde_json::from_str(&stdout).map_err(|error| ToolExecutionError {
                    kind: ToolExecutionErrorKind::InvalidJsonOutput,
                    message: format!(
                        "tool `{}` produced invalid JSON output: {error}",
                        invocation.tool_name
                    ),
                })?,
            )
        } else {
            None
        };

        Ok(ToolExecutionResult {
            tool_name: invocation.tool_name,
            command: command_line,
            cwd: invocation.cwd.display().to_string(),
            outcome,
            exit_code: if timed_out {
                None
            } else {
                status.and_then(|status| status.code())
            },
            duration_ms,
            stdout,
            stderr,
            json_output,
        })
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct ToolManifestLoadError {
    pub kind: ToolManifestLoadErrorKind,
    pub message: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ToolManifestLoadErrorKind {
    Io,
    InvalidJson,
    DuplicateToolName,
    InvalidManifest,
}

fn build_command_line(manifest: &ToolManifest, args: &[String]) -> Vec<String> {
    let mut command = Vec::with_capacity(1 + manifest.default_args.len() + args.len());
    command.push(manifest.executable.clone());
    command.extend(manifest.default_args.clone());
    command.extend(args.iter().cloned());
    command
}

fn spawn_reader<R>(stream: Option<R>) -> thread::JoinHandle<io::Result<Vec<u8>>>
where
    R: Read + Send + 'static,
{
    thread::spawn(move || {
        let mut bytes = Vec::new();
        if let Some(mut stream) = stream {
            stream.read_to_end(&mut bytes)?;
        }
        Ok(bytes)
    })
}

fn join_reader(
    handle: thread::JoinHandle<io::Result<Vec<u8>>>,
    tool_name: &str,
) -> Result<String, ToolExecutionError> {
    let bytes = handle.join().map_err(|_| ToolExecutionError {
        kind: ToolExecutionErrorKind::OutputReadFailed,
        message: format!("failed to join output reader for tool `{tool_name}`"),
    })?;
    let bytes = bytes.map_err(|error| ToolExecutionError {
        kind: ToolExecutionErrorKind::OutputReadFailed,
        message: format!("failed to read output for tool `{tool_name}`: {error}"),
    })?;
    Ok(String::from_utf8_lossy(&bytes).into_owned())
}

fn kill_child(child: &mut std::process::Child, tool_name: &str) -> Result<(), ToolExecutionError> {
    match child.kill() {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == io::ErrorKind::InvalidInput => Ok(()),
        Err(error) => Err(ToolExecutionError {
            kind: ToolExecutionErrorKind::WaitFailed,
            message: format!("failed to terminate timed out tool `{tool_name}`: {error}"),
        }),
    }
}
