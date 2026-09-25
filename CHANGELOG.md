# Changelog

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
