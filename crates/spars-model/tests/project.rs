use spars::{Model, ModelStore};
use spars_model::{download_model, DownloadOptions};
use std::{fs, path::PathBuf};
#[test]
#[ignore = "requires official sm wheel; mandatory acceptance"]
fn rust_download_and_short_cli_share_environment_resolution() {
    let project = std::env::temp_dir().join(format!("spars-project-{}", std::process::id()));
    fs::create_dir_all(&project).unwrap();
    let project = fs::canonicalize(project).unwrap();
    let archive = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../assets/en_core_web_sm-3.8.0-py3-none-any.whl");
    let store = project.join("models");
    let options = DownloadOptions {
        path: Some(store.clone()),
        archive: Some(archive.clone()),
        version: None,
    };
    let first = download_model("en_core_web_sm", options.clone()).unwrap();
    let second = download_model("en_core_web_sm", options).unwrap();
    assert_eq!(first.path, second.path);
    let model_store = ModelStore::new(&store);
    model_store.register("en_core_web_md", &first.path).unwrap();
    assert!(model_store
        .load("en_core_web_md")
        .err()
        .unwrap()
        .to_string()
        .contains("identity"));
    assert!(ModelStore::new(&store)
        .load("en_core_web_sm")
        .unwrap()
        .process("Alice works in London.")
        .unwrap()
        .entities()
        .is_some());
    assert!(!project.join("spars.json").exists());
    let cli = env!("CARGO_BIN_EXE_spars");
    let run = std::process::Command::new(cli)
        .current_dir(&project)
        .env("SPARS_MODEL_DIR", &store)
        .args(["download", "en_core_web_sm", "--archive"])
        .arg(&archive)
        .output()
        .unwrap();
    assert!(
        run.status.success(),
        "{}",
        String::from_utf8_lossy(&run.stderr)
    );
    assert_eq!(
        PathBuf::from(String::from_utf8(run.stdout).unwrap().trim()),
        first.path
    );
    let child = std::process::Command::new(std::env::current_exe().unwrap())
        .current_dir(&project)
        .env("SPARS_MODEL_DIR", &store)
        .env("SPARS_LOAD_CHILD", "1")
        .args(["--exact", "load_child", "--nocapture"])
        .output()
        .unwrap();
    assert!(
        child.status.success(),
        "{}",
        String::from_utf8_lossy(&child.stderr)
    );
    for args in [
        vec!["download"],
        vec!["download", "en_core_web_sm", "--path"],
        vec!["download", "en_core_web_sm", "--version", "9.0.0"],
        vec!["download", "../bad"],
        vec!["download", "en_core_web_sm", "--path", "", "--path", "x"],
    ] {
        let result = std::process::Command::new(cli)
            .current_dir(&project)
            .env("SPARS_MODEL_DIR", &store)
            .args(args)
            .output()
            .unwrap();
        assert!(!result.status.success());
    }
    fs::remove_dir_all(project).unwrap();
}
#[test]
fn load_child() {
    if std::env::var_os("SPARS_LOAD_CHILD").is_some() {
        // A same-named local directory must not shadow the configured model.
        fs::create_dir("en_core_web_sm").unwrap();
        let model = Model::load("en_core_web_sm").unwrap();
        let doc = model.process("Alice works in London.").unwrap();
        assert_eq!(model.document_vector(&doc).len(), 96);
        assert!(Model::load("not_installed")
            .err()
            .unwrap()
            .to_string()
            .contains("spars download not_installed"));
    }
}
