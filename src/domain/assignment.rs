use std::collections::BTreeMap;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct Assignment {
    pub assignment_id: String,
    pub work_item: Option<WorkItemRef>,
    pub objective: String,
    pub repo_root: String,
    pub worktree_root: String,
    pub constraints: AssignmentConstraints,
    pub allowed_tools: Vec<ToolSpec>,
    pub skill_packs: Vec<SkillPackRef>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub metadata: BTreeMap<String, String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct WorkItemRef {
    pub system: String,
    pub id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct AssignmentConstraints {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_runtime_seconds: Option<u64>,
    pub require_clean_worktree: bool,
    pub network_policy: NetworkPolicy,
    pub sandbox_policy: SandboxPolicy,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum NetworkPolicy {
    Denied,
    Allowed,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum SandboxPolicy {
    ReadOnly,
    WorkspaceWrite,
    DangerFullAccess,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct ToolSpec {
    pub name: String,
    pub command: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub args_schema: Option<ParameterSchema>,
    pub supports_json: bool,
    pub timeout_policy: TimeoutPolicy,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub usage_examples: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct ParameterSchema {
    pub format: SchemaFormat,
    pub definition: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum SchemaFormat {
    JsonSchema,
    PlainText,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct TimeoutPolicy {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub soft_timeout_seconds: Option<u64>,
    pub hard_timeout_seconds: u64,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct SkillPackRef {
    pub name: String,
    pub path: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub revision_hint: Option<String>,
    pub scope: SkillPackScope,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum SkillPackScope {
    Assignment,
    Repository,
    Workspace,
}
