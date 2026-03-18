use std::{collections::BTreeSet, fs, path::Path};

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_yaml::Value;

use crate::tools::{ToolManifestCatalog, ToolManifestState};

pub const SKILL_ROOT_RELATIVE_PATH: &str = ".codex/skills";

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SkillCatalog {
    pub skills: Vec<SkillDocument>,
}

impl SkillCatalog {
    pub fn discover(repo_root: &Path) -> Result<Self, SkillCatalogError> {
        let skill_root = repo_root.join(SKILL_ROOT_RELATIVE_PATH);
        if !skill_root.is_dir() {
            return Ok(Self { skills: Vec::new() });
        }

        let mut skill_paths = Vec::new();
        for entry in fs::read_dir(&skill_root).map_err(|error| SkillCatalogError {
            message: format!(
                "failed to read skill directory {}: {error}",
                skill_root.display()
            ),
        })? {
            let entry = entry.map_err(|error| SkillCatalogError {
                message: format!(
                    "failed to read a skill entry under {}: {error}",
                    skill_root.display()
                ),
            })?;
            let candidate = entry.path().join("SKILL.md");
            if candidate.is_file() {
                skill_paths.push(candidate);
            }
        }
        skill_paths.sort();

        let skills = skill_paths
            .into_iter()
            .map(|path| load_skill_document(repo_root, &path))
            .collect::<Result<Vec<_>, _>>()?;

        Ok(Self { skills })
    }

    pub fn validate_against(&self, tools: &ToolManifestCatalog) -> SkillValidationReport {
        let mut entries = Vec::new();
        let mut findings = Vec::new();

        for skill in &self.skills {
            let entry = validate_skill(skill, tools);
            findings.extend(entry.findings.iter().cloned());
            entries.push(entry);
        }

        SkillValidationReport {
            skills: entries,
            findings,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SkillDocument {
    pub directory_name: String,
    pub declared_name: Option<String>,
    pub description: Option<String>,
    pub path: String,
    body: String,
    frontmatter_error: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct SkillValidationReport {
    pub skills: Vec<SkillValidationEntry>,
    pub findings: Vec<SkillValidationFinding>,
}

impl SkillValidationReport {
    pub fn is_clean(&self) -> bool {
        self.findings.is_empty()
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct SkillValidationEntry {
    pub skill_name: String,
    pub path: String,
    pub referenced_tools: Vec<String>,
    pub findings: Vec<SkillValidationFinding>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct SkillValidationFinding {
    pub skill_name: String,
    pub path: String,
    pub kind: SkillValidationFindingKind,
    pub message: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tool_name: Option<String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum SkillValidationFindingKind {
    InvalidFrontmatter,
    SkillNameMismatch,
    UnknownToolReference,
    DeprecatedToolReference,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SkillCatalogError {
    pub message: String,
}

#[derive(Debug, Deserialize)]
struct SkillFrontmatter {
    name: Option<String>,
    description: Option<Value>,
}

fn load_skill_document(repo_root: &Path, path: &Path) -> Result<SkillDocument, SkillCatalogError> {
    let contents = fs::read_to_string(path).map_err(|error| SkillCatalogError {
        message: format!("failed to read skill {}: {error}", path.display()),
    })?;
    let directory_name = path
        .parent()
        .and_then(Path::file_name)
        .and_then(|name| name.to_str())
        .ok_or_else(|| SkillCatalogError {
            message: format!(
                "skill path {} does not have a valid directory name",
                path.display()
            ),
        })?
        .to_string();
    let relative_path = make_relative(repo_root, path);
    let (frontmatter, body, frontmatter_error) = parse_frontmatter(&contents);

    Ok(SkillDocument {
        directory_name,
        declared_name: frontmatter.as_ref().and_then(|value| value.name.clone()),
        description: frontmatter
            .as_ref()
            .and_then(|value| yaml_string(value.description.as_ref())),
        path: relative_path,
        body,
        frontmatter_error,
    })
}

fn validate_skill(skill: &SkillDocument, tools: &ToolManifestCatalog) -> SkillValidationEntry {
    let skill_name = skill
        .declared_name
        .clone()
        .unwrap_or_else(|| skill.directory_name.clone());
    let referenced_tools = extract_tool_references(&skill.body);
    let mut findings = Vec::new();

    if let Some(error) = &skill.frontmatter_error {
        findings.push(SkillValidationFinding {
            skill_name: skill_name.clone(),
            path: skill.path.clone(),
            kind: SkillValidationFindingKind::InvalidFrontmatter,
            message: error.clone(),
            tool_name: None,
        });
    }
    if let Some(declared_name) = &skill.declared_name
        && declared_name != &skill.directory_name
    {
        findings.push(SkillValidationFinding {
            skill_name: skill_name.clone(),
            path: skill.path.clone(),
            kind: SkillValidationFindingKind::SkillNameMismatch,
            message: format!(
                "skill directory `{}` does not match declared name `{declared_name}`",
                skill.directory_name
            ),
            tool_name: None,
        });
    }

    for tool_name in &referenced_tools {
        match tools.get(tool_name) {
            Some(manifest) if manifest.state == ToolManifestState::Deprecated => {
                findings.push(SkillValidationFinding {
                    skill_name: skill_name.clone(),
                    path: skill.path.clone(),
                    kind: SkillValidationFindingKind::DeprecatedToolReference,
                    message: format!(
                        "skill references deprecated tool `{tool_name}` instead of a supported manifest entry"
                    ),
                    tool_name: Some(tool_name.clone()),
                });
            }
            Some(_) => {}
            None => {
                findings.push(SkillValidationFinding {
                    skill_name: skill_name.clone(),
                    path: skill.path.clone(),
                    kind: SkillValidationFindingKind::UnknownToolReference,
                    message: format!(
                        "skill references tool `{tool_name}` that is not present in manifests/tools.json"
                    ),
                    tool_name: Some(tool_name.clone()),
                });
            }
        }
    }

    SkillValidationEntry {
        skill_name,
        path: skill.path.clone(),
        referenced_tools,
        findings,
    }
}

fn parse_frontmatter(contents: &str) -> (Option<SkillFrontmatter>, String, Option<String>) {
    let mut lines = contents.lines();
    if lines.next() != Some("---") {
        return (
            None,
            contents.to_string(),
            Some("skill is missing YAML frontmatter".to_string()),
        );
    }

    let mut frontmatter_lines = Vec::new();
    let mut found_terminator = false;
    for line in &mut lines {
        if line.trim() == "---" {
            found_terminator = true;
            break;
        }
        frontmatter_lines.push(line);
    }

    let remaining = lines.collect::<Vec<_>>().join("\n");
    if !found_terminator {
        return (
            None,
            remaining,
            Some("skill frontmatter is not terminated with `---`".to_string()),
        );
    }

    let raw_frontmatter = frontmatter_lines.join("\n");
    match serde_yaml::from_str::<SkillFrontmatter>(&raw_frontmatter) {
        Ok(frontmatter) => (Some(frontmatter), remaining, None),
        Err(error) => (
            None,
            remaining,
            Some(format!("skill frontmatter could not be parsed: {error}")),
        ),
    }
}

fn extract_tool_references(body: &str) -> Vec<String> {
    let mut tools = BTreeSet::new();
    let mut in_shell_block = false;

    for line in body.lines() {
        let trimmed = line.trim();
        if let Some(language) = trimmed.strip_prefix("```") {
            if in_shell_block {
                in_shell_block = false;
            } else {
                let language = language.trim();
                in_shell_block = matches!(language, "sh" | "bash" | "shell");
            }
            continue;
        }

        if in_shell_block {
            collect_references_from_snippet(trimmed, &mut tools);
            continue;
        }

        for snippet in inline_code_spans(line) {
            if inline_snippet_looks_like_command(&snippet) {
                collect_references_from_snippet(&snippet, &mut tools);
            }
        }
    }

    tools.into_iter().collect()
}

fn collect_references_from_snippet(snippet: &str, tools: &mut BTreeSet<String>) {
    for segment in split_pipeline_segments(snippet) {
        collect_substitution_commands(segment, tools);
        if let Some(command) = first_command_token(segment) {
            tools.insert(command);
        }
    }
}

fn collect_substitution_commands(snippet: &str, tools: &mut BTreeSet<String>) {
    let mut remaining = snippet;
    while let Some(start) = remaining.find("$(") {
        let after = &remaining[start + 2..];
        let end = after.find(')').unwrap_or(after.len());
        let nested = &after[..end];
        if let Some(command) = first_command_token(nested) {
            tools.insert(command);
        }
        remaining = after
            .get(end + usize::from(end < after.len())..)
            .unwrap_or_default();
    }
}

fn first_command_token(snippet: &str) -> Option<String> {
    for token in snippet.split_whitespace() {
        let cleaned = clean_token(token);
        if cleaned.is_empty() {
            continue;
        }
        if is_assignment(&cleaned) {
            if cleaned.contains("$(") {
                return None;
            }
            continue;
        }
        if SHELL_KEYWORDS.contains(&cleaned.as_str()) || SHELL_BUILTINS.contains(&cleaned.as_str())
        {
            return None;
        }
        if is_command_name(&cleaned) {
            return Some(cleaned);
        }
        return None;
    }
    None
}

fn inline_code_spans(line: &str) -> Vec<String> {
    let mut spans = Vec::new();
    let mut in_span = false;
    let mut current = String::new();

    for ch in line.chars() {
        if ch == '`' {
            if in_span {
                spans.push(current.clone());
                current.clear();
            }
            in_span = !in_span;
            continue;
        }
        if in_span {
            current.push(ch);
        }
    }

    spans
}

fn clean_token(token: &str) -> String {
    token
        .trim_matches(|ch: char| matches!(ch, '"' | '\'' | '`' | '(' | ')' | ',' | ';'))
        .trim_start_matches("$(")
        .trim_end_matches(')')
        .to_string()
}

fn is_assignment(token: &str) -> bool {
    let Some((lhs, _)) = token.split_once('=') else {
        return false;
    };
    !lhs.is_empty()
        && lhs
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '_' | '-'))
}

fn is_command_name(token: &str) -> bool {
    let mut chars = token.chars();
    matches!(chars.next(), Some(ch) if ch.is_ascii_lowercase() || ch.is_ascii_digit())
        && chars.all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '.' | '-'))
}

fn inline_snippet_looks_like_command(snippet: &str) -> bool {
    let trimmed = snippet.trim();
    trimmed.contains(' ') || trimmed.contains("$(") || trimmed.contains('|')
}

fn split_pipeline_segments(snippet: &str) -> Vec<&str> {
    let mut segments = Vec::new();
    let mut start = 0;
    let mut in_single_quotes = false;
    let mut in_double_quotes = false;
    let chars = snippet.char_indices().collect::<Vec<_>>();

    for (index, ch) in &chars {
        match ch {
            '\'' if !in_double_quotes => in_single_quotes = !in_single_quotes,
            '"' if !in_single_quotes => in_double_quotes = !in_double_quotes,
            '|' if !in_single_quotes && !in_double_quotes => {
                let previous = snippet[..*index].chars().next_back();
                let next = snippet[index + ch.len_utf8()..].chars().next();
                if previous == Some('|') || next == Some('|') {
                    continue;
                }
                segments.push(snippet[start..*index].trim());
                start = index + ch.len_utf8();
            }
            _ => {}
        }
    }

    segments.push(snippet[start..].trim());
    segments
        .into_iter()
        .filter(|segment| !segment.is_empty())
        .collect()
}

fn make_relative(repo_root: &Path, path: &Path) -> String {
    path.strip_prefix(repo_root)
        .unwrap_or(path)
        .display()
        .to_string()
}

fn yaml_string(value: Option<&Value>) -> Option<String> {
    match value {
        Some(Value::String(value)) => Some(value.clone()),
        Some(other) => serde_yaml::to_string(other)
            .ok()
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty()),
        None => None,
    }
}

const SHELL_KEYWORDS: &[&str] = &["do", "done", "else", "fi", "for", "if", "then", "while"];
const SHELL_BUILTINS: &[&str] = &["[", "echo", "exit", "set", "source", "true"];
