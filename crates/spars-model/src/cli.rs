use spars_model::{download_model, DownloadOptions};
use std::{ffi::OsString, path::PathBuf};
/// The short CLI shares the same API used by Node; legacy installation commands remain available.
pub fn project_command(
    command: &OsString,
    mut args: impl Iterator<Item = OsString>,
) -> Result<(), Box<dyn std::error::Error>> {
    if command != "download" {
        return Err("use `spars download MODEL [--path DIR]`".into());
    }
    let name = args.next().ok_or("download requires a model name")?;
    let mut path = None;
    let mut archive = None;
    let mut version = None;
    while let Some(flag) = args.next() {
        let value = args.next().ok_or("each option requires a value")?;
        let field = match flag.to_str() {
            Some("--path") => &mut path,
            Some("--archive") => &mut archive,
            Some("--version") => &mut version,
            _ => return Err("unknown option".into()),
        };
        if field.replace(value).is_some() {
            return Err("duplicate option".into());
        }
    }
    let options = DownloadOptions {
        path: path.map(PathBuf::from),
        archive: archive.map(PathBuf::from),
        version: version
            .map(|v| v.into_string().map_err(|_| "version must be UTF-8"))
            .transpose()?
            .map(|v| v.parse())
            .transpose()?,
    };
    let installed = download_model(name.to_str().ok_or("model name must be UTF-8")?, options)?;
    println!("{}", installed.path.display());
    Ok(())
}
