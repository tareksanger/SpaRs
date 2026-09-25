mod cli;
use spars_model::{catalog, install, list, verify, ModelName, Version};
use std::{ffi::OsString, path::PathBuf};
fn main() {
    if let Err(error) = run() {
        eprintln!("{error}");
        std::process::exit(1);
    }
}
fn run() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args_os().skip(1);
    let command=args.next().ok_or("usage: spars-model catalog | install --version VERSION --root DIR [--archive WHEEL] | list --root DIR | verify PATH")?;
    if command == "--help" || command == "-h" || command == "help" {
        if args.next().is_some() {
            return Err("help takes no arguments".into());
        }
        println!("spars download MODEL [--path DIR] [--version VERSION] [--archive WHEEL]\nDirectory: SPARS_MODEL_DIR or the user cache.\nLegacy: spars-model catalog | install --version VERSION --root DIR | list --root DIR | verify PATH");
        return Ok(());
    }
    if command == "download" {
        return cli::project_command(&command, args);
    }
    if command == "catalog" {
        if args.next().is_some() {
            return Err("catalog takes no arguments".into());
        }
        println!("{}", serde_json::to_string_pretty(&catalog()?)?);
        return Ok(());
    }
    if command == "verify" {
        let path = PathBuf::from(args.next().ok_or("verify requires a path")?);
        if args.next().is_some() {
            return Err("verify takes one path".into());
        }
        let installed = verify(&path)?;
        println!("{}", installed.path.display());
        return Ok(());
    }
    let mut root = None;
    let mut archive = None;
    let mut version = None;
    let mut model = None;
    while let Some(flag) = args.next() {
        let value = args.next().ok_or("each option requires a value")?;
        let target = match flag.to_str() {
            Some("--root") => &mut root,
            Some("--archive") => &mut archive,
            Some("--version") => &mut version,
            Some("--model") => &mut model,
            _ => return Err("unknown option".into()),
        };
        if target.replace(value).is_some() {
            return Err("duplicate option".into());
        }
    }
    let root = PathBuf::from(root.ok_or("--root is required")?);
    if command == "list" {
        if archive.is_some() || version.is_some() || model.is_some() {
            return Err("list accepts only --root".into());
        }
        for entry in list(&root)? {
            println!("{}", entry.path.display());
        }
        return Ok(());
    }
    if command != "install" {
        return Err("unknown command".into());
    }
    let model = model.unwrap_or_else(|| OsString::from("en_core_web_md"));
    let model: ModelName = model.to_str().ok_or("model must be UTF-8")?.parse()?;
    let version: Version = version
        .ok_or("--version is required; no implicit latest model")?
        .to_str()
        .ok_or("version must be UTF-8")?
        .parse()?;
    let archive = archive.map(PathBuf::from);
    let installed = install(&root, archive.as_deref(), model, version)?;
    println!("{}", installed.path.display());
    Ok(())
}
