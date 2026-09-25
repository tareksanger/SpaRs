use super::*;

#[test]
fn prepares_only_the_default_branch_workflow_from_the_repository_root() {
    run(&[], |command| {
        assert_eq!(command.get_program(), "gh");
        assert_eq!(
            command.get_args().collect::<Vec<_>>(),
            ["workflow", "run", "release.yml"]
        );
        assert_eq!(
            command.get_current_dir().unwrap().canonicalize().unwrap(),
            Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../..")
                .canonicalize()
                .unwrap()
        );
        Ok(true)
    })
    .unwrap();
}

#[test]
fn preview_and_help_do_not_invoke_github() {
    for option in ["--dry-run", "--help", "-h"] {
        run(&[option.into()], |_| {
            panic!("preview/help must not dispatch")
        })
        .unwrap();
    }
}

#[test]
fn unsupported_options_fail_before_dispatch() {
    for args in [
        vec!["--publish".into()],
        vec!["--ref".into(), "branch".into()],
        vec!["--dry-run".into(), "extra".into()],
    ] {
        assert!(run(&args, |_| panic!("invalid arguments must not dispatch")).is_err());
    }
}

#[test]
fn dispatch_failures_are_reported() {
    assert!(run(&[], |_| Ok(false))
        .unwrap_err()
        .contains("failed to dispatch"));
    assert!(run(&[], |_| Err(io::Error::from(io::ErrorKind::NotFound)))
        .unwrap_err()
        .contains("Install gh"));
}
