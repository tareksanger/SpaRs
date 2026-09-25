# Repository layout rules

Choose a project's location by its responsibility. Extend an existing project when the code belongs to its current responsibility; create a separate project when it needs its own public interface, dependencies, or packaging boundary. Keep the repository root for workspace configuration, shared development tools, and repository metadata.

## Project placement

| Responsibility | Location | Reason |
|---|---|---|
| Native Rust library or command-line tool | `crates/<crate-directory>/` | Keeps native implementation projects together under one Cargo workspace. The library is `crates/spars/`; native model acquisition and conversion belong to `crates/spars-model/`. |
| Binding for another programming language | `bindings/<language>/` | Keeps the Rust adapter, language package, declarations, and language-specific tests together. The Node package is `bindings/node/`. Its Rust crate is a workspace member. |
| Independent consumer verification | Existing `consumer/`; new projects under `tests/consumers/<name>/` | Exercises the library as a dependency from outside the workspace, including independent dependency resolution and build configuration. These are test applications rather than shipped products. |
| Development, export, reference, or verification tooling | `tools/` | Keeps maintainer tooling separate from production inference and ordinary library builds. |
| Downstream application | Separate repository | Keeps application features and their dependencies outside the NLP library's scope. |

Keep each crate's production code in its `src/`, integration tests in its `tests/`, and runnable examples in its `examples/`. Keep unit tests in separate `tests.rs` modules as required by [AGENTS.md](../AGENTS.md). Shared frozen evaluation data belongs in `fixtures/`; model acquisition records belong in `models/`; pinned upstream provenance belongs in `reference/`. Keep crate-specific embedded resources with the crate that consumes them, with their licenses and provenance intact.

## Cargo ownership

The root [Cargo.toml](../Cargo.toml) remains a virtual workspace: it owns workspace membership, exclusions, dependency resolution, and shared build profiles. Package manifests and production source belong inside their project directories. Add each production Rust crate, including Rust binding crates, to the root workspace. Members share the root `Cargo.lock` and `target/`; use the workspace lockfile instead of adding member lockfiles or nested workspace declarations.

Independent consumer checks are the exception. Give each its own workspace declaration and lockfile, explicitly exclude it from the root workspace, and verify it separately. Preserve the existing `consumer/` check when adding another consumer. Its separation tests what workspace feature unification and shared resolution can otherwise conceal.

Keep `default-members` set to the core library unless intentionally changing root command behavior. Use `--workspace` with supported commands to check all members, and `-p` to select one package. See [development commands](DEVELOPMENT.md#cargo-workspace). Shared workspace membership does not make installer or binding dependencies part of the library's public dependency graph.

## Supporting files and generated output

Put user and contributor guides in `docs/`, agent instructions in `AGENTS.md` and its linked rule files, and automation in `.github/`. Keep downloaded models and upstream checkouts in their existing ignored acquisition directories. Put temporary diagnostics and build output under ignored `target/`, with generated verification and benchmark reports in `target/reports/`. Commit reusable tools, frozen fixtures, checksums, and provenance alongside their owning project or shared data directory.

## Adding or moving a project

Update Cargo membership and path dependencies together with the project files. Reconcile embedded resource paths, test assets and scratch directories, developer commands, CI, dependency update configuration, release version updates, package inventories, and source paths used by diagnostic tools. Preserve shared licenses and provenance when changing package contents.

Run the workspace metadata test in `tools/test_workspace.py`, locked Cargo checks for affected projects, and the affected release-tooling tests. For a production crate or binding relocation, run `.venv/bin/python tools/verify.py` and exercise any changed installation command. Confirm independent consumers still build separately and frozen fixtures remain unchanged. Follow the review and acceptance requirements in [AGENTS.md](../AGENTS.md) and the [quality guide](QUALITY.md).
