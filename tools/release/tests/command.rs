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
    assert!(String::from_utf8(preview.stdout)
        .unwrap()
        .contains("gh workflow run release.yml --ref main"));
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
