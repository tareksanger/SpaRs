# Choose a model

SpaRs runs three English models published by the [spaCy](https://spacy.io) team, all at version 3.8.0. They are the same kind of model in three sizes. This page helps you pick one. If you are not sure, start with `en_core_web_sm`. The commands below use the `spars` tool from the [quick setup](index.md#quick-setup).

## The three models side by side

| | `en_core_web_sm` | `en_core_web_md` | `en_core_web_lg` |
|---|---|---|---|
| Download size | 12 MB | 32 MB | 382 MB |
| Tokens, lemmas, parts of speech | yes | yes | yes |
| Grammar, sentences, noun chunks | yes | yes | yes |
| Entities | yes | yes | yes |
| Stored word vectors | none | 20,000 vectors shared by 684,830 words | 342,918 vectors shared by 684,830 words |
| Numbers in each token or document vector | 96 | 300 | 300 |

The download sizes are the sizes of the official spaCy packages that SpaRs downloads and converts.

All three run the same pipeline stages, described in [core concepts](CONCEPTS.md#the-pipeline). The difference is the word vectors.

## What word vectors are

A **word vector** is a list of numbers that stands for the meaning of a word. Words with related meanings, such as "dog" and "puppy", get lists that are close to each other. That lets a program compare meanings with arithmetic: SpaRs can give a **similarity** score for two words, two spans or two documents.

- `en_core_web_md` and `en_core_web_lg` store word vectors. These are called static vectors, because a word always gets the same vector wherever it appears.
- Both know the same 684,830 words. Neither keeps a separate vector for every word; `en_core_web_md` keeps far fewer, so more of its words share one. That is why it is much smaller.
- `en_core_web_sm` stores no word vectors. It can still produce a vector for a token or a document, worked out from the surrounding words while it reads the text. These are called contextual vectors, and each one has 96 numbers.

## Which one to pick

- **Pick `en_core_web_sm`** to learn SpaRs, or when you need tokens, grammar and entities and do not compare meanings. It is the fastest to download.
- **Pick `en_core_web_md`** when you want to compare the meaning of words or texts and want a small download.
- **Pick `en_core_web_lg`** when you compare meanings and want more distinct vectors, so fewer words share one.

SpaRs makes no claim about how accurate each model is. The spaCy team publishes accuracy figures for every model on the [spaCy models page](https://spacy.io/models/en). To measure speed and memory on your own machine, follow the [performance guide](PERFORMANCE.md).

## Use the model you picked

Download a model by name, then load it by the same name:

```sh
spars download en_core_web_md
```

In Node.js projects the command is `npx spars download en_core_web_md`. Then pass `"en_core_web_md"` to `Model::load` in Rust or to `loadModel` in Node.js, in place of `"en_core_web_sm"` in the [getting started example](index.md#getting-started).

You can install more than one model. Each is stored separately, and each `Model` you load is independent of the others.

## Where to go next

- [Installing a model](MODEL_INSTALLATION.md) explains where models are stored and how to install from a file you already have.
- [Developer guide](DEVELOPMENT.md#select-a-model-size) has a Rust example that loads all three sizes and reads their vectors.
- [Node.js guide](NODE.md) shows vectors and similarity in Node.js.
