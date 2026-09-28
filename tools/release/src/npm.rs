use std::{io, path::Path, process::Command};

const HELP: &str = "Usage: cargo publish-npm <vX.Y.Z> [--build-only] [--source SHA] [--dry-run]\n       cargo publish-npm <vX.Y.Z> --download-only|--from-release [--output DIR] [--dry-run]\n       cargo publish-npm <vX.Y.Z> --attach-artifacts DIR [--dry-run]\n       cargo publish-npm --artifacts DIR [--dry-run]\n\nWith a tag: build, test, retain release assets, and publish through GitHub Actions.\n--build-only retains tested assets without publishing to npm.\n--source selects an exact historical source commit; workflow uses the remote default branch.\n--download-only verifies retained release files; --from-release also publishes them locally.\n--output selects a new empty directory; default creates a unique directory under target/npm-recovery.\n--attach-artifacts retries attachment of a downloaded npm-release-set artifact.\n--artifacts publishes an existing complete tarball set locally.\nLocal publication requires Node.js 24, npm login, and tar. GitHub operations require gh auth login.\n--dry-run prints the command without contacting GitHub or npm.";

#[derive(Debug, PartialEq)]
enum Destination<'a> {
    Workflow {
        tag: &'a str,
        publish: bool,
        source: Option<&'a str>,
    },
    Assets {
        tag: &'a str,
        mode: &'static str,
        directory: &'a str,
    },
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
    let Some(first) = args.first() else {
        return Err(HELP.into());
    };
    let mut options = args[1..].iter();
    let artifacts = if first == "--artifacts" {
        Some(
            options
                .next()
                .filter(|s| !s.is_empty() && !s.starts_with('-'))
                .ok_or(HELP)?,
        )
    } else {
        if !stable_tag(first) {
            return Err(HELP.into());
        }
        None
    };
    let mut dry_run = false;
    let mut build_only = false;
    let mut source = None;
    let mut mode = None;
    let mut output = None;
    while let Some(option) = options.next() {
        match option.as_str() {
            "--dry-run" if !dry_run => dry_run = true,
            "--build-only" if !build_only => build_only = true,
            "--source" if source.is_none() => {
                let sha = options.next().ok_or(HELP)?;
                if sha.len() != 40
                    || !sha
                        .bytes()
                        .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
                {
                    return Err(HELP.into());
                }
                source = Some(sha.as_str());
            }
            "--download-only" if mode.is_none() => mode = Some("download"),
            "--from-release" if mode.is_none() => mode = Some("recover"),
            "--attach-artifacts" if mode.is_none() && output.is_none() => {
                mode = Some("upload");
                output = Some(options.next().ok_or(HELP)?.as_str());
            }
            "--output" if output.is_none() => output = Some(options.next().ok_or(HELP)?.as_str()),
            _ => return Err(HELP.into()),
        }
    }
    if output.is_some_and(|s| s.is_empty() || s.starts_with('-')) {
        return Err(HELP.into());
    }
    let destination = if let Some(directory) = artifacts {
        if build_only || source.is_some() || mode.is_some() || output.is_some() {
            return Err(HELP.into());
        }
        Destination::Artifacts(directory)
    } else if let Some(mode) = mode {
        if build_only || source.is_some() {
            return Err(HELP.into());
        }
        Destination::Assets {
            tag: first,
            mode,
            directory: output.unwrap_or("-"),
        }
    } else {
        if output.is_some() {
            return Err(HELP.into());
        }
        Destination::Workflow {
            tag: first,
            publish: !build_only,
            source,
        }
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
        Destination::Workflow {
            tag,
            publish,
            source,
        } => {
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
            if let Some(sha) = source {
                command.args(["-f", &format!("source={sha}")]);
            }
            prerequisite = "Install gh and authenticate with gh auth login.";
            println!(
                "gh workflow run npm-release.yml -f tag={tag} -f publish={publish}{}",
                source
                    .map(|s| format!(" -f source={s}"))
                    .unwrap_or_default()
            );
            println!("Uses the remote default branch; local changes are not pushed. Check the workflow run for completion.");
        }
        Destination::Artifacts(directory) => {
            command = Command::new("node");
            command.args(["bindings/node/scripts/release-publish.mts", directory]);
            prerequisite = "Install Node.js 24 and authenticate with npm login.";
            println!("node bindings/node/scripts/release-publish.mts {directory:?}");
        }
        Destination::Assets {
            tag,
            mode,
            directory,
        } => {
            command = Command::new("node");
            command.args([
                "bindings/node/scripts/release-assets.mts",
                mode,
                tag,
                directory,
            ]);
            prerequisite = "Install Node.js 24 and gh; authenticate with gh auth login and, for publication, npm login.";
            println!("node bindings/node/scripts/release-assets.mts {mode} {tag} {directory:?}");
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
