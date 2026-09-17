# How changes become verified

A passing test is useful only if it would catch an incorrect result. Every new feature needs a clear behavior contract, tests that run, and an example that a developer can follow. Review checks whether those tests are sufficient; automated checks run them consistently. Neither can promise that every possible bug has been found.

## Work in small steps

1. Define one behavior and its supported inputs. Name the official spaCy and Thinc versions. Record unsupported options in the compatibility inventory.
2. Read the relevant upstream code. Record the source version and explain any important ordering or numerical rules in the change description.
3. Add reference cases before implementing the behavior. Include an ordinary example, a boundary case, and an invalid input where the API can receive one. Add repeated-call or concurrent-call tests when shared state could matter.
4. Confirm the new test fails for the missing or incorrect behavior. The failure should explain the input, expected result, and actual result. A deliberate small defect can also demonstrate that a test detects the behavior it claims to cover.
5. Implement the behavior in Rust. Keep Python in the reference and export tools.
6. Run the focused tests, then the full acceptance command below. Add a runnable Rust example when public behavior changes and explain its result in plain English. For internal refactors and optimizations, rerun the existing examples.
7. Review the change and its test evidence. Resolve findings, update the compatibility inventory, and report any remaining limits before calling the feature verified.

For a model or reference-version change, create a new fixture version and document the expected behavior changes. Do not silently replace the old comparison set.

## Choose tests that answer different questions

| Change | Required evidence |
|---|---|
| Tokenization or lexical features | Exact token text, whitespace, offsets, normalization and relevant feature IDs against spaCy; Unicode and exception boundaries |
| Neural calculations | Small numerical checks and intermediate outputs from the official model, using the recorded float tolerances |
| Parser or named entity recognition (NER) | Valid actions and selected actions where affected, followed by exact final token annotations and spans |
| Public API | A caller example, unavailable versus empty results, invalid arguments, and state isolation where relevant |
| Model loading | A valid official export and malformed resources that reach the affected validation path |
| Performance optimization | All fidelity tests pass first, then comparable before/after measurements on the same machine and corpus |
| Documentation | Rust snippets execute, links resolve, commands match the tools, and a reader can understand the result |

“Exact” means every expected discrete value agrees. Matching most tokens does not pass. Numerical tolerances allow small differences in floating-point calculations; they never excuse a different tag, dependency, or entity. See [validation](VALIDATION.md) for the limits and their reasoning.

## Use strong types

Use named records for data with known fields. Rust model configuration uses structs and enums, so an unknown feature or a tensor reference with the wrong type fails when the model loads. Python tools use annotated functions, dataclasses, and TypedDicts (dictionaries whose field names and value types are specified).

External data still needs validation. For example, `json_int(True)` raises `ValueError`, even though Python normally treats booleans as integers. `tools/test_quality_tools.py` tests this boundary and malformed model metadata. Types cannot prove that an array has the right dimensions, so model loading also checks shapes and bounds.

Run the typing checks independently with:

```sh
tools/node_modules/.bin/pyright --project pyrightconfig.json
.venv/bin/python tools/check_typing_policy.py
```

Pyright runs in strict mode over all Python tools. The policy check requires function annotations and rejects `Any`, unchecked casts, and ignored type errors. Resolve the underlying type mismatch instead of weakening these checks. Narrow local stubs describe the spaCy and Thinc interfaces used by the exporter; a byte-identical export and regenerated reference fixture comparisons test those declarations against the real libraries. Regenerated annotations and hashes must match exactly; neural values use the existing floating-point limits in [validation](VALIDATION.md). Stubs must describe actual behavior, not merely satisfy the checker.

## Keep saved data portable

Fixtures and reports must not contain personal directories or absolute filesystem paths. Use project-relative paths when a path is needed. Report generators remove external host paths from captured output, and the quality check rejects absolute paths in saved JSON artifacts. Source URLs remain intact for attribution.

## Keep evaluation data stable

A fixture is a saved input and its expected output. Official spaCy produces the expected outputs; SpaRs reads them in tests. `fixtures/checksums.json` records the file hashes so accidental edits fail verification. A hash detects changes; reviewing a changed checksum still requires a person or reviewer to check the reason.

Keep original inputs and expected outputs unchanged when fixing a bug. Add a new regression file instead. Once a comparison set has influenced a fix, treat it as a regression suite. A future claim about unseen inputs needs a newly frozen set that has not influenced implementation.

New full-pipeline fixture files can be generated with `.venv/bin/python tools/evaluation_reference.py INPUT OUTPUT`. Both arguments are paths. The input is a versioned JSON object with a `cases` list; each case has `id`, `category`, and `text`. The tool refuses to overwrite an existing output. Record the source, license, purpose, and which cases influenced fixes in [fixture provenance](../fixtures/README.md). Review new hashes before adding them to `fixtures/checksums.json`.

## Run the acceptance checks

After the setup in the [developer guide](DEVELOPMENT.md), run:

```sh
.venv/bin/python tools/verify.py
```

This checks strict Python types, the explicit typing policy, frozen fixtures, documentation structure, reviewer configuration, quality-tool failure cases, regeneration of the development, stage, and hash reference fixtures, Rust formatting, Clippy (Rust code checks), all Rust tests, executed Markdown examples, the separate native consumer, Cargo packaging, and a second official export. Missing model assets fail the run. Packaging verifies a local archive; it does not publish a release.

Use this command for just the expanded reference comparison:

```sh
cargo test --release --offline --test evaluation -- --include-ignored
```

Use this command for malformed-input and state-isolation checks:

```sh
cargo test --release --offline --test robustness -- --include-ignored
```

Ordinary `cargo test` skips model-dependent tests. Acceptance and CI include them explicitly. CI uses the same verification script as local development, saves reports even on failure, and runs on Linux and macOS. Repository administrators should make both `native-fidelity` jobs required before merging; the workflow alone does not configure branch protection.

The verification report records commands, outputs, versions, and fixture hashes. Individual comparison reports show denominators and mismatches. Benchmark results live separately because machine speed should not determine whether the annotations are correct.

## Use reviewers with clear jobs

The project defines four Codex reviewers in [`.codex/agents`](../.codex/agents):

- `reference_review` checks behavior against official source and reference results.
- `test_review` checks whether tests catch mistakes and actually execute in CI.
- `docs_review` checks plain English, examples, and supported-feature claims.
- `performance_review` checks processing time, loading time, memory use, repeated work, and how cost grows with input size. Runtime changes require this review, including new features and dependency changes.

Ask Codex: “Review this change with reference_review, test_review, and docs_review. Give each reviewer its matching scope, wait for their findings, and resolve concrete issues before marking it verified.” Include `performance_review` whenever runtime performance can change. Supply the changed files and comparable before/after results; missing measurements mean performance is unverified. Resolve measured regressions before completion. The main agent makes changes and runs tests; the reviewers inspect files and report findings. Their reports supplement the automated checks.

The files follow the [official custom-agent format](https://learn.chatgpt.com/docs/agent-configuration/subagents). They inherit the selected model and use read-only review instructions. Configuration syntax is checked in CI; loading the files still depends on the Codex client. These agents are not a background service and do not run inside GitHub Actions.

## Write documentation for developers

Use ordinary words, short paragraphs, and concrete examples. Explain a technical term when it is first needed. Show prerequisites, a command or code example, the result to expect, and the feature's limits. Keep conversation history and task handoffs out of public docs. Keep one prose paragraph per source line.

Rust examples in the README and `docs/` use plain `rust` fences and assertions. `tools/check_docs.py` compiles and runs the actual fenced code; it rejects `ignore`, `no_run`, and `compile_fail` flags in these guides. Source API documentation also has Cargo doc tests; a `no_run` source example is compile-checked only. Readability and adequate explanations remain review responsibilities.
