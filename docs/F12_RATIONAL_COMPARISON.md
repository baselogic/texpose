# F12 — exact rational comparison fast-path (2026-10-08)

## Hypothesis and invariant

The existing non-negative rational comparator uses a Euclidean/continued-fraction
loop with `u128` division. For frequent small ratios, two checked cross-products
can compare exactly with less work. The comparator must remain total, signed,
exact, and panic-free for every normalized `Dim`, including products exceeding
`u128::MAX`. The original continued-fraction loop remains the overflow fallback.

## Measurement

Reported from the developer's Windows x86-64 TeXpose checkout, after F11
(commit `635286b180b44368af90f03264b0f96f48314fa9`), using
`cargo bench --bench layout` (optimized profile, 31 samples per workload).
The table shows median nanoseconds per operation; lower is better.
The CPU model and within-run CPU-frequency controls were not recorded.
These are sequential before/after runs, not randomized paired samples.

| Workload | Original | Checked product | Difference |
| --- | ---: | ---: | ---: |
| `dim cmp small` | 17.2 ns | 4.4 ns | 74.4% lower |
| `dim cmp large-small` | 18.8 ns | 4.4 ns | 76.6% lower |
| `dim cmp overflow` | 55.2 ns | 56.0 ns | 1.4% higher |
| `dim cmp identical large` | 77.8 ns | 77.4 ns | 0.5% lower |
| `layout simple` | 3274.1 ns | 3214.4 ns | 1.8% lower |
| `layout complex` | 44875.0 ns | 44557.4 ns | 0.7% lower |
| `layout stress` | 481634.4 ns | 467687.5 ns | 2.9% lower |

The end-to-end layout differences are not reliable attribution evidence: the
layout workloads contain other variable costs and the experiment lacks paired
isolation. The modest overflow difference is also too small to establish a
material slowdown. The large changes in the two targeted small-rational
benchmarks do support the fast-path for those inputs.

## Decision and preservation argument

Keep the `checked_mul` fast-path, subject to successful final correctness and
lint gates. Both products are compared only when *both* are representable in
`u128`; therefore the ordering equals the mathematical cross-product ordering
for positive denominators. When either multiplication overflows, execution
falls through unchanged to the exact Euclidean comparator. Signed ordering is
unchanged because `Dim::cmp` handles signs before calling this helper.

The existing deterministic arithmetic property tests and the new focused
overflow/sign test remain permanent contract tests. The four F12 timing probes
were removed from `benches/layout.rs` after the measurements: they were temporary
experimental instrumentation, not a standing performance gate. F11's separate
symbol-lookup benchmarks remain because they protect an ongoing contract.

No dependency, public API, or `unsafe` was introduced. Reconsider this choice
if a supported-target benchmark shows materially worse overflow-heavy or
end-to-end workloads, if production profiles show a high fallback rate, or if
later compilers alter checked-arithmetic code generation enough to reverse the
tradeoff. Exact behavior takes precedence over throughput.
