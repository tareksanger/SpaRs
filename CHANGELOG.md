# Changelog

## [0.2.0](https://github.com/tareksanger/SpaRs/compare/v0.1.0...v0.2.0) (2026-09-25)


### Features

* add cargo release command for workflow preparation ([118f806](https://github.com/tareksanger/SpaRs/commit/118f8068bb8c58e7cf1102347d64c4afdc403db5))
* add native model downloads and name-based loading ([#29](https://github.com/tareksanger/SpaRs/issues/29)) ([ed1b516](https://github.com/tareksanger/SpaRs/commit/ed1b516ce6997d5137b05b0068ca982f1d8223f2))


### Bug Fixes

* follow the default branch throughout releases ([ff03243](https://github.com/tareksanger/SpaRs/commit/ff0324369b9a7bf4534bfe87c20bc753c5a708b7))
* stale lock file ([366e938](https://github.com/tareksanger/SpaRs/commit/366e93854bcb0b491f37c91476b54a5a5dc43c9a))

## 0.1.0 (2026-09-25)


### Features

* **ci:** automate reviewed source releases ([1e064e1](https://github.com/tareksanger/SpaRs/commit/1e064e140f96522fdad5f04d6a8731b9ef59f78c))

## Changelog

Release entries are prepared in release pull requests and published after acceptance CI passes for the merged commit.

## Initial release overview

The first release PR must include this overview in its versioned entry before merging:

- Native Rust tokenization and inference for the official English `sm`, `md`, and `lg` 3.8.0 models, referenced against spaCy 3.8.14 and Thinc 8.3.13.
- Explicit native model installation and separate Node.js bindings built from source.
- Model assets acquired separately; no model weights bundled in the Rust crate.
- Experimental API; inference supports only the three English pipelines listed above. See [compatibility](docs/COMPATIBILITY.md) for supported behavior and remaining limitations.
