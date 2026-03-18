#![forbid(unsafe_code)]

pub mod domain;
pub mod harness;

pub use domain::{
    AgentEvent, AgentEventPayload, AgentResult, AgentResultStatus, ArtifactEntrypoint,
    ArtifactEntrypointRole, ArtifactGroup, ArtifactIndex, ArtifactKind, ArtifactRef, Assignment,
    AssignmentConstraints, BlockerInfo, CheckOutcome, CheckResult, CodexOutputFormat,
    CodexOutputRef, HarnessReplayRecord, HarnessResult, HarnessStatus, LogRef, LogStream,
    NetworkPolicy, ParameterSchema, RunState, SandboxPolicy, SchemaFormat, SkillPackRef,
    SkillPackScope, TimeoutPolicy, ToolExecutionOutcome, ToolSpec, ValidationResultRef,
    ValidationStatus, WorkItemRef,
};
