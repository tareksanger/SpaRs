# Troubleshooting

This page lists the problems people meet first, with the message you will see and how to fix it. Most of them are about finding the model.

## How SpaRs finds a model

When you load a model by name, such as `en_core_web_sm`, SpaRs looks in one folder, called the model store. It never searches your disk and never downloads anything while loading.

The model store is:

1. The folder named by the `SPARS_MODEL_DIR` environment variable, when it is set. An environment variable is a named setting that your terminal or system passes to the programs it starts.
2. Otherwise a cache folder for your user: `Library/Caches/spars/models` under your home folder on macOS, `spars/models` under `LOCALAPPDATA` on Windows, and `spars/models` under `XDG_CACHE_HOME` (or `.cache` in your home folder) on other systems.

The download command and your program must agree on this folder. A mismatch here is a common cause of "not installed" errors.

## "Model … is not installed in …"

You asked for a model by name and it is not in the model store. The message names the folder that was searched and the command that fixes it. The folder is shown as a full path on your machine; it is shortened here:

```text
Model en_core_web_xx is not installed in target/models. Run `spars download en_core_web_xx` with the same SPARS_MODEL_DIR (or --path).
```

In Rust this is `spars::Error::Io` holding an error of kind `NotFound`. In Node.js the error has the code `SPARS_IO`. This example asks for a model that does not exist and checks the error:

::: code-group

```rust
use spars::{Error, Model};
use std::io::ErrorKind;

fn main() {
    // Ask for a model that is not installed.
    let result = Model::load("en_core_web_xx");

    // Loading fails with an I/O error of kind `NotFound`. The message names the
    // download command to run.
    match result {
        Err(Error::Io(error)) => {
            assert_eq!(error.kind(), ErrorKind::NotFound);
            assert!(error.to_string().contains("spars download en_core_web_xx"));
        }
        _ => panic!("expected a missing-model error"),
    }
}
```

```typescript
import assert from 'node:assert/strict';
import { loadModel } from '@spars/node';

// Ask for a model that is not installed, and look at the error it rejects with.
await assert.rejects(loadModel('en_core_web_xx'), (error: unknown) => {
  // SpaRs errors are `Error` objects that carry a short text `code`.
  assert.ok(error instanceof Error && 'code' in error);
  assert.equal(error.code, 'SPARS_IO');

  // The message names the download command to run.
  assert.ok(error.message.includes('spars download en_core_web_xx'));
  return true;
});
```

:::

Check these in order:

- **Did the download finish?** Run the download command again. It checks an existing installation and reuses it.
- **Is `SPARS_MODEL_DIR` the same in both places?** It must be set to the same folder in the terminal where you download and in the environment where your program runs. If you set it in one terminal window only, a program started elsewhere does not see it.
- **Is `SPARS_MODEL_DIR` a relative path?** A relative path, such as `./models`, is read from the folder your program starts in, so it points somewhere else when you start the program from another folder. An absolute path, which starts from the top of the disk, avoids this.
- **Did you download with `--path`?** `--path` applies to that one command. To load from the same folder, set `SPARS_MODEL_DIR` to it, or pass the folder when loading: `loadModel(name, { path })` in Node.js or `ModelStore::new(path).load(name)` in Rust.
- **Did you use the older `spars-model install` command?** It creates a model folder but does not record the model under its name in the model store. Load that model by its folder path, or run `spars download` instead.

## "No such file or directory"

You passed a folder path, and the folder does not exist or is not a model folder (it has no `manifest.json` file). The words after `I/O:` come from your operating system, so they differ on Windows:

```text
I/O: No such file or directory (os error 2)
```

Check the path, and remember that a relative path is read from the folder your program starts in.

In Rust this also happens when a name contains a character that names cannot have. A model name uses only lowercase letters, digits and underscores, and starts with a letter. Rust treats anything else, such as `my-model`, as a folder path.

## "invalid model name"

In Node.js, a value without a slash or backslash is read as a name (apart from `.` and `..`, which are folders). If it is not a valid name, loading fails with the code `SPARS_INVALID_MODEL`:

```text
invalid model: invalid model name; use lowercase ASCII letters, digits and underscores
```

To load from a folder in Node.js, write it as a path, such as `./my-model`.

## A folder is read as a model name

In Rust, `Model::load("models")` looks for an installed model called `models`, even when a folder of that name is right there, because `models` is a valid model name. Write the folder as a path instead: `Model::load("./models")`.

## A value is missing

A lemma, tag, dependency, sentence list or entity list can be missing: `None` in Rust, `null` in Node.js. That usually means the stage that produces it did not run, for example because you asked the pipeline to stop early. It does not mean "none found": a document with no entities has an empty list, not a missing one. See [the pipeline](CONCEPTS.md#the-pipeline).

## Node.js rejects a call with `SPARS_BUSY`

SpaRs limits how many texts it works on at once. `SPARS_BUSY` means the text was fine but every place was taken. Wait and try again, as shown in [handle processing errors](NODE.md#handle-processing-errors), or raise the limits with `configureExecution`, described under [server resource limits](NODE.md#server-resource-limits).

## Still stuck

Search the [issue tracker](https://github.com/tareksanger/SpaRs/issues), or open an issue with the SpaRs version, the model name, the smallest text that shows the problem, and what you expected to happen.
