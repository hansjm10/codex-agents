use std::{fs, path::Path};

use codex_agents::skills::{SkillCatalog, SkillValidationFindingKind};
use codex_agents::tools::ToolManifestCatalog;
use tempfile::tempdir;

#[test]
fn repository_skills_validate_against_the_real_tool_manifest() {
    let repo_root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let tool_catalog = ToolManifestCatalog::load_from_repo(repo_root).expect("manifest loads");
    let skill_catalog = SkillCatalog::discover(repo_root).expect("skills should load");

    let report = skill_catalog.validate_against(&tool_catalog);

    assert!(report.is_clean(), "expected clean report, got: {report:#?}");
}

#[test]
fn skill_validation_reports_unknown_and_deprecated_tool_references() {
    let repo_root = tempdir().expect("tempdir should exist");

    write_manifest(
        repo_root.path(),
        r#"{
  "version": 1,
  "tools": [
    {
      "name": "known-tool",
      "description": "A stable tool.",
      "executable": "python3",
      "default_args": [],
      "state": "stable",
      "supports_json": false,
      "timeout_policy": {
        "soft_timeout_seconds": 1,
        "hard_timeout_seconds": 2
      },
      "inherit_parent_env": true,
      "allowed_env": [],
      "usage_examples": [
        "known-tool"
      ]
    },
    {
      "name": "deprecated-tool",
      "description": "A deprecated tool.",
      "executable": "python3",
      "default_args": [],
      "state": "deprecated",
      "supports_json": false,
      "timeout_policy": {
        "soft_timeout_seconds": 1,
        "hard_timeout_seconds": 2
      },
      "inherit_parent_env": true,
      "allowed_env": [],
      "usage_examples": [
        "deprecated-tool"
      ]
    }
  ]
}"#,
    );
    write_skill(
        repo_root.path(),
        "example",
        r#"---
name: example
description: Example validation target.
---

# Example

Use `known-tool --json` when things are healthy.

```sh
deprecated-tool --flag
missing-tool run
branch=$(known-tool status)
```
"#,
    );

    let tool_catalog = ToolManifestCatalog::load_from_repo(repo_root.path()).expect("manifest");
    let skill_catalog = SkillCatalog::discover(repo_root.path()).expect("skills");

    let report = skill_catalog.validate_against(&tool_catalog);

    assert!(!report.is_clean());
    assert!(report.findings.iter().any(|finding| {
        finding.kind == SkillValidationFindingKind::DeprecatedToolReference
            && finding.tool_name.as_deref() == Some("deprecated-tool")
    }));
    assert!(report.findings.iter().any(|finding| {
        finding.kind == SkillValidationFindingKind::UnknownToolReference
            && finding.tool_name.as_deref() == Some("missing-tool")
    }));
}

fn write_manifest(repo_root: &Path, contents: &str) {
    let manifest_dir = repo_root.join("manifests");
    fs::create_dir_all(&manifest_dir).expect("manifest directory should exist");
    fs::write(manifest_dir.join("tools.json"), contents).expect("manifest should be written");
}

fn write_skill(repo_root: &Path, name: &str, body: &str) {
    let skill_dir = repo_root.join(".codex/skills").join(name);
    fs::create_dir_all(&skill_dir).expect("skill directory should exist");
    fs::write(skill_dir.join("SKILL.md"), body).expect("skill should be written");
}
