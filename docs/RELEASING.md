# Release spars-nlp

Run these commands from the repository root. `cargo release` prepares a GitHub release PR. After you merge it, GitHub creates the tag and release. You then run `cargo publish` to upload `spars-nlp` to crates.io.

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

Registry credentials stay local; the GitHub workflow does not need your crates.io token.
