use std::{io, path::Path, process::Command};

const HELP: &str = "Usage: cargo publish-npm <vX.Y.Z> [--build-only] [--dry-run]\n       cargo publish-npm --artifacts <directory> [--dry-run]\n\nWith a tag: start the npm workflow from the remote default branch using gh.\nWith --artifacts: publish the complete set of downloaded workflow tarballs using Node.js 24 and npm login.\n--build-only builds and tests without publishing.\n--dry-run prints the command without contacting GitHub or npm.\nSee docs/RELEASING.md for first-release setup.";

#[derive(Debug, PartialEq)]
enum Destination<'a> {
    Workflow { tag: &'a str, publish: bool },
    Artifacts(&'a str),
}

fn stable_tag(tag: &str) -> bool {
    let Some(version) = tag.strip_prefix('v') else {
        return false;
    };
    let parts: Vec<_> = version.split('.').collect();
    parts.len() == 3
        && parts.iter().all(|part| {
            !part.is_empty()
                && part.bytes().all(|byte| byte.is_ascii_digit())
                && (part.len() == 1 || !part.starts_with('0'))
        })
}

fn parse(args: &[String]) -> Result<(Destination<'_>, bool), String> {
    let (destination, options) = match args {
        [flag, directory, options @ ..]
            if flag == "--artifacts" && !directory.is_empty() && !directory.starts_with('-') =>
        {
            (Destination::Artifacts(directory), options)
        }
        [tag, options @ ..] if stable_tag(tag) => {
            (Destination::Workflow { tag, publish: true }, options)
        }
        _ => return Err(HELP.into()),
    };
    let mut dry_run = false;
    let mut build_only = false;
    for option in options {
        match option.as_str() {
            "--dry-run" if !dry_run => dry_run = true,
            "--build-only"
                if !build_only && matches!(destination, Destination::Workflow { .. }) =>
            {
                build_only = true
            }
            _ => return Err(HELP.into()),
        }
    }
    let destination = match destination {
        Destination::Workflow { tag, .. } => Destination::Workflow {
            tag,
            publish: !build_only,
        },
        other => other,
    };
    Ok((destination, dry_run))
}

pub(super) fn run(
    args: &[String],
    invoke: impl FnOnce(&mut Command) -> io::Result<bool>,
) -> Result<(), String> {
    if matches!(args, [option] if option == "--help" || option == "-h") {
        println!("{HELP}");
        return Ok(());
    }
    let (destination, dry_run) = parse(args)?;
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let mut command;
    let prerequisite;
    match destination {
        Destination::Workflow { tag, publish } => {
            command = Command::new("gh");
            command.args([
                "workflow",
                "run",
                "npm-release.yml",
                "-f",
                &format!("tag={tag}"),
                "-f",
                &format!("publish={publish}"),
            ]);
            prerequisite = "Install gh and authenticate with gh auth login.";
            println!("gh workflow run npm-release.yml -f tag={tag} -f publish={publish}");
            println!("Uses the remote default branch; local changes are not pushed. Check the workflow run for completion.");
        }
        Destination::Artifacts(directory) => {
            command = Command::new("node");
            command.args(["bindings/node/scripts/release-publish.mts", directory]);
            prerequisite = "Install Node.js 24 and authenticate with npm login.";
            println!("node bindings/node/scripts/release-publish.mts {directory:?}");
        }
    }
    if dry_run {
        return Ok(());
    }
    command.current_dir(root);
    match invoke(&mut command) {
        Ok(true) => Ok(()),
        Ok(false) => Err("npm release command failed; see its output above.".into()),
        Err(error) => Err(format!(
            "Could not run npm release command: {error}. {prerequisite}"
        )),
    }
}

#[cfg(test)]
mod tests;
