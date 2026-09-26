use std::process::Command;

fn isolated_command() -> Command {
    let mut command = Command::new(env!("CARGO_BIN_EXE_spars-release-command"));
    // No GitHub CLI is reachable: these process tests cannot dispatch a release.
    command.env("PATH", "");
    command
}

#[test]
fn preview_and_help_succeed_without_github_cli() {
    let preview = isolated_command().arg("--dry-run").output().unwrap();
    assert!(preview.status.success());
    assert_eq!(
        String::from_utf8(preview.stdout).unwrap().lines().next(),
        Some("gh workflow run release.yml")
    );
    assert!(isolated_command().arg("--help").status().unwrap().success());
}

#[test]
fn invalid_arguments_exit_unsuccessfully() {
    let output = isolated_command().arg("--publish").output().unwrap();
    assert!(!output.status.success());
    assert!(String::from_utf8(output.stderr).unwrap().contains("Usage:"));
}

#[test]
fn missing_github_cli_exits_unsuccessfully() {
    let output = isolated_command().output().unwrap();
    assert!(!output.status.success());
    assert!(String::from_utf8(output.stderr)
        .unwrap()
        .contains("Install gh"));
}

#[test]
fn npm_previews_succeed_without_node_or_github() {
    for (args, expected) in [
        (
            vec!["npm", "v0.3.0", "--dry-run"],
            "gh workflow run npm-release.yml -f tag=v0.3.0 -f publish=true",
        ),
        (
            vec!["npm", "v0.3.0", "--build-only", "--dry-run"],
            "gh workflow run npm-release.yml -f tag=v0.3.0 -f publish=false",
        ),
        (
            vec!["npm", "--artifacts", "target/npm-bootstrap", "--dry-run"],
            "node bindings/node/scripts/release-publish.mts",
        ),
    ] {
        let output = isolated_command().args(args).output().unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(String::from_utf8(output.stdout).unwrap().contains(expected));
    }
}

#[test]
fn npm_missing_tools_fail_with_setup_instructions() {
    for (args, expected) in [
        (vec!["npm", "v0.3.0"], "Install gh"),
        (
            vec!["npm", "--artifacts", "target/npm-bootstrap"],
            "Install Node.js 24",
        ),
    ] {
        let output = isolated_command().args(args).output().unwrap();
        assert!(!output.status.success());
        assert!(String::from_utf8(output.stderr).unwrap().contains(expected));
    }
}
