# Prepare a release

Making the source repository public, publishing a Cargo or npm package, and distributing compiled binaries or model exports are separate actions. Each needs an explicit maintainer decision. The Node package and native installer are marked private or non-publishable; their tested installation route is a source build.

## Check the exact commit

Follow the [developer setup](DEVELOPMENT.md), then run:

```sh
.venv/bin/python tools/verify.py
git status --short
```

Verify the final commit in GitHub Actions: both Linux and macOS reference jobs and the separate native installation job must pass. Review the reference jobs' report artifacts and the native installation job's logs when investigating failures. A local pass or a pass from an earlier commit does not establish release verification.

Review the README, compatibility inventory, model versions, and minimum toolchain versions. Confirm that examples execute and the Cargo package dry run succeeds. Keep generated binaries, models, reports, local environment files, and credentials out of the source distribution.

## Review what becomes public

Inspect the current tree and all history reachable through branches and tags for credentials, private input, personal filesystem paths, and unwanted artifacts. Review GitHub Actions logs and artifacts separately. Deleting a file from the latest commit does not remove historical copies. Rotate exposed credentials if any are found; a history rewrite alone cannot revoke them.

Any history rewrite must be coordinated with collaborators because it changes commit IDs and requires replacement of remote history. Keep a private backup and verify the resulting source tree before replacing shared refs. Enable private vulnerability reporting in the repository's security settings and check that the route in [SECURITY.md](../SECURITY.md) works before inviting reports. Configure required CI checks in branch protection or repository rules. Require pull requests, block force pushes and branch deletion, and keep the bypass list empty unless a documented operational need requires an exception. For a sole maintainer, zero required approvals permits their own pull requests to merge after CI passes. Review collaborators, deploy keys, installed applications, and Actions permissions before publication.

While the repository is public, enable secret scanning and push protection where available, and require approval before running workflows from outside contributors. Some security settings are unavailable while the repository is private; verify them again immediately after an explicitly approved visibility change. Keep workflow tokens read-only, disable persisted checkout credentials, and pin actions to full commit hashes. Dependabot proposes action, Cargo, and npm updates; audit the pinned Python reference requirements separately when preparing a release.

## Preserve applicable notices

The source includes the [project license](../LICENSE) and [third-party attribution](../THIRD_PARTY_NOTICES.md). Keep notices for translated code and copied resources. A model or language-resource update needs its own provenance and license review; do not assume all official models use identical terms.

Before distributing a compiled artifact, review the exact dependencies linked into it and include their required license texts. Audit the corresponding lockfile for published security advisories. Repeat that review for each Rust library, installer, Node addon, and target platform being distributed. A source-repository license or dependency lockfile alone does not fulfill every binary-distribution notice requirement.

Before registry publication, confirm package-name ownership, package contents, platform support, and consumer installation from the packaged artifact. Node publication additionally needs a deliberate platform-binary packaging strategy and its own clean-consumer tests. Those checks are separate from building the addon in this repository.
