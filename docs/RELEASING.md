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

## GitHub release automation

[Release Please](https://github.com/googleapis/release-please) prepares one release PR for the root `spars-nlp` crate. The PR updates `Cargo.toml`, the root lockfile, the root crate entries in the three dependent lockfiles, the release manifest, and `CHANGELOG.md`. The source-only installer and Node package retain their separate versions and non-publishable settings. Changes to those directories are still included in the repository release history.

The [release workflow](../.github/workflows/release.yml) runs after successful `native-fidelity` push CI on `main`. It publishes a release only when that exact tested commit merged a PR labeled `autorelease: pending`. Version and release notes are read from that commit; the tag targets that commit even if `main` advances. Ordinary commits prepare or refresh the release PR without publishing a release. Merging a release PR authorizes its GitHub source release once the merged commit passes CI. Registry publication and compiled artifacts remain separate maintainer actions.

The publisher refuses conflicting tags, missing notes, and malformed versions. Rerun the failed Release workflow after resolving its error; rerunning acceptance for the same commit also retries publication. On retry, an existing matching tag and release are reused. It marks a completed release PR `autorelease: tagged`. A failed publication leaves the pending label for retry. Do not reuse or move a published version tag.

### Repository setup

1. Create a GitHub App and install it only on this repository. Grant repository permissions **Contents: read and write**, **Pull requests: read and write**, and **Issues: read and write**. The last permission allows release lifecycle labels. No webhook subscription is required for this workflow.
2. Add its App ID as the Actions repository variable `RELEASE_APP_ID`. Generate an App private key and store it as the Actions repository secret `RELEASE_APP_PRIVATE_KEY`. The workflow mints a short-lived installation token scoped to this repository. App-authored PRs trigger normal CI; the default workflow token suppresses those automatic workflow triggers.
3. Enable squash merging and select **Pull request title** as the default squash commit title. Use squash merges for contributions so the validated title reaches the commit history. Disable other merge methods if enforcing this repository-wide is desired.
4. Once the title workflow is on `main` and has run, add **Conventional PR title** to the required branch checks alongside the existing acceptance checks. The workflow checks titles but cannot make itself required. Keep release-tooling required as well. The title check reads event data only and never checks out PR code despite using `pull_request_target`.
5. Keep action references pinned to full commit hashes. Dependabot maintains those pins. Do not grant the release App a branch-protection bypass; its release PR must pass the normal checks.

Without the App variable and secret, the release job fails at token creation and creates no release. The title check and acceptance CI require no release credentials.

### First release and versioning

The empty release manifest lets the Rust strategy propose its initial `0.1.0` release. The bootstrap commit in the configuration excludes older, inconsistently named commits from automated notes. Merge the automation setup with a title such as `feat(ci): automate reviewed source releases` to trigger the initial release PR. Copy the initial overview in [the changelog](../CHANGELOG.md) into that PR's versioned `0.1.0` entry and remove the unversioned overview before merging. Review the entry against the current compatibility inventory. No historical release is implied by the overview.

Before `1.0.0`, fixes increment the patch version and features increment the minor version; breaking changes also increment the minor version. A feature and a breaking change can therefore produce the same version increment, so migration notes remain necessary. After `1.0.0`, breaking changes increment the major version. Documentation and maintenance commits alone do not normally open a release PR. Review every proposed version; a deliberately chosen version can be requested using a `Release-As: 0.1.0` commit footer, replacing the example version as appropriate.

Releases use `vX.Y.Z` tags and the corresponding changelog entry as their body. This publisher accepts three-part versions only; prerelease suffixes need a separate tested extension. GitHub supplies source archives. Model weights, prebuilt binaries, and registry uploads are not attached or published by this workflow.

### Check release tooling locally

With Node.js 24 available, first run `npm --prefix tools ci --ignore-scripts --no-audit --no-fund`, then `node --test .github/scripts/*.test.cjs` from the checkout. These tests execute the title-check script and exercise release success, rejected CI events, conflicting tags, absent notes, API failures, and retries using a simulated GitHub API. They also exercise initial version generation and dependent lockfile updates with the pinned Release Please library. Keep that development dependency aligned with the library bundled in the pinned action when updating it. The release-tooling CI job runs the same command. They do not create remote releases or prove that an App installation is configured correctly.
