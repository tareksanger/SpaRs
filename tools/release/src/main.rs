mod npm;

use std::{
    io,
    path::Path,
    process::{Command, ExitCode},
};

const HELP: &str = "Usage: cargo release [--dry-run]\n\nPrepare or refresh the release PR from the remote default branch using GitHub CLI.\nRequires gh authenticated with repository write access.\n--dry-run prints the command without contacting GitHub.\nUse cargo publish for crates.io or cargo publish-npm --help for npm.";

fn run(
    args: &[String],
    invoke: impl FnOnce(&mut Command) -> io::Result<bool>,
) -> Result<(), String> {
    if let Some((command, rest)) = args.split_first() {
        if command == "npm" {
            return npm::run(rest, invoke);
        }
    }
    match args {
        [] => {}
        [option] if option == "--dry-run" => {
            println!("gh workflow run release.yml");
            println!("Uses the remote default branch for this checkout's GitHub repository; local changes are not pushed.");
            return Ok(());
        }
        [option] if option == "--help" || option == "-h" => {
            println!("{HELP}");
            return Ok(());
        }
        _ => return Err(HELP.into()),
    }
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let mut command = Command::new("gh");
    command
        .current_dir(root)
        .args(["workflow", "run", "release.yml"]);
    println!(
        "Preparing the release PR from the remote default branch; local changes are not pushed."
    );
    match invoke(&mut command) {
        Ok(true) => Ok(()),
        Ok(false) => {
            Err("GitHub CLI failed to dispatch the release workflow; see its output above.".into())
        }
        Err(error) => Err(format!(
            "Could not run GitHub CLI: {error}. Install gh and authenticate with gh auth login."
        )),
    }
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match run(&args, |command| {
        command.status().map(|status| status.success())
    }) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("{error}");
            ExitCode::FAILURE
        }
    }
}

#[cfg(test)]
mod tests;
