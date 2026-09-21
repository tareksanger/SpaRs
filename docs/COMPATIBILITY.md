# spaCy compatibility inventory

Implemented means native code exists; only the evidence column establishes its verified scope. Partial does not mean arbitrary configurations are supported.

| Surface | State | Evidence / missing work |
|---|---|---|
| Language/Pipeline inference | Partial | Model::process, sequential pipe, stage controls, offline consumer; pinned English only |
| Doc/Token/Span | Partial | Owned text, immutable indexed access; offset newtypes; checked borrowed views and native snapshots; children, ancestors, subtree and sentence access (98 documents / 5,568 reference tokens); no extensions |
| Vocabulary/StringStore/Lexeme | Partial | Hashing, norm/shape/features and vector map; read-only lexical flags and pinned Unicode data; no mutable vocab/string store |
| English tokenizer | Partial | 4,057 tokenizer/feature cases and 52,236 regex-span checks; English configuration only |
| MultiHashEmbed.v2 | Implemented | Six/four features, source-derived Murmur hashing, static projection |
| MaxoutWindowEncoder.v2 | Implemented | Normalized maxout, evolving padded residual context |
| Tagger.v2 | Implemented | Source-derived unnormalized linear label selection |
| TransitionBasedParser.v2 | Partial | Greedy arc-eager, learned padding, HEAD deprojection; 634 exact parser/NER action-trace steps |
| AttributeRuler | Partial | Official model's finite patterns and precedence; no public general matcher |
| English rule lemmatizer | Implemented | Official WordNet tables and English base-form handling |
| NER | Partial | Native BILUO inference on raw text; exact full-output and stage parity; no preset entities/blocking API |
| Sentence boundaries | Implemented | Parser roots/subtrees; independent senter/sentencizer not exposed |
| English noun chunks | Implemented | Upstream dependency/POS rules |
| Static vectors | Partial | Token/span/doc vectors and similarities verified; no floret or vector mutation |
| DependencyMatcher | Partial | All 20 relationships; seven token attributes and five predicates; ordered official reference comparisons; no callbacks, regex, fuzzy matching, or span input |
| Token Matcher | Partial | Typed shared conditions, repetition, overlapping spans and default discovery order; no greedy selection, alignments, callbacks, regex, fuzzy matching or span input |
| PhraseMatcher | Unimplemented | Next matcher priority; attribute selection, overlaps and result ordering need reference suites |
| EntityRuler/SpanRuler | Unimplemented | No public rule-based span annotation API |
| Retokenization | Unimplemented | No merge/split API |
| DocBin / spaCy byte serialization | Unimplemented | Validated native JSON snapshots; no spaCy format interoperability |
| Model serialization | Partial | Versioned native official-export format; not spaCy binary-compatible |
| Model installation without Python | Partial | [Native installer](MODEL_INSTALLATION.md) downloads or reads the pinned official package, converts and verifies all model data; new installations use resource revision 2, existing revision 1 installations remain verifiable; no automatic updates or second validated model release |
| Node-API bindings | Partial | Separate [Node crate](NODE.md): asynchronous loading/inference, ordered batches, stage controls, owned typed output and static vectors; 190 documents / 7,029 tokens; no matcher/traversal bindings or prebuilt npm packages |
| Browser WASM bindings | Unimplemented | Byte-backed loading, browser asset acquisition, compilation, fidelity and performance still need verification |
| Custom components / registry | Unimplemented | Only fixed ordered pipeline accepted |
| Morphologizer / trainable senter | Unimplemented | Export excludes disabled senter |
| Textcat, span categorizer/finder, entity linker, edit-tree lemmatizer | Unimplemented | No supported architectures |
| Transformers, character CNN, LSTM | Unimplemented | Explicitly outside supported model config |
| Other languages / multilingual resources | Unimplemented | English only |
| Training / optimizers / backprop / pretraining | Unimplemented | Official pretrained inference only |
| GPU, beam search, multiprocessing | Unimplemented | Native CPU greedy inference |
| Scorer / training Examples / alignment | Unimplemented | Development parity harness only |
| displaCy / projects / CLI ecosystem | Unimplemented | Standalone export and example tools only |

Completing one pipeline is not completing spaCy. Additional architectures, languages, matching, mutable docs, serialization and training remain substantial work and must receive their own acceptance suites.

Declared English suite: 190 documents / 7,029 tokens; exact discrete outputs. See [implementation status](PROGRESS.md) for the declared scope and the [quality process](QUALITY.md) to reproduce verification and locate run artifacts.
