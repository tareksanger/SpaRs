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
