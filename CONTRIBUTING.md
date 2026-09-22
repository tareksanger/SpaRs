# Contribute to SpaRs

Start with the [developer guide](docs/DEVELOPMENT.md) for setup and working Rust examples. Follow the [quality process](docs/QUALITY.md) when adding a feature or fixing a bug.

Keep each change focused on a clear behavior. Use official spaCy as the reference, add a test that can expose the problem, and preserve existing expected outputs. Describe what changed, how it was checked, and any remaining limitations.

The [compatibility inventory](docs/COMPATIBILITY.md) lists supported and missing features. A feature is verified only for the configurations and cases covered by its evidence.

Report bugs and propose features in the [issue tracker](https://github.com/tareksanger/SpaRs/issues). Include the commit or version, model version, a small input, expected and actual results, and a reproducible command. Remove personal data and machine-specific paths. Follow [SECURITY.md](SECURITY.md) for vulnerabilities and [the release checklist](docs/RELEASING.md) for distribution changes.

Submit changes through a pull request from a branch or fork. The default branch requires all three CI checks and resolved review conversations before merging. Maintainers merge accepted contributions; opening a pull request does not grant repository write access.

Keep discussions respectful and focused on the work. Critique ideas and code, avoid personal attacks, and do not share someone else's private information.
