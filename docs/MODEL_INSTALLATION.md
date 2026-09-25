# Install an official model without Python

The separate `spars-model` tool downloads and converts the official `en_core_web_sm`, `en_core_web_md`, and `en_core_web_lg` 3.8.0 packages entirely natively. It requires a Rust toolchain to build from this checkout. Installation does not require Python, Node, WASM, `curl`, or a model service. Applications still depend only on the `spars-nlp` Cargo package, imported as `spars`; its builds and inference do not download anything.

## Environment and simple downloads

Set `SPARS_MODEL_DIR` in the environment used by both the downloader and application:

```sh
export SPARS_MODEL_DIR="./target/models"
cargo install --path installer --bin spars --locked
spars download en_core_web_lg
```

The Node package supplies the same `spars download` command and a `downloadModel` API without requiring a separate CLI build; see [Node setup](NODE.md). The command accepts `--path DIR`, `--archive WHEEL`, and `--version VERSION`. A path override applies only to that operation. Neither downloader edits shell files, `.env`, or project JSON. Environment discovery and operating-system cache defaults are shared by Rust and Node. The root Rust library remains offline and has no networking dependency.

Rust applications use `spars_model::download_model("en_core_web_lg", spars_model::DownloadOptions::default())` from the separate `spars-model` crate (available at `installer/` in this checkout). Set `DownloadOptions.path`, `.archive`, or `.version` for explicit overrides. Inference then uses `spars::Model::load("en_core_web_lg")`. Applications managing multiple roots can use `spars::ModelStore::new(path).load(name)` without changing environment variables.

A completed download records the selected version in a small plain-text record under the model root's `.spars` directory. This is installation metadata, not project configuration. Loading reads that bounded record, validates the selected directory and model identity, and never picks an arbitrary version by scanning. Repeating a download verifies and selects that installation; older versioned directories remain intact. The legacy `install` command below still returns a directory directly and does not select it for name-based loading.

## Download and install

Run from the repository root:

```sh
cargo build --locked --release --manifest-path installer/Cargo.toml
model_dir=$(installer/target/release/spars-model install --version 3.8.0 --root target/models)
cargo run --release --example analyze -- "$model_dir" 'Alice visited New York.'
```

Installation prints the directory that you pass to `Model::load`. Diagnostic errors go to standard error and return a failing exit code. The model name defaults to `en_core_web_md`; `--model en_core_web_sm` or `--model en_core_web_lg` selects a different size. The version and installation root are required. These commands install local files; they do not deploy or publish anything.

To install an official package you already have, use `--archive`. After Rust dependencies have been fetched and the tool built, this path works without network access:

```sh
model_dir=$(installer/target/release/spars-model install --version 3.8.0 --root target/models --archive assets/en_core_web_md-3.8.0-py3-none-any.whl)
installer/target/release/spars-model verify "$model_dir"
installer/target/release/spars-model list --root target/models
```

Keep the chosen directory in your application's configuration. Existing documentation examples use the Python reference export at `assets/en_core_web_md-3.8.0`; replace that path with the installed directory in your application. Both have the same model data. Use a dedicated installation root: `list` checks every non-hidden entry and returns an error for damaged or unsupported installations.

## Versions and updates

`catalog` lists releases supported by the installed tool:

```sh
installer/target/release/spars-model catalog
```

The catalog contains `en_core_web_sm`, `en_core_web_md`, and `en_core_web_lg` 3.8.0. The installation directory includes the model version, native format version, conversion-recipe revision, language-resource revision, and archive checksum. Installing an already-present identity verifies and reuses it. Failed conversion leaves prior installations intact. An abandoned staging directory is recovered on the next attempt; concurrent installers cannot write through the same root lock.

There is no implicit “latest” version. Legacy `install` leaves name-based selections unchanged; explicit `download` selects that installation for subsequent name-based loads. Already loaded models and older directories remain unchanged. A future supported release will install alongside the old one; applications will switch by explicitly selecting its directory and can roll back by selecting the retained old directory. Future catalogs must retain supported older recipes to verify those installations. Other model families, automatic update discovery, and model-removal commands are not implemented. New architectures still require native implementation and their own reference tests.

## What is converted and checked

The installer checks the complete official archive checksum before decoding. It reads all 67 (`sm`) or 69 (`md`/`lg`) tensor arrays, the vector-key mapping, and lemma tables from the package. A reproducible, embedded recipe supplies the reviewed architecture configuration, tokenizer rules, normalization and language resources, and source notices. This is a converter for a pinned package, not a general reader for arbitrary spaCy or Thinc models. Recipes contain no pretrained tensor arrays. Download, entry, expanded archive, and installed-file limits come from each release in [the acquisition catalog](../models/catalog.json), so larger models do not require globally relaxed limits.

Conversion must reproduce the reference weights byte for byte, and the lookup sections must match their pinned checksums. The native loader validates the complete model before the directory becomes available. Installation writes to a temporary directory and renames the finished result into place. `verify` checks the complete expected file inventory against the receipt, the embedded resource and tensor digests, and the pinned manifest data; it does not trust a receipt alone. Archive links, unsafe paths, duplicate destinations, unsupported formats, and oversized entries return errors.

The tool preserves model, spaCy, Thinc, and Unicode notices. Python's license is not included because the tool does not redistribute Python. Resource revision 2 removes the redundant notice while leaving all model data unchanged. Existing revision 1 installations remain verifiable against their original checksums; installing revision 2 creates a separate directory. The installer retains only the old notice's checksum for that compatibility check, not its text. Recipe generation uses the official Python reference during development; consumer installation does not. To regenerate resources for review into a new directory:

```sh
.venv/bin/python tools/installer_recipe.py --out target/recipe-review
```

Add `--model en_core_web_sm` or `--model en_core_web_lg` to generate their recipes into a separate empty directory. Their source records are checked against the checksum-pinned official wheels.

Recipe regeneration checks shared implementation sources against `reference/source-lock.json`. Medium-model files, licenses, versions, and release URLs also match that pinned capture; small and large model records match their catalog-pinned official wheels. Four Python installation bookkeeping files (`INSTALLER`, `REQUESTED`, `direct_url.json`, and `uv_cache.json`) may differ between machines and are excluded from that comparison. One generated C source has a separately verified Linux hash because its build-directory comments differ; the exact allowed hashes are recorded in [source provenance](../reference/README.md). Unknown source changes still fail. The installer keeps the original provenance bytes and installation identity.

## Verification

The regular acceptance script runs the installer tests, complete manifest and weight comparisons against the Python exporter, and local-package installation followed by the independent Rust consumer with an empty `PATH`. The native installation CI job separately downloads, converts, verifies, and uses the model without a Python setup step. Missing official assets fail model-dependent tests; they do not silently skip in acceptance.

```sh
cargo test --release --offline --manifest-path installer/Cargo.toml -- --include-ignored
.venv/bin/python tools/check_installer.py
```

Run these after the reference setup and consumer build in the [README](../README.md#reproduce-verification). Failure tests cover archive corruption, array formats, checksum mismatches, HTTP failures and timeouts, damaged receipts, locking, abandoned staging data, and repeated installation. Exact converted data equality links the installed assets to the existing pipeline reference suites. See the [implementation inventory](PROGRESS.md) for remaining compatibility work.
