#!/bin/sh
set -eu
case "${1:-}" in
  build|smoke) ;;
  *) echo "Expected build or smoke" >&2; exit 1 ;;
esac
npm --prefix bindings/node ci --ignore-scripts --no-audit --no-fund
case "${1:-}" in
  build)
    cargo fetch --locked
    cargo test --locked --release -p spars-model install::tests
    npm --prefix bindings/node run build
    npm --prefix bindings/node run typecheck
    node --test bindings/node/tests/release*.test.mts
    node bindings/node/scripts/release.mts collect "$RELEASE_TARGET" "target/npm-input/$RELEASE_TARGET" "$RELEASE_SHA"
    ;;
  smoke)
    cargo build --locked --release -p spars-model
    model_dir=$(target/release/spars-model install --model en_core_web_sm --version 3.8.0 --root target/models)
    node bindings/node/scripts/release-smoke.mts target/npm-tarballs "$model_dir"
    ;;
  *) echo "Expected build or smoke" >&2; exit 1 ;;
esac
