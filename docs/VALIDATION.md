# Fidelity contract

Primary reference: official spaCy 3.8.14, Thinc 8.3.13, English medium
model 3.8.0 on CPU. Python 3.12.5. The disabled senter is not part of this pipeline.

Discrete annotations, feature IDs, token offsets, transition choices and span
boundaries must agree exactly for every case. Inputs in [development.json](../fixtures/development.json)
were authored for this implementation and influence fixes. [holdout.json](../fixtures/holdout.json)
was frozen before the first comparison and is not used to select rules or weights.
New failures go into a separate regression corpus; expected outputs are never
edited to accommodate Rust output.

Floating comparison contract, specified before numerical acceptance assertions:
`abs(actual - expected) <= 2e-5 + 2e-5 * abs(expected)` for normalized activations,
`<= 2e-4 + 2e-5 * abs(expected)` for unnormalized classifier/transition scores,
and `<= 2e-6 + 2e-6 * abs(expected)` for averaged static vectors. All operands must
be finite and dimensions must match. Static vector rows themselves must be exact.

The native baseline uses scalar f32 products and sums; Thinc uses BLAS with a
different reduction order. f32 epsilon is about 1.19e-7 and the longest dot product
has 672 terms (a worst-case relative accumulation bound about 8e-5 before
cancellation/normalization). These limits allow small reduction-order differences
while requiring exact decisions. They are empirical engineering limits, not a
proof bounding every possible input. Near score ties can still cause discrete
mismatches, which fail regardless of floating tolerances. No tolerance is used to
excuse annotation or action differences.

Empty input has zero rows: the pipeline handles it, but calling Thinc's raw
MultiHashEmbed model directly with only an empty document raises a BLAS shape
error. Fixtures record an empty activation array and still invoke official
Language on the empty input for annotations.

Reports retain inputs, versions, expected/actual differences and denominators.
No throughput or linguistic-accuracy claims follow from these tests.
