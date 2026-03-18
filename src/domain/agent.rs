use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::HarnessResult;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum RunState {
    Queued,
    Preparing,
    Running,
    Validating,
    Blocked,
    Completed,
    Failed,
    Cancelled,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum AgentResultStatus {
    Completed,
    Blocked,
    Failed,
    Cancelled,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct BlockerInfo {
    pub code: String,
    pub message: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub missing_capability: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub resolution_hint: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct AgentResult {
    pub run_id: String,
    pub assignment_id: String,
    pub status: AgentResultStatus,
    pub final_state: RunState,
    pub summary: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub observations: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub final_message: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub blocker: Option<BlockerInfo>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub harness_result: Option<HarnessResult>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct AgentEvent {
    pub sequence: u64,
    pub timestamp: String,
    pub state: RunState,
    #[serde(flatten)]
    pub payload: AgentEventPayload,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "event_type", rename_all = "snake_case")]
pub enum AgentEventPayload {
    RunStarted {
        run_id: String,
        assignment_id: String,
    },
    StateChanged {
        from: RunState,
        to: RunState,
    },
    ToolInvoked {
        invocation_id: String,
        tool_name: String,
    },
    ToolCompleted {
        invocation_id: String,
        tool_name: String,
        outcome: ToolExecutionOutcome,
    },
    ArtifactRecorded {
        artifact_id: String,
    },
    ValidationCompleted {
        status: ValidationStatus,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        failing_checks: Vec<String>,
    },
    Blocked {
        blocker: BlockerInfo,
    },
    RunCompleted {
        status: AgentResultStatus,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ToolExecutionOutcome {
    Succeeded,
    Failed,
    TimedOut,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ValidationStatus {
    Passed,
    Failed,
}
