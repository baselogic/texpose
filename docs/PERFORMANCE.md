# Performance evidence and decisions

This document is the durable Phase I performance record. `benches/layout.rs` is the maintained benchmark surface; historical experiments and rejected implementations are not kept live merely as evidence.

## Current policy

TeXpose keeps the current production architecture after Phase I. The I1/I3 benchmark results identify representative costs, but they do not establish a causal mechanism that justifies a new optimization. In particular, the higher measured cost of glyph assembly and `aligned` layout is not by itself evidence for a cache, alternate representation, special-case fast path, or public profiling API.

The font-access decision remains the E11/I2 result recorded in `docs/FONT_ACCESS.md`: direct OpenType/MATH lookup stays in place, `MathGsubPlan` remains lazy and reusable within one layout operation, and `MathFont` does not gain persistent lookup maps or owned precomputed MATH tables.

No machine-specific latency threshold is part of the correctness gate. Absolute timings are environment-dependent. Performance changes require paired measurements on the same controlled environment and a causal explanation of the mechanism being changed.

## Maintained benchmark contract

Run the suite with:

```text
cargo bench --bench layout
```

The benchmark requires Cargo's optimized bench profile, uses the committed STIX Two Math fixture, calibrates each batch toward 10 ms, and records 31 samples. It reports minimum, median, p95, and maximum nanoseconds per operation. Parsing is performed outside each I3 layout measurement so the primitive workloads measure layout rather than parser cost.

The maintained I1 boundaries are `MathFont` construction, `ttf_parser::Face::parse`, parsing, simple layout, complex layout, and the committed `hard-brutal-core` stress case. Semantic normalization remains inside layout because it is a private proof boundary; Phase I does not widen production visibility solely for benchmarking. The recorded I1 baseline predates Phase J. J2/J5 now make exact positioned flattening and final `Dim -> f32` emission part of the public `layout` path, so post-J benchmark runs must be treated as a new public-layout baseline rather than compared as a same-boundary regression.

The maintained I3 workloads are:

| Workload | Source bytes | Style | Intent |
| --- | ---: | --- | --- |
| scripts | 17 | Text | nested sub/superscripts |
| fractions | 31 | Display | nested generalized fraction layout |
| radicals | 19 | Display | indexed radical |
| delimiters | 28 | Display | large delimiter selection |
| assembly | 31 | Display | forced vertical glyph assembly |
| large operator | 18 | Display | display operator with limits/scripts |
| matrix | 45 | Display | 3x3 matrix grid |
| aligned | 74 | Display | three-row aligned environment |

The I1 simple and complex sources are 1 and 47 bytes respectively. The committed `hard-brutal-core` stress source is 298 bytes in the current fixture.

### Phase L1 benchmark boundary

L1 removes lexer/token types from the consumer API. The isolated `tokenize complex` row below remains historical Phase-I evidence only; it is no longer emitted by the maintained integration benchmark because retaining a public tokenizer solely for benchmark access would make test instrumentation define production API. Parser end-to-end cost remains measured by `parse complex`. Reintroducing a lexer-only benchmark requires an internal benchmark mechanism that does not widen the public crate surface.

## Validation snapshot — 2026-10-07

The following operator-provided run passed `cargo fmt --check`, `cargo test`, `cargo clippy --all-targets`, and `cargo bench --bench layout`. The benchmark process is visibly a Windows `.exe` run under PowerShell. Exact CPU model, Rust target triple, Cargo version, and rustc version were not captured in this transcript, so these absolute timings are a local reference snapshot only. They must not be compared as a cross-machine or cross-toolchain regression threshold.

I1/core measurements, in nanoseconds per operation:

| Operation | Input | Min | Median | p95 | Max | Batch |
| --- | --- | ---: | ---: | ---: | ---: | ---: |
| `MathFont` construction | STIX Two Math fixture | 547.6 | 550.9 | 561.8 | 568.6 | 32768 |
| `Face::parse` | STIX Two Math fixture | 450.1 | 452.9 | 460.2 | 471.2 | 32768 |
| tokenize complex | 47-byte source | 378.0 | 382.5 | 389.2 | 390.6 | 32768 |
| parse complex | 47-byte source | 6206.5 | 6243.7 | 6426.9 | 6453.9 | 2048 |
| layout simple | 1-byte source | 2943.6 | 2967.7 | 3003.6 | 3024.7 | 4096 |
| layout complex | 47-byte source | 34419.9 | 34831.4 | 35753.5 | 35762.1 | 512 |
| layout stress | 298-byte `hard-brutal-core` | 418521.9 | 425575.0 | 456475.0 | 467090.6 | 32 |

I3 primitive-layout measurements, in nanoseconds per operation:

| Workload | Min | Median | p95 | Max | Batch |
| --- | ---: | ---: | ---: | ---: | ---: |
| scripts | 20237.7 | 20430.1 | 20944.9 | 21490.4 | 512 |
| fractions | 18783.8 | 19114.1 | 19375.1 | 19472.1 | 1024 |
| radicals | 28212.7 | 28360.7 | 28770.9 | 29091.4 | 512 |
| delimiters | 33185.7 | 33572.3 | 34037.9 | 34052.5 | 512 |
| assembly | 98921.1 | 100409.4 | 104619.5 | 105428.1 | 128 |
| large operator | 19132.8 | 19435.9 | 22059.0 | 22287.5 | 512 |
| matrix | 24761.3 | 25062.3 | 25679.7 | 25698.4 | 512 |
| aligned | 126002.3 | 128335.9 | 130510.2 | 130651.6 | 128 |

This snapshot establishes only the measured distributions for these workloads on that run. It does not prove that any named internal function is hot, nor does it isolate allocation, lookup, branch, cache, or algorithmic cost.

## Production decisions

### Font lookup and precomputation

Hypothesis: repeated OpenType/MATH access might justify persistent indexes or caches in `MathFont`.

Decision: rejected, except for the already-adopted lazy per-operation `MathGsubPlan`. The E11/I2 experiment includes its exact Windows target/environment, Rust toolchain, workloads, baselines, distributions, structural audit, allocation evidence, rejected variants, and reopening condition in `docs/FONT_ACCESS.md`.

Memory effect: no persistent font index or cache is added. The retained GSUB plan is operation-local and constructed only when mathematical GSUB is needed.

### Core and primitive layout

Hypothesis: the Phase I benchmark census might expose a production optimization that is justified without further profiling.

Decision: no production change. Among the selected primitive workloads, `aligned` and forced glyph assembly cost more than the other measured primitives in this snapshot, but workload cost alone does not identify the responsible mechanism. Adding caches, special cases, alternate box representations, or extra precomputation without a profile and paired candidate measurement would convert an observation into an unsupported architectural cost.

Memory effect: none; I1/I3 add benchmark code only and do not alter runtime state or production allocation policy.

Structural evidence: I3 parses its sources before timing and repeatedly measures the existing `layout` entry point. The benchmark therefore avoids attributing parser cost to primitive layout, but it intentionally does not claim finer attribution inside layout.

Adopted mechanism: none. The existing production implementation remains authoritative.

## Retained and removed experimental machinery

`benches/layout.rs` remains because the roadmap gives it an ongoing contract as the maintained core/primitive benchmark suite. It is not retained as historical archaeology.

Temporary E11 font-access probes, alternate fast paths, forced-inline variants, and experimental row-storage changes were removed after the decision; `docs/FONT_ACCESS.md` preserves the evidence and rationale. Phase I adds no public profiling API, counters, feature switches, or alternate backend.

## Reopen conditions

Reopen a layout optimization decision only when all of the following hold:

1. a controlled end-to-end profile or equivalent evidence identifies a concrete repeated mechanism as material for a representative workload;
2. a candidate change has a stated causal model, not merely a lower number on one formula;
3. paired release measurements on the same target/toolchain show a repeatable improvement across the workloads affected by the change, including small and large formulas where relevant;
4. correctness gates remain clean;
5. construction cost, peak/retained memory, dependency cost, and added state are measured when the candidate can affect them.

Reopen the font-index/precomputation decision under the more specific condition in `docs/FONT_ACCESS.md`: an end-to-end multi-font profile must show repeated font-table lookup as a material cost, and a paired experiment must demonstrate a net gain without unacceptable construction or memory penalty.
