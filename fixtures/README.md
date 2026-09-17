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
