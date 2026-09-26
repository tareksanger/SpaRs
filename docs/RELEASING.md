# Release SpaRs

Run these commands from the repository root. `cargo release` prepares a GitHub release PR. After you merge it, GitHub creates the tag and release. You then run `cargo publish` to upload `spars-nlp` to crates.io. The [npm procedure](#publish-the-node-package) is a separate, manually started workflow.

## Prerequisites

- Complete the [developer setup](DEVELOPMENT.md), including Rust and Git.
- Install [GitHub CLI](https://cli.github.com/) and have write access to the repository.
- Have permission to publish `spars-nlp` on crates.io.
- Complete the [one-time repository setup](#one-time-repository-setup) below.
- Use a clone whose `origin` points to the repository being released, with GitHub CLI selecting that same repository.
- Start with a clean checkout and merge all intended changes into the default branch.

Authenticate if you have not already:

```bash
gh auth login
cargo login
```

`cargo login` prompts for your crates.io API token. Use a token with permission to publish this crate.

## 1. Prepare the release PR

```bash
cargo release
```

This uses the repository's current default branch on GitHub, including after a branch rename. It does not push local commits. To preview the command without starting a workflow, use `cargo release --dry-run`.

Once the workflow finishes, find the release PR:

```bash
gh pr list --label 'autorelease: pending'
```

Release Please chooses the version from commit messages: `fix:` bumps the patch version and `feat:` bumps the minor version. Breaking changes bump the minor version before 1.0 and the major version afterward. Documentation and maintenance commits alone may produce no release PR.

## 2. Review and merge the PR

Replace `123` with the release PR number:

```bash
release_pr=123
gh pr view "$release_pr" --web
gh pr checks "$release_pr" --watch
```

Review the version, changelog, and package changes. The version must not already exist on crates.io. If more changes land on the default branch, run `cargo release` again to refresh the PR and review it again.

When the checks pass and the PR is ready:

```bash
gh pr merge "$release_pr" --squash
```

GitHub waits for CI to pass on the exact merge commit, then creates its `vX.Y.Z` tag and GitHub Release. This does not upload anything to crates.io.

## 3. Fetch and check out the release tag

Wait for the new release to appear:

```bash
gh release list
```

Replace `v0.2.0` with the tag from that release:

```bash
release_tag=v0.2.0
git fetch origin tag "$release_tag"
git switch --detach "$release_tag"
git status --short
```

The status output should be empty. You are now on the source commit being published.

## 4. Check and publish the crate

Inspect the package contents and check that it builds before uploading:

```bash
cargo package --list
cargo publish --dry-run --locked
```

Confirm the archive includes the README and license notices, with no credentials, model assets, or generated reports. If either command fails, fix the problem before publishing; do not bypass it with `--allow-dirty`.

Publish:

```bash
cargo publish --locked
```

From the root, Cargo selects `spars-nlp` through the workspace's `default-members`. The installer and Node binding are not published. A version cannot be uploaded to crates.io twice.

## 5. Return to development

```bash
default_branch=$(gh repo view --json defaultBranchRef --jq '.defaultBranchRef.name')
git fetch origin "$default_branch:refs/remotes/origin/$default_branch"
git switch "$default_branch"
git pull --ff-only origin "$default_branch"
```

## If the GitHub release fails

Check **Actions → Release**. For a transient CI failure or timeout, rerun CI for the release PR's merge commit, then rerun the failed Release job. CI passing on a later commit does not qualify the original release commit. A code defect needs a corrected release candidate; do not move a published tag.

## One-time repository setup

1. Create a GitHub App, install it on this repository, and grant **Contents**, **Pull requests**, and **Issues** read/write permissions. No webhook is needed.
2. Set the Actions repository variable `RELEASE_APP_ID` to the App ID and the repository secret `RELEASE_APP_PRIVATE_KEY` to its private key. The App lets the generated release PR trigger CI.
3. Enable squash merging with the PR title as the default commit title. Require **Conventional PR title** and the acceptance CI checks, including `release-tooling`, in branch protection. Do not give the release App a bypass.
4. Ensure the release workflow is merged into the default branch before running `cargo release`.

The GitHub workflow does not need your crates.io token. npm uses the separate setup below.

## Publish the Node package

The `npm release` workflow builds Linux x64 and ARM64 (glibc, Ubuntu 24.04 build baseline) and macOS ARM64 binaries with Node.js 24. Windows, Linux musl, and Intel macOS prebuilds are not included. Older glibc versions than the build runner are not guaranteed. Every release tests the packed API, CLI, TypeScript declarations, and native inference on all three build platforms before publication. Model downloads remain explicit and weights are not bundled. Pull requests run the same build, assembly, and installation checks against their merge commit without requiring a release tag. Publication runs only through an explicit manual dispatch.

### One-time npm setup

1. Obtain publishing access to the `@spars` npm scope. If you use another scope, change the Node package name, lockfile name, package README, and package-install tests before releasing. The platform names are derived from the main name.
2. Create a GitHub environment named `npm`, restrict it to the repository default branch, and optionally require a reviewer before publication.
3. Merge the packaging workflow and scripts into the default branch and wait for its CI to pass. The workflow builds its exact default-branch commit, which must be the tagged commit or a descendant of it and keep the same Node package version. This allows packaging an existing release whose tag predates the npm tooling without moving that tag. The npm package includes all source changes in the workflow commit, including changes made after the tag; its source commit can therefore differ from the tagged Rust release. Keep `cargo release` as the version authority; do not run `npm version` separately.
4. For the first version, run the build-only procedure below. After all three smoke jobs pass, download its `npm-tarballs` artifact and publish those exact tarballs locally with `npm login` and the commands below. This creates all four packages before configuring their trusted publishers.

For local bootstrap publication, install Node.js 24 or newer with npm, and ensure `tar` is available on `PATH`. Replace `123456` with the successful build-only workflow run ID and use a new, empty download directory:

```bash
npm_run=123456
gh run download "$npm_run" --name npm-tarballs --dir target/npm-bootstrap
npm login
cargo publish-npm --artifacts target/npm-bootstrap
```

Configure trusted publishing in npm settings for **each** of `@spars/node`, `@spars/node-linux-x64-gnu`, `@spars/node-linux-arm64-gnu`, and `@spars/node-darwin-arm64`: select GitHub Actions, owner `tareksanger`, repository `SpaRs`, workflow `npm-release.yml`, and environment `npm`. Enable direct `npm publish` permission. Substitute the actual owner/repository if this is a fork. No `NPM_TOKEN` secret is required. The workflow uses Node.js 24 with npm 11.5.1 or newer; see [npm trusted publishing](https://docs.npmjs.com/trusted-publishers/). Local bootstrap publication does not provide GitHub Actions provenance. Prepare a new release version when adding a platform because the main package manifest changes. Bootstrap all four packages from that version’s successful manual build-only run, then configure the new platform’s trusted publisher. The local publisher verifies and skips already published identical tarballs.

### Build and inspect a release

Prepare and merge the release PR as above, then wait for its GitHub release. From the repository root, supply the released tag:

```bash
cargo publish-npm v0.3.0 --build-only
gh run list --workflow npm-release.yml
```

`cargo publish-npm TAG` dispatches GitHub Actions using the remote default branch; it does not push local changes or wait for workflow completion. Add `--dry-run` to either the tag or `--artifacts` form to print the command without contacting GitHub or npm. The artifact directory is relative to the repository root, and must contain the four tarballs from a successful manual build-only workflow, not locally assembled test fixtures.

The workflow checks that the tag still points to its recorded release commit, then builds the exact commit selected when the workflow was started. That commit must be the tagged commit or a descendant of it, retain its Node version, and have successful default-branch push CI. It rejects draft releases, prereleases, diverged history, and mismatched Node versions. Wait for CI to finish before starting the npm workflow. In Actions, inspect all three smoke jobs and download `npm-tarballs`. Four tarballs contain the main package and its three native dependencies, including license notices. Nothing is uploaded to npm in build-only mode. Artifacts expire after 14 days; retain the original tarballs if publication or recovery will happen later.

### Publish later versions

After all four trusted publishers are configured, prepare and merge a new release PR, wait for its GitHub release, then request publication explicitly. For example, if the new release is `v0.3.1`:

```bash
cargo publish-npm v0.3.1
```

This run rebuilds and retests the release before publishing its own tested tarballs. It publishes all platform packages before the main package. All four packages use the release version. The source manifest allows public publication. Use `cargo publish-npm` for releases: it uses the workflow or its complete tarball set, including native dependencies and notices. Running `npm publish` directly inside `bindings/node` does not assemble that release set. There are no publication lifecycle hooks.

If a publish job stops after uploading one package, keep that run's exact tarballs. Rerun only the failed publish job so it reuses that run's tested artifact. The publisher skips existing versions only when npm reports the exact SHA-512 integrity of the retained tarball, and stops before uploading anything if an existing version has different bytes. Do not restart all jobs to rebuild a partially published version. Inspect `npm view PACKAGE@VERSION dist.integrity`, compare it with the retained tarball's SHA-512 integrity, then publish only the missing platform packages and finally the main package using the bootstrap commands with your retained files. Do not upload different bytes under an existing version.

After publication, check the registry and install from npm on each supported platform:

```bash
npm view @spars/node@0.3.1 optionalDependencies
npm view @spars/node-linux-x64-gnu@0.3.1 version
npm view @spars/node-linux-arm64-gnu@0.3.1 version
npm view @spars/node-darwin-arm64@0.3.1 version
npm install @spars/node@0.3.1
npx spars download en_core_web_sm
```

The workflow verifies local tarball installation before publishing; these registry checks remain a separate post-publication step. Prebuilt package availability is established by a successful registry publication, not by the existence of this workflow.
