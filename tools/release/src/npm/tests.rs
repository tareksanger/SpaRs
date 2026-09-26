use super::*;

fn args(values: &[&str]) -> Vec<String> {
    values.iter().map(|value| (*value).into()).collect()
}

#[test]
fn workflow_dispatch_selects_publish_or_build_only() {
    for (options, publish) in [
        (args(&["v0.3.0"]), "publish=true"),
        (args(&["v0.3.0", "--build-only"]), "publish=false"),
    ] {
        run(&options, |command| {
            assert_eq!(command.get_program(), "gh");
            assert_eq!(
                command.get_args().collect::<Vec<_>>(),
                [
                    "workflow",
                    "run",
                    "npm-release.yml",
                    "-f",
                    "tag=v0.3.0",
                    "-f",
                    publish
                ]
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
}

#[test]
fn artifacts_are_one_literal_argument_to_the_existing_publisher() {
    run(
        &args(&["--artifacts", "target/my packages; echo nope"]),
        |command| {
            assert_eq!(command.get_program(), "node");
            assert_eq!(
                command.get_current_dir().unwrap().canonicalize().unwrap(),
                Path::new(env!("CARGO_MANIFEST_DIR"))
                    .join("../..")
                    .canonicalize()
                    .unwrap()
            );
            assert_eq!(
                command.get_args().collect::<Vec<_>>(),
                [
                    "bindings/node/scripts/release-publish.mts",
                    "target/my packages; echo nope"
                ]
            );
            Ok(true)
        },
    )
    .unwrap();
}

#[test]
fn invalid_or_ambiguous_options_never_invoke_a_process() {
    for options in [
        vec![],
        vec!["main"],
        vec!["v01.2.3"],
        vec!["v1.2"],
        vec!["v1.2.3-beta"],
        vec!["--artifacts"],
        vec!["--artifacts", ""],
        vec!["--artifacts", "--dry-run"],
        vec!["--artifacts", "target/packages", "--build-only"],
        vec!["v1.2.3", "--dry-run", "--dry-run"],
        vec!["v1.2.3", "--build-only", "--build-only"],
        vec!["v1.2.3", "extra"],
    ] {
        assert!(run(&args(&options), |_| panic!("invalid arguments executed")).is_err());
    }
}

#[test]
fn preview_help_and_process_errors_are_explicit() {
    for options in [
        vec!["--help"],
        vec!["v1.2.3", "--dry-run"],
        vec!["--artifacts", "target/packages", "--dry-run"],
    ] {
        run(&args(&options), |_| panic!("preview executed")).unwrap();
    }
    assert!(run(&args(&["v1.2.3"]), |_| Ok(false))
        .unwrap_err()
        .contains("failed"));
    for (options, hint) in [
        (args(&["v1.2.3"]), "Install gh"),
        (
            args(&["--artifacts", "target/packages"]),
            "Install Node.js 24",
        ),
    ] {
        assert!(run(&options, |_| Err(io::ErrorKind::NotFound.into()))
            .unwrap_err()
            .contains(hint));
    }
}
