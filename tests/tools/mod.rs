use std::{collections::BTreeMap, fs, path::Path};

use codex_agents::domain::ToolExecutionOutcome;
use codex_agents::tools::{
    ToolExecutionErrorKind, ToolInvocation, ToolManifestCatalog, ToolManifestState, ToolRunner,
};
use tempfile::tempdir;

#[test]
fn repository_tool_manifests_load_and_can_be_inspected() {
    let repo_root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let catalog = ToolManifestCatalog::load_from_repo(repo_root).expect("manifest should load");

    let tool_names = catalog.tool_names();

    assert!(tool_names.contains(&"cargo".to_string()));
    assert!(tool_names.contains(&"git".to_string()));
    assert!(tool_names.contains(&"gh".to_string()));

    let github = catalog.get("gh").expect("gh manifest should exist");
    assert_eq!(github.executable, "gh");
    assert!(github.supports_json);
    assert_eq!(github.state, ToolManifestState::Stable);
}

#[test]
fn tool_runner_enforces_cwd_env_timeout_and_json_capture() {
    let repo_root = tempdir().expect("tempdir should exist");
    write_manifest(
        repo_root.path(),
        r#"{
  "version": 1,
  "tools": [
    {
      "name": "python-json",
      "description": "Run python for deterministic json output.",
      "executable": "python3",
      "default_args": [],
      "state": "stable",
      "supports_json": true,
      "timeout_policy": {
        "soft_timeout_seconds": 1,
        "hard_timeout_seconds": 2
      },
      "inherit_parent_env": true,
      "allowed_env": [
        "CODEX_VALUE"
      ],
      "usage_examples": [
        "python3 -c 'print(1)'"
      ]
    }
  ]
}"#,
    );

    let catalog = ToolManifestCatalog::load_from_repo(repo_root.path()).expect("manifest loads");
    let runner = ToolRunner::new(catalog);
    let cwd = tempdir().expect("cwd tempdir should exist");

    let result = runner
        .run(ToolInvocation {
            tool_name: "python-json".to_string(),
            cwd: cwd.path().to_path_buf(),
            args: vec![
                "-c".to_string(),
                "import json, os; print(json.dumps({'cwd': os.getcwd(), 'value': os.getenv('CODEX_VALUE')}))".to_string(),
            ],
            env: BTreeMap::from([("CODEX_VALUE".to_string(), "from-test".to_string())]),
            timeout_override_seconds: Some(1),
            capture_json: true,
        })
        .expect("tool execution should succeed");

    assert_eq!(result.outcome, ToolExecutionOutcome::Succeeded);
    assert_eq!(result.cwd, cwd.path().display().to_string());
    assert_eq!(
        result.json_output.expect("json output")["value"],
        "from-test"
    );
}

#[test]
fn tool_runner_rejects_undeclared_env_vars() {
    let repo_root = tempdir().expect("tempdir should exist");
    write_manifest(
        repo_root.path(),
        r#"{
  "version": 1,
  "tools": [
    {
      "name": "python-json",
      "description": "Run python for deterministic json output.",
      "executable": "python3",
      "default_args": [],
      "state": "stable",
      "supports_json": true,
      "timeout_policy": {
        "soft_timeout_seconds": 1,
        "hard_timeout_seconds": 2
      },
      "inherit_parent_env": true,
      "allowed_env": [],
      "usage_examples": [
        "python3 -c 'print(1)'"
      ]
    }
  ]
}"#,
    );

    let catalog = ToolManifestCatalog::load_from_repo(repo_root.path()).expect("manifest loads");
    let runner = ToolRunner::new(catalog);
    let cwd = tempdir().expect("cwd tempdir should exist");

    let error = runner
        .run(ToolInvocation {
            tool_name: "python-json".to_string(),
            cwd: cwd.path().to_path_buf(),
            args: vec!["-c".to_string(), "print('ok')".to_string()],
            env: BTreeMap::from([("SECRET_VALUE".to_string(), "blocked".to_string())]),
            timeout_override_seconds: Some(1),
            capture_json: false,
        })
        .expect_err("undeclared env should fail");

    assert_eq!(
        error.kind,
        ToolExecutionErrorKind::EnvironmentVariableNotAllowed
    );
}

#[test]
fn tool_runner_returns_timed_out_outcome_when_process_exceeds_policy() {
    let repo_root = tempdir().expect("tempdir should exist");
    write_manifest(
        repo_root.path(),
        r#"{
  "version": 1,
  "tools": [
    {
      "name": "python-sleeper",
      "description": "Run python for timeout testing.",
      "executable": "python3",
      "default_args": [],
      "state": "stable",
      "supports_json": false,
      "timeout_policy": {
        "soft_timeout_seconds": 1,
        "hard_timeout_seconds": 1
      },
      "inherit_parent_env": true,
      "allowed_env": [],
      "usage_examples": [
        "python3 -c 'import time; time.sleep(1)'"
      ]
    }
  ]
}"#,
    );

    let catalog = ToolManifestCatalog::load_from_repo(repo_root.path()).expect("manifest loads");
    let runner = ToolRunner::new(catalog);
    let cwd = tempdir().expect("cwd tempdir should exist");

    let result = runner
        .run(ToolInvocation {
            tool_name: "python-sleeper".to_string(),
            cwd: cwd.path().to_path_buf(),
            args: vec![
                "-c".to_string(),
                "import time; time.sleep(2); print('late')".to_string(),
            ],
            env: BTreeMap::new(),
            timeout_override_seconds: None,
            capture_json: false,
        })
        .expect("timeout should still return an execution result");

    assert_eq!(result.outcome, ToolExecutionOutcome::TimedOut);
    assert_eq!(result.exit_code, None);
}

#[test]
fn tool_runner_does_not_try_to_parse_json_after_timeout() {
    let repo_root = tempdir().expect("tempdir should exist");
    write_manifest(
        repo_root.path(),
        r#"{
  "version": 1,
  "tools": [
    {
      "name": "python-json-timeout",
      "description": "Run python for timeout testing with json capture.",
      "executable": "python3",
      "default_args": [],
      "state": "stable",
      "supports_json": true,
      "timeout_policy": {
        "soft_timeout_seconds": 1,
        "hard_timeout_seconds": 1
      },
      "inherit_parent_env": true,
      "allowed_env": [],
      "usage_examples": [
        "python3 -c 'import time; time.sleep(1)'"
      ]
    }
  ]
}"#,
    );

    let catalog = ToolManifestCatalog::load_from_repo(repo_root.path()).expect("manifest loads");
    let runner = ToolRunner::new(catalog);
    let cwd = tempdir().expect("cwd tempdir should exist");

    let result = runner
        .run(ToolInvocation {
            tool_name: "python-json-timeout".to_string(),
            cwd: cwd.path().to_path_buf(),
            args: vec![
                "-c".to_string(),
                "import json, time; time.sleep(2); print(json.dumps({'late': True}))".to_string(),
            ],
            env: BTreeMap::new(),
            timeout_override_seconds: None,
            capture_json: true,
        })
        .expect("timeout should still return an execution result");

    assert_eq!(result.outcome, ToolExecutionOutcome::TimedOut);
    assert_eq!(result.json_output, None);
}

fn write_manifest(repo_root: &Path, contents: &str) {
    let manifest_dir = repo_root.join("manifests");
    fs::create_dir_all(&manifest_dir).expect("manifest directory should exist");
    fs::write(manifest_dir.join("tools.json"), contents).expect("manifest should be written");
}
