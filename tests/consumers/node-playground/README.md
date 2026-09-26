# Node playground

An independent TypeScript consumer of the local `@spars/node` package. It loads a model once and prints tokens, grammatical annotations, named entities, sentences, and noun chunks for text you enter. Processing runs in native Rust, offline. This is a terminal playground, not a browser application.

## Setup

Use Node.js 24 or newer. First build the native addon following the [Node guide](../../../docs/NODE.md#build-and-install-a-model); Rust is required for that build. From the repository root, install this project's dependencies:

```sh
npm --prefix tests/consumers/node-playground install --ignore-scripts --no-audit --no-fund
```

The default model is the local `assets/en_core_web_sm-3.8.0` export, resolved relative to this script. If that export is absent, explicitly install the small model and select it by name:

```sh
export SPARS_MODEL_DIR="$PWD/target/models"
node bindings/node/bin/spars.mjs download en_core_web_sm
SPARS_MODEL=en_core_web_sm npm --prefix tests/consumers/node-playground start
```

Installation uses the shared model cache or `SPARS_MODEL_DIR` if set. The command above makes the store path absolute so downloading and loading use the same directory. You can also set `SPARS_MODEL` to an explicit export directory; use an absolute path because npm starts the script from this project's directory. Loading does not download a missing model.

## Run

With the default export available, run these commands from the repository root:

```sh
npm --prefix tests/consumers/node-playground run typecheck
npm --prefix tests/consumers/node-playground start
```

The playground analyzes an initial sentence, then accepts one line of text at a time. Type `:quit` or press Ctrl-D to exit. Blank input is processed as an empty document. For a single input followed by exit:

```sh
npm --prefix tests/consumers/node-playground start -- "Apple opened an office in London."
```

Edit `src/main.mts` to experiment with the binding. `pos` is a part-of-speech label, `lemma` is a word's base form, and `head` is the index of its dependency parent. Entity labels such as `ORG` and `GPE` identify organizations and political places. Predictions vary with the model; the printed elapsed time includes the Node binding call and is not a benchmark. The supported model and API limits are described in the [Node guide](../../../docs/NODE.md).

The local dependency links to `bindings/node`; rebuild that package when changing its Rust code. This project is private and does not publish or download models during startup.
