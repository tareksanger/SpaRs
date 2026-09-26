# SpaRs for Node.js

Native Rust NLP using official spaCy pretrained weights. Node.js 24 or newer is required. Release packages target Linux x64 with glibc and macOS ARM64; other platforms require a source build and separate verification. See the [Node guide](https://github.com/tareksanger/SpaRs/blob/main/docs/NODE.md) for the typed API and limitations.

Model loading and NLP inference (`loadModel`, `process`, and `processBatch`) run asynchronously on background worker threads, keeping CPU-heavy inference off the event loop. Input copying and result creation still use the JavaScript thread. Inference admission defaults to two active jobs and 32 queued jobs across models in each JavaScript isolate; excess calls reject with `SPARS_BUSY`. Inputs default to at most 32,768 UTF-16 units per text, 128 texts per batch, and 65,536 units per batch; excess calls reject with `SPARS_INPUT_LIMIT`. See the Node guide for `configureExecution`, `configureInputLimits`, and worker-pool limits.

After installing a published version:

```sh
npm install @spars/node
npx spars download en_core_web_sm
```


Use the executable examples in the [Node guide](https://github.com/tareksanger/SpaRs/blob/main/docs/NODE.md) to load a model and inspect token annotations.

Model acquisition is explicit. Loading and inference run offline. Set `SPARS_MODEL_DIR` to share an installation directory across applications. The verified model releases are `en_core_web_sm`, `en_core_web_md`, and `en_core_web_lg` 3.8.0; this does not establish full spaCy compatibility. Model assets retain their own licenses and are acquired separately.

The package includes the project MIT license and upstream notices. Platform packages also include Cargo dependency license texts. SpaRs is independent of Explosion.
