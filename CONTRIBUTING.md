# Contribute to SpaRs

Start with the [developer guide](docs/DEVELOPMENT.md) for setup and working Rust examples. Follow the [quality process](docs/QUALITY.md) when adding a feature or fixing a bug.

Keep each change focused on a clear behavior. Use official spaCy as the reference, add a test that can expose the problem, and preserve existing expected outputs. Describe what changed, how it was checked, and any remaining limitations.

The [compatibility inventory](docs/COMPATIBILITY.md) lists supported and missing features. A feature is verified only for the configurations and cases covered by its evidence.

Report bugs and propose features in the [issue tracker](https://github.com/tareksanger/SpaRs/issues). Include the commit or version, model version, a small input, expected and actual results, and a reproducible command. Remove personal data and machine-specific paths. Follow [SECURITY.md](SECURITY.md) for vulnerabilities and [the release checklist](docs/RELEASING.md) for distribution changes.

Submit changes through a pull request from a branch or fork. The default branch requires its configured CI checks and resolved review conversations before merging. Maintainers merge accepted contributions; opening a pull request does not grant repository write access.

Keep discussions respectful and focused on the work. Critique ideas and code, avoid personal attacks, and do not share someone else's private information.

## Pull request titles

Use `type(optional-scope)!: description`. The scope and breaking-change marker (`!`) are optional. Allowed types are `feat`, `fix`, `docs`, `style`, `refactor`, `perf`, `test`, `build`, `ci`, `chore`, and `revert`. Use a short scope such as `models`, `tokenizer`, or `deps`; spaces are not allowed inside the scope.

Examples: `feat(models): support another pipeline`, `fix: preserve trailing whitespace`, and `feat!: change the loading API`. The Conventional PR title check runs when a PR opens, changes title, receives commits, reopens, or becomes ready for review, including contributions from forks.

Squash-merge contributions and use the validated PR title as the squash commit title. When a maintainer runs `gh workflow run release.yml --ref main`, Release Please reads those commits to prepare versions and changelogs. Normal PRs and merges do not prepare a release. Merging the prepared release PR starts GitHub publication after its merge commit passes CI. Explain breaking changes and migration steps in the PR body and review their release notes before merging the release PR. See [releasing](docs/RELEASING.md) for versioning and automation setup.
