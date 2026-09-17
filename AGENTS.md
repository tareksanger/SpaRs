# Contributing agent guidance

## Project scope

SpaRs (`spars`) is a standalone native Rust NLP library. The current supported model is the official `en_core_web_md` 3.8.0 export, using spaCy 3.8.14 and Thinc 8.3.13 as the reference. Keep broader compatibility status in `docs/COMPATIBILITY.md`; one English pipeline does not establish full spaCy parity.

Production tokenization and inference must run natively in Rust. Python belongs only in development, export, and reference tooling. Keep acquisition explicit and processing offline.

## Documentation

Keep each prose paragraph on one source line; do not hard-wrap Markdown to a fixed column width. Keep structural line breaks for headings, list items, tables, and code blocks.

Commit documentation for lasting supported behavior, repeatable workflows, settled design decisions, and enduring limitations. Keep active-task notes, exploratory profiling, interim benchmark runs, and temporary findings in ignored `target/` files or temporary storage. Update public guides after the result is verified; do not commit a new document or report for each work step. Keep reusable diagnostic tools separate from their temporary output. Write generated test, verification, and benchmark reports under ignored `target/reports/`; CI retains them as artifacts. Keep tests, fixtures, expected results, checksums, and source/model provenance in Git.

Write repository documentation for users and contributors who have not read any development conversation. Describe the software, supported behavior, limitations, design rationale, setup, verification, and contribution requirements.

- Exclude conversation summaries, agent handoffs, user requests, local workspace arrangements, commit organization, and repository-setup narration.
- Describe the resulting behavior rather than recounting how a task was assigned or completed. For example, document the supported model and parity suite instead of saying that an agent finished the requested milestone.
- Keep `docs/PROGRESS.md` as a durable implementation inventory with evidence, unresolved technical blockers, and concrete next work, not a session diary.
- Preserve technically relevant history: source provenance, fixture influence, resolved compatibility differences, and reasons for implementation constraints.
- Scope claims to their evidence. Distinguish implemented, verified, partial, and unsupported behavior. Link validation claims to reports and reproducible commands; do not infer linguistic accuracy from reference agreement.
- Avoid transient publication or CI-status claims in prose. Document how to install, verify, or locate CI results instead. Generated report metadata must describe only what that generator actually checks.
- Save only project-relative filesystem paths in fixtures, reports, and documentation. Remove external host paths from captured diagnostics before saving them. Never record personal home directories or temporary-directory locations. Keep upstream source URLs intact.
- Use repository-relative links and portable commands. Do not assume readers know a URL, path, or artifact mentioned in a conversation.
- When changing behavior, reconcile the README, compatibility inventory, model format, and validation documentation where relevant. Do not rewrite license notices or remove provenance to simplify prose.

## Quality reviews

Follow `docs/QUALITY.md` for every behavior change. Add a failing reference or invariant test before the fix, run focused checks and the full acceptance suite, and add an executed developer example when public behavior changes. For internal refactors and optimizations, rerun the existing examples instead of adding redundant documentation. Keep fixture inputs and expected outputs frozen; review checksum changes explicitly.

Delegate independent review of substantial behavior changes to the project agents `reference_review`, `test_review`, and `docs_review`, using `.codex/agents/*.toml`. Give each reviewer the changed files and a bounded question. Reviewers report findings; the main agent owns edits and test execution. If custom roles are unavailable, pass the matching instructions to a standard subagent. Wait for findings and resolve concrete issues before claiming verification. Documentation-only changes need the documentation review, not three redundant reviews.

Write plain English and explain necessary technical terms. Guide examples must run from their Markdown source during `tools/check_docs.py`; compiling without running does not count as a tested example.

## Strong typing

Use concrete types throughout Rust and Python, including tools and tests. Represent known records with structs, enums, dataclasses, or TypedDicts rather than generic dictionaries. Annotate Python function parameters and return values. Validate external JSON and upstream library values at the boundary, then pass typed records into computation. Generic JSON is appropriate for decoding, serialization, and deliberately malformed test inputs, not as a substitute for model or domain types.

Run strict Pyright and `tools/check_typing_policy.py` through the acceptance script. Fix errors rather than adding `Any`, unchecked casts, ignored diagnostics, or weaker checker settings. Keep local stubs narrow and accurate to the pinned upstream interfaces; exercise those interfaces against the real reference environment. Type annotations do not replace shape, bounds, or semantic validation.

## Performance changes

Speed is a required design constraint for every runtime change. Delegate an independent review to `performance_review` for changes to runtime algorithms, model loading, data structures, batching, or runtime dependencies, including new features. Give it the affected paths and available before/after measurements. If the custom role is unavailable, use a standard subagent with its instructions. Resolve measured regressions before completion; report missing measurements as unverified performance.

Profile the actual workload before choosing an optimization. Measure the final implementation against the same baseline, model, corpus, build mode, thread limits, and warmup procedure. Keep loading, inference, and process memory separate. Run measurements without competing test or benchmark processes.

Optimize measured hot paths with reusable buffers, precompiled immutable resources, cached per-call features, and matrix operations across token rows where appropriate. Preserve model reuse and call isolation. Keep numerical tolerances and discrete-output expectations unchanged; an optimization that fails parity is unfinished.

Keep unsafe numerical code inside a small checked wrapper. Validate dimensions, strides, bounds, and nonaliasing assumptions, explain the safety argument, and test rectangular, empty, and malformed inputs against an independent calculation. Changes to such wrappers need independent review.

## Implementation and verification

Keep changes small and focused. Prefer pure computation, immutable loaded models, and per-call state. Preserve exact text, whitespace, and distinct byte offsets, code-point offsets, and token indices. Unsupported configurations and unavailable annotations must remain explicit rather than being omitted or fabricated.

Keep source files focused, unit tests in separate `tests.rs` modules, integration tests in `tests/`, and runnable examples in `examples/`. Export tooling must stay outside normal consumer Cargo builds.

Do not change frozen evaluation inputs or expected outputs to make an implementation pass. Add regressions separately. Preserve declared numerical tolerances and exact discrete-output requirements; record unresolved mismatches.

For runtime or model changes, follow the setup in `README.md` and run `.venv/bin/python tools/verify.py`. Ordinary `cargo test` skips model-dependent tests; the acceptance run requires `--include-ignored` and available model assets. For documentation-only changes, check factual claims, links, and command consistency; a full model export and parity run is unnecessary unless affected.

Preserve unrelated working-tree changes, including generated verification reports. Keep commits scoped. Repository pushes, releases, and registry publication require user authorization; authorization to push does not imply authorization to publish a package or release.
