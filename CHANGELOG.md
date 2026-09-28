# Changelog

## [0.3.0](https://github.com/tareksanger/SpaRs/compare/v0.2.0...v0.3.0) (2026-09-28)


### ⚠ BREAKING CHANGES

* **node:** align processing defaults with spaCy ([#58](https://github.com/tareksanger/SpaRs/issues/58))

### Features

* add native exact-text PhraseMatcher with spaCy parity ([#59](https://github.com/tareksanger/SpaRs/issues/59)) ([843dca6](https://github.com/tareksanger/SpaRs/commit/843dca6f654bd45ba5eaf5f9001a618fe4a90735))
* **node:** add Intel macOS release packages ([#53](https://github.com/tareksanger/SpaRs/issues/53)) ([f044210](https://github.com/tareksanger/SpaRs/commit/f044210ef41c2f8e9b9d9d86004cb0d92aaf613f))
* **node:** add Linux ARM64 release packages ([#42](https://github.com/tareksanger/SpaRs/issues/42)) ([919d1e3](https://github.com/tareksanger/SpaRs/commit/919d1e35cf410437a90001b7e6bc642dcd5c1af3))
* **node:** add native Alpine musl release packages ([#56](https://github.com/tareksanger/SpaRs/issues/56)) ([c9c1a28](https://github.com/tareksanger/SpaRs/commit/c9c1a28657908ee95973bf082151403206ede45d))
* **node:** add Windows ARM64 release packages ([#55](https://github.com/tareksanger/SpaRs/issues/55)) ([28029ce](https://github.com/tareksanger/SpaRs/commit/28029ce1e808f6dc117c1480659f93d5301b6b84))
* **node:** add Windows x64 release packages ([#46](https://github.com/tareksanger/SpaRs/issues/46)) ([362d6b6](https://github.com/tareksanger/SpaRs/commit/362d6b6a08c2a5fa5b68fbe281a0bd17fcd884bd))
* **node:** align processing defaults with spaCy ([#58](https://github.com/tareksanger/SpaRs/issues/58)) ([9cdfb74](https://github.com/tareksanger/SpaRs/commit/9cdfb74c2cc0e04b8509eb64440bce58b6e4b4cd))
* **node:** bound active and queued inference jobs ([#43](https://github.com/tareksanger/SpaRs/issues/43)) ([ccfe95e](https://github.com/tareksanger/SpaRs/commit/ccfe95e5dd0cf96c3a19cb6e13e00dee2a1bf3e5))
* **node:** reject oversized inference inputs before native copies ([#44](https://github.com/tareksanger/SpaRs/issues/44)) ([12570e7](https://github.com/tareksanger/SpaRs/commit/12570e71d0bd1e6724ce9941a861f06d7dd38bf7))
* **node:** retain release assets and add Cargo recovery commands ([#57](https://github.com/tareksanger/SpaRs/issues/57)) ([28aed63](https://github.com/tareksanger/SpaRs/commit/28aed63264e9534a8953c9edde57a96ac6560d20))
* **release:** add cargo publish-npm command ([ce8009e](https://github.com/tareksanger/SpaRs/commit/ce8009e2d9548c188ba5025bd517dbe44409c093))
* **release:** add verified native npm publication workflow ([acb9c9e](https://github.com/tareksanger/SpaRs/commit/acb9c9e9b6fa297f7ec12c5e7e01ee75f7c107fc))


### Bug Fixes

* repair release tooling and isolate verification model stores ([02909c2](https://github.com/tareksanger/SpaRs/commit/02909c21c4ffd659aa17bc3885629cd48f7f6fa7))

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
