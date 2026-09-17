# Fixture provenance and use

Inputs were authored for this project; MIT licensed with the project. Outputs are computed by official spaCy 3.8.14, Thinc 8.3.13, en_core_web_md 3.8.0. Model resource licenses apply to copied lexical/vector data; see [licenses](../licenses/).

- development.json: 25 inputs used during implementation; development.expected.json records offsets, lexical feature IDs, shared tok2vec, all final annotations and doc vectors. Currency/dash BASE_NORMS fixes were influenced by this set.
- holdout.json: 10 inputs frozen before implementation comparison; exact agreement on first evaluation, no implementation choices were selected from its outputs. It is now a regression gate, not a fresh future holdout.
- regression.json: 9 subsequent stress cases, including long input (640 tokens), Unicode class/lowercase and whitespace cases. Kept separate from frozen sets.
- stages.expected.json: 25 development cases, each embedding mix and each encoder layer for shared/NER tok2vec; parser/NER context IDs, validity masks, scores and selected action indices. Includes pseudo-projective L-pobj||prep decoding.
- tokenizer.expected.json: 4,057 cases, all 1,347 official exceptions and affix compositions plus Unicode, whitespace and URL cases; 14,737 tokens.
- lexical.expected.json: 4,374 cases covering Unicode property range boundaries and lexical flags. Influenced the exported lower mappings and anchored email fix.
- regex.expected.json: 13,059 texts with 52,236 direct regex span comparisons, from the frozen lexical boundary characters in URL/affix/infix contexts.
- hash.expected.json: 30 Thinc uint64 hash/seed cases including maximal uint64.
- vectors.expected.json: 8 exact static rows/availability checks and 6 similarity pairs, including empty and missing-vector identity behavior.

Generators (run from the repository root): `tools/fixtures.py SPLIT`, [tools/stages.py](../tools/stages.py), [tools/tokenizer_fixtures.py](../tools/tokenizer_fixtures.py), [tools/lexical_fixtures.py](../tools/lexical_fixtures.py), and [tools/regex_fixtures.py](../tools/regex_fixtures.py). Expected fixtures are not generated in normal CI; model export is. The generators exist to reproduce the reference, not to redefine expected results during fixes. Numerical tolerance justification is in the [fidelity contract](../docs/VALIDATION.md).

## Expanded evaluation

- `evaluation-v1.json`: 98 original synthetic inputs across 13 categories, including news-style prose, science, commerce, conversation, instructions, narrative, questions, coordination, negation, web text, Unicode, layout, and long inputs. Frozen before the first comparison. The first run exposed the tokenizer contraction-boundary bug, so this set influenced the correction. The discovered exception-ordering issue and correction are described in [implementation status](../docs/PROGRESS.md). Expected annotations cover 5,568 tokens. The `web-07` path example uses a project-relative path; its expected output was regenerated with the pinned official reference when absolute paths were removed. The other 97 cases remain unchanged.
- `tokenizer-boundaries-v1.json`: 24 cases / 120 tokens chosen to check the discovered exception-ordering issue. Covers contractions around hyphens, slashes, dashes, punctuation, and whitespace, including curly apostrophes. These are development regressions, not a holdout.
- `unseen-v1.json`: 24 new cases / 306 tokens authored and frozen after the tokenizer correction. The first comparison passed without further runtime changes. It is now a regression suite and cannot support a future claim about unseen inputs.

All three input sets are original project-authored examples under the project MIT license. They are not sampled news articles or a representative accuracy benchmark. `tools/evaluation_reference.py` generates their expected files using the pinned official environment, records the input hash, and refuses to overwrite an existing expected file. Expected token annotations, spans, and document vectors are compared; intermediate activations continue to be checked by the separate stage suite.

`checksums.json` freezes the inventory and contents of every JSON fixture. New fixtures need a reviewed checksum entry. Existing hashes and expected values must not change just to make an implementation pass. See the [quality process](../docs/QUALITY.md).

## Dependency traversal

`traversal-v1.expected.json` records official spaCy 3.8.14 children, ancestors, subtree order, and containing sentence for 5,568 tokens across the 98 existing evaluation documents. It reuses project-authored MIT-licensed input text and influenced traversal implementation; it is a regression suite, not an untouched comparison set. `tools/traversal_reference.py` records the input hash, verifies regenerated dependency heads against the frozen input, and refuses to overwrite an output file. To reproduce it after reference setup, run `.venv/bin/python tools/traversal_reference.py fixtures/evaluation-v1.expected.json target/reports/traversal-regenerated.json` and compare that file with the frozen fixture. Rust unit tests separately cover crossing dependencies, missing annotations, cycles, and long chains.

## Dependency matching

`dependency-match-v1.expected.json` contains six project-authored cases under the MIT license: parsed English, multiple sentences, Unicode, empty input, a constructed crossing-dependency tree, and morphology normalization. Each case applies 52 rule additions covering all 20 official relationship operators, seven token attributes, duplicate patterns, repeated token bindings, and stable result ordering. These cases influenced the initial implementation and are regression evidence, not a linguistic accuracy benchmark. `tools/dependency_match_reference.py` obtains expected matches from pinned spaCy 3.8.14, records its source checksum, and refuses to overwrite output. CI regenerates this fixture and compares it with the frozen expectations.

`dependency-match-regressions-v1.expected.json` adds separate source-review regressions for empty and duplicate morphology, POS-field normalization, conjunctive conditions, and selective dependency-tree ordering. It is generated with `tools/dependency_match_reference.py --regressions OUTPUT`. The original dependency matcher fixture remains unchanged. Both fixtures influenced implementation and are regenerated in CI.

## Token Matcher

`token-match-v1.expected.json` contains 10 development documents, 38 tokens, 480 rule registrations, and 704 ordered matches from official spaCy 3.8.14 and Thinc 8.3.13 with `en_core_web_md` 3.8.0. Inputs are project-authored (MIT) and cover token conditions, repetition, overlaps, duplicate patterns, rule updates, empty input, and Unicode. The generator records the official matcher source checksum. These cases guided implementation and are regression evidence, not an untouched accuracy evaluation.

Regenerate to a new path with `.venv/bin/python tools/token_match_reference.py target/token-match.expected.json`. The generator refuses to overwrite an existing file. Acceptance regenerates and compares this fixture without modifying the checked-in expectations.

`token-match-exhaustive-v1.expected.json` covers every sequence of the five basic repetition operators up to three items long, against all binary `a`/`b` token sequences up to four tokens: 31 documents, 4,805 rule registrations and 6,261 ordered matches. `token-match-branching-v1.expected.json` adds 248 registrations and 288 matches for optional and mixed optional/star patterns of length 9 and 16, including failing suffixes. These separate project-authored MIT regression fixtures verify equivalent-state pruning; they do not change the original expectations. Regenerate with `--exhaustive` or `--branching` before the output path.
