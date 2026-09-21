# Report a security issue

Use **Report a vulnerability** in the repository's [Security tab](https://github.com/tareksanger/SpaRs/security) when private reporting is available. If that option is unavailable, [open an issue](https://github.com/tareksanger/SpaRs/issues/new) asking for a private contact. Leave vulnerability details, exploit code, credentials, and personal data out of public issues.

Include the affected commit or version, operating system, model version, a small reproduction, and the expected security impact in the private report. Remove private text and model paths from examples. Ordinary parsing or annotation mistakes belong in the [issue tracker](https://github.com/tareksanger/SpaRs/issues).

Security fixes target the current `main` branch. Older revisions have no promised backport schedule. Checksums identify supported model assets; they do not make arbitrary model configurations safe or supported. Applications processing untrusted text should set input and concurrency limits appropriate to their deployment.
