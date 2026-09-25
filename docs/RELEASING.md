# Prepare a release

Making the source repository public, publishing a Cargo or npm package, and distributing compiled binaries or model exports are separate actions. Each needs an explicit maintainer decision. The Node package and native installer are marked private or non-publishable; their tested installation route is a source build.

## Check the exact commit

Follow the [developer setup](DEVELOPMENT.md), then run:

```sh
.venv/bin/python tools/verify.py
git status --short
```

Verify the final commit in GitHub Actions: the Linux and macOS reference jobs, native installation job, and release-tooling job must pass. Review the reference job's report artifacts and the native installation job's logs when investigating failures. A local pass or a pass from an earlier commit does not establish release verification.

Review the README, compatibility inventory, model versions, and minimum toolchain versions. Confirm that examples execute and the Cargo package dry run succeeds. Keep generated binaries, models, reports, local environment files, and credentials out of the source distribution.

From a clean checkout, inspect the package contents and run the publication check without uploading:

```sh
cargo package --list
cargo publish --dry-run
```

The Cargo archive contains production Rust source, package metadata, the README, and the project, spaCy, and Thinc notices. Tests, fixtures, examples, contributor guides, model assets, model-specific notices, and development tools stay in the repository. Run the test suite from a repository checkout; test modules are not distributed in the crate. Models and the native installer retain their own required notices.

If Cargo lists unexpected files, correct the package's include rules before publishing. Explicit include rules override Git's ignore rules; a filename pattern without a directory can match files in nested folders. The [packaging regression test](../tools/test_package.py) checks those boundaries, and [package verification](../tools/check_package.py) builds a separate Rust application against the unpacked archive and runs model inference with an empty runtime PATH. Both checks run during acceptance. Avoid bypassing unexpected-file errors with `--allow-dirty`.

## Review what becomes public

Inspect the current tree and all history reachable through branches and tags for credentials, private input, personal filesystem paths, and unwanted artifacts. Review GitHub Actions logs and artifacts separately. Deleting a file from the latest commit does not remove historical copies. Rotate exposed credentials if any are found; a history rewrite alone cannot revoke them.

Any history rewrite must be coordinated with collaborators because it changes commit IDs and requires replacement of remote history. Keep a private backup and verify the resulting source tree before replacing shared refs. Enable private vulnerability reporting in the repository's security settings and check that the route in [SECURITY.md](../SECURITY.md) works before inviting reports. Configure required CI checks in branch protection or repository rules. Require pull requests, block force pushes and branch deletion, and keep the bypass list empty unless a documented operational need requires an exception. For a sole maintainer, zero required approvals permits their own pull requests to merge after CI passes. Review collaborators, deploy keys, installed applications, and Actions permissions before publication.

While the repository is public, enable secret scanning and push protection where available, and require approval before running workflows from outside contributors. Some security settings are unavailable while the repository is private; verify them again immediately after an explicitly approved visibility change. Keep workflow tokens read-only, disable persisted checkout credentials, and pin actions to full commit hashes. Dependabot proposes action, Cargo, and npm updates; audit the pinned Python reference requirements separately when preparing a release.

## Preserve applicable notices

The source includes the [project license](../LICENSE) and [third-party attribution](../THIRD_PARTY_NOTICES.md). Keep notices for translated code and copied resources. A model or language-resource update needs its own provenance and license review; do not assume all official models use identical terms.

Before distributing a compiled artifact, review the exact dependencies linked into it and include their required license texts. Audit the corresponding lockfile for published security advisories. Repeat that review for each Rust library, installer, Node addon, and target platform being distributed. A source-repository license or dependency lockfile alone does not fulfill every binary-distribution notice requirement.

Before registry publication, confirm package-name ownership, package contents, platform support, and consumer installation from the packaged artifact. Node publication additionally needs a deliberate platform-binary packaging strategy and its own clean-consumer tests. Those checks are separate from building the addon in this repository.

## Start a release when ready

Install GitHub CLI and authenticate with `gh auth login`, then run this command from a checkout of the repository to prepare a release:

```sh
gh workflow run release.yml --ref main
```

The workflow uses Release Please to calculate the next version from commits since the previous release and open or refresh a version/changelog PR. It uses the remote `main` branch, so merge changes intended for this release before running the command. Normal pushes and PRs do not prepare releases. If more changes land while the release PR is open, run the same command again to refresh it.

Review the proposed version and changelog, wait for PR checks, and merge the release PR when ready. Only merging a PR labeled `autorelease: pending` starts publication. The publisher waits up to approximately 40 minutes for `native-fidelity` push CI on that exact merge commit, then automatically creates the `vX.Y.Z` tag and GitHub Release. Ordinary PR merges skip the publication job. The tag stays on the selected merge commit even if `main` advances.

The workflow must be merged into the default branch first, and the caller needs repository write access. The same preparation can be started through **Actions → Release → Run workflow**, selecting `main`. See [GitHub's manual workflow guide](https://docs.github.com/en/actions/how-tos/manage-workflow-runs/manually-run-a-workflow).

If CI encounters a transient failure or the wait expires, rerun CI for that same release merge commit, then rerun the failed publication job in the Release workflow. A code defect requires a corrected, reviewed release candidate; successful CI on a later fix commit cannot qualify the original merge commit. Matching tags/releases are reused, and release lifecycle labels are repaired after partial failures. The publisher refuses unmerged or ordinary PRs, unsuccessful merge-commit CI, conflicting tags, missing notes, and malformed versions. A local test result or successful PR CI alone does not qualify. Do not move or reuse a published version tag.

[Release Please](https://github.com/googleapis/release-please) prepares one release PR for the workspace, including `spars-nlp` in `crates/spars/`. It updates the member manifests, project entries in the shared root lockfile and the independent consumer lockfile, the release manifest, and `CHANGELOG.md`. The same release version is applied to the installer and Node Rust manifests, the Node npm manifest, and their lockfile entries. The simple release strategy uses `version.txt` and explicit TOML/JSON updates because the pinned Rust strategy requires a package section in the root manifest. The root Cargo manifest remains a virtual workspace. Publication settings stay unchanged. Their changes are included in the shared changelog.

Cargo and Node package publication remain manual. After the GitHub Release exists, fetch its tag and check it out in a clean checkout; publish the appropriate package from that tagged source using its registry tooling. This workflow never invokes `cargo publish` or `npm publish`, and it does not prepare Node platform binaries. The Node npm manifest currently has `private: true`; npm publication requires a separate packaging and publication-readiness change before it can be uploaded.

### Repository setup

1. Create a GitHub App and install it only on this repository. Grant repository permissions **Contents: read and write**, **Pull requests: read and write**, and **Issues: read and write**. The last permission allows release lifecycle labels. No webhook subscription is required for this workflow.
2. Add its App ID as the Actions repository variable `RELEASE_APP_ID`. Generate an App private key and store it as the Actions repository secret `RELEASE_APP_PRIVATE_KEY`. The workflow mints a short-lived installation token scoped to this repository. App-authored PRs trigger normal CI; the default workflow token suppresses those automatic workflow triggers.
3. Enable squash merging and select **Pull request title** as the default squash commit title. Use squash merges for contributions so the validated title reaches the commit history. Disable other merge methods if enforcing this repository-wide is desired.
4. Once the title workflow is on `main` and has run, add **Conventional PR title** to the required branch checks alongside the existing acceptance checks. The workflow checks titles but cannot make itself required. Keep release-tooling required as well. The title check reads event data only and never checks out PR code despite using `pull_request_target`.
5. Keep action references pinned to full commit hashes. Dependabot maintains those pins. Do not grant the release App a branch-protection bypass; its release PR must pass the normal checks.

The preparation job requires the App variable and secret so its PR triggers CI. The publish job uses the built-in workflow token with Actions read access to verify CI and repository write access for the release and labels; it needs no additional App permissions or registry credentials. The title check and acceptance CI require no release credentials.

### First release and versioning

The release strategy starts at the configured initial version `0.1.0` when no prior release version is recorded. The bootstrap commit in the configuration excludes older, inconsistently named commits from automated notes. For the first release, review [the changelog](../CHANGELOG.md) against the compatibility inventory and include an overview of the existing supported features in its versioned entry.

Before `1.0.0`, fixes increment the patch version and features increment the minor version; breaking changes also increment the minor version. A feature and a breaking change can therefore produce the same version increment, so migration notes remain necessary. After `1.0.0`, breaking changes increment the major version. Even when manually requested, preparation needs releasable changes; documentation and maintenance commits alone do not normally produce a release PR. Review every proposed version; a deliberately chosen version can be requested using a `Release-As: 0.1.0` commit footer, replacing the example version as appropriate.

Releases use `vX.Y.Z` tags and the corresponding changelog entry as their body. This publisher accepts three-part versions only; prerelease suffixes need a separate tested extension. GitHub supplies source archives. Model weights, prebuilt binaries, and registry uploads are not attached or published by this workflow.

### Check release tooling locally

With Node.js 24 available, first run `npm --prefix tools ci --ignore-scripts --no-audit --no-fund`, then `node --test .github/scripts/*.test.cjs` from the checkout. These tests execute the title-check script and exercise release success, explicit preparation, release-PR merge triggers, rejected CI results, wait timeouts, conflicting tags, absent notes, API failures, and retries using a simulated GitHub API. They also exercise initial version generation and dependent lockfile updates with the pinned Release Please library. Keep that development dependency aligned with the library bundled in the pinned action when updating it. The release-tooling CI job runs the same command. They do not create remote releases or prove that an App installation is configured correctly.
