# Font access cost and precomputation policy

This document records the E11 font-access experiment and the resulting indexing/precomputation decision. It is the durable record for this decision; the temporary probes and worktrees used to obtain the measurements are not part of the supported product.

## Decision

TeXpose does **not** add a persistent owned font index or new precomputed MATH data at E11.

The existing design remains:

- parse one `ttf_parser::Face` view per layout operation and reuse that view for the operation;
- resolve `cmap`, horizontal metrics, bounding boxes, MATH constants, variants, assemblies, and MathKern directly from that view;
- build `MathGsubPlan` lazily once per layout operation only if mathematical GSUB is needed, then reuse that plan for glyph substitutions;
- keep `MathFont` as shared immutable font bytes plus explicit face index rather than adding persistent lookup tables.

The direct MATH lookups measured here are small relative to end-to-end layout, and the experiment does not establish enough repeated same-glyph bounding-box traffic to justify an owned cache. `glyph_bounding_box` is the most expensive individual lookup measured, but lookup cost alone is not evidence that a persistent cache pays for its construction, memory, and additional state.

Reopen indexing/precomputation only if an end-to-end multi-font profile shows repeated font-table lookup as a material layout cost and a paired experiment demonstrates a net improvement across both small and large formulas without an unacceptable construction or memory penalty.

## Environment and method

Measurement date: 2026-10-02.

Candidate commit: `c351daf0ab4c784a07aee165255da218c2b58ff5` (`feat: degrade missing glyphs with diagnostics`).

Environment:

- Microsoft Windows NT 10.0.26200.0 / `x86_64-pc-windows-msvc`;
- Intel Core i5-13600KF;
- PowerShell 7.6.1 Core;
- Cargo 1.95.0 (`f2d3ce0bd`, 2026-03-21);
- rustc 1.95.0 (`59807616e`, 2026-04-14), LLVM 22.1.2;
- release build: `cargo build --release --bin texpose_e11_font_access_probe`;
- 31 timed samples per supported operation after calibration; iterations shown below are iterations per sample;
- reported values are nanoseconds per operation; median and p95 are computed across samples.

The probe measured direct access costs, not end-to-end layout. Capability discovery for a suitable glyph/construction was outside the timed region. `Face::parse` was measured independently from full `MathFont` construction/validation. The GSUB-plan row rebuilds the reachable `math`/`DefaultLangSys` plan every iteration; substitution rows reuse one already-built plan. Unsupported rows are capability observations, not zero-cost measurements.

Verification-font hashes:

| Font | SHA-256 |
| --- | --- |
| STIX Two Math | `f2076b9f1676438439dd41e23676f5ab99056e83d6b8f8c27841591ef2ccfa72` |
| Libertinus Math | `e81bd44acbb7119c8f00128b36fecc5d980e10d2450a226ba52402ccf4da9d32` |
| Fira Math | `2028cbd3dd4d8c0cf1608520eb4759956a83a67931d7b6d8e7c313520186e35b` |

## Results

| Font | Operation | Workload | Iterations/sample | Median ns | p95 ns |
| --- | --- | --- | ---: | ---: | ---: |
| STIX Two Math | `Face::parse` | parse face index 0 and read glyph count | 16384 | 454.510 | 457.605 |
| STIX Two Math | `glyph_index` | `Face::glyph_index('x')` | 262144 | 25.296 | 25.926 |
| STIX Two Math | `glyph_hor_advance` | `Face::glyph_hor_advance(gid('x'))` | 8388608 | 0.590 | 0.599 |
| STIX Two Math | `glyph_bounding_box` | `Face::glyph_bounding_box(gid('x'))` | 16384 | 353.876 | 368.732 |
| STIX Two Math | MathConstants access | MATH `axis_height` design units | 4194304 | 1.602 | 1.867 |
| STIX Two Math | variant lookup | horizontal construction gid 732, first prepared variant | 1048576 | 9.121 | 9.317 |
| STIX Two Math | assembly lookup | horizontal construction gid 746, first assembly part | 524288 | 12.231 | 12.321 |
| STIX Two Math | MathKern lookup | MathKernInfo gid 3, first available corner kern[0] | 262144 | 25.500 | 25.788 |
| STIX Two Math | math/default GSUB feature lookup | build reachable `ssty`/`flac`/`dtls` plan from `math` DefaultLangSys | 32768 | 265.082 | 267.392 |
| STIX Two Math | `ssty` level 1 substitution | gid 3 through adopted `MathGsubPlan` | 131072 | 46.210 | 47.919 |
| STIX Two Math | `ssty` level 2 substitution | gid 3 through adopted `MathGsubPlan` | 131072 | 46.291 | 47.768 |
| STIX Two Math | `flac` substitution | gid 728 through adopted `MathGsubPlan` | 262144 | 29.502 | 29.754 |
| STIX Two Math | `dtls` substitution | gid 263 through adopted `MathGsubPlan` | 262144 | 29.671 | 29.978 |
| Libertinus Math | `Face::parse` | parse face index 0 and read glyph count | 16384 | 378.180 | 381.915 |
| Libertinus Math | `glyph_index` | `Face::glyph_index('x')` | 262144 | 26.228 | 26.399 |
| Libertinus Math | `glyph_hor_advance` | `Face::glyph_hor_advance(gid('x'))` | 16777216 | 0.591 | 0.594 |
| Libertinus Math | `glyph_bounding_box` | `Face::glyph_bounding_box(gid('x'))` | 8192 | 637.292 | 655.762 |
| Libertinus Math | MathConstants access | MATH `axis_height` design units | 4194304 | 1.603 | 1.697 |
| Libertinus Math | variant lookup | vertical construction gid 9, first prepared variant | 524288 | 11.968 | 12.059 |
| Libertinus Math | assembly lookup | vertical construction gid 9, first assembly part | 524288 | 12.171 | 12.344 |
| Libertinus Math | MathKern lookup | no MathKernInfo in face | 0 | — | — |
| Libertinus Math | math/default GSUB feature lookup | build reachable `ssty`/`flac`/`dtls` plan from `math` DefaultLangSys | 65536 | 85.548 | 86.523 |
| Libertinus Math | `ssty` level 1 substitution | gid 1781 through adopted `MathGsubPlan` | 131072 | 45.422 | 46.323 |
| Libertinus Math | `ssty` level 2 substitution | gid 1781 through adopted `MathGsubPlan` | 131072 | 46.197 | 49.030 |
| Libertinus Math | `flac` substitution | feature absent or no substituting glyph in face | 0 | — | — |
| Libertinus Math | `dtls` substitution | feature absent or no substituting glyph in face | 0 | — | — |
| Fira Math | `Face::parse` | parse face index 0 and read glyph count | 32768 | 259.607 | 261.923 |
| Fira Math | `glyph_index` | `Face::glyph_index('x')` | 262144 | 26.018 | 26.165 |
| Fira Math | `glyph_hor_advance` | `Face::glyph_hor_advance(gid('x'))` | 16777216 | 0.590 | 0.594 |
| Fira Math | `glyph_bounding_box` | `Face::glyph_bounding_box(gid('x'))` | 65536 | 122.362 | 126.736 |
| Fira Math | MathConstants access | MATH `axis_height` design units | 4194304 | 1.603 | 1.705 |
| Fira Math | variant lookup | vertical construction gid 9, first prepared variant | 524288 | 10.109 | 10.978 |
| Fira Math | assembly lookup | vertical construction gid 9, first assembly part | 524288 | 10.707 | 12.033 |
| Fira Math | MathKern lookup | no MathKernInfo in face | 0 | — | — |
| Fira Math | math/default GSUB feature lookup | build reachable `ssty`/`flac`/`dtls` plan from `math` DefaultLangSys | 131072 | 47.791 | 48.175 |
| Fira Math | `ssty` level 1 substitution | gid 556 through adopted `MathGsubPlan` | 262144 | 22.559 | 22.895 |
| Fira Math | `ssty` level 2 substitution | gid 556 through adopted `MathGsubPlan` | 262144 | 22.541 | 22.583 |
| Fira Math | `flac` substitution | feature absent or no substituting glyph in face | 0 | — | — |
| Fira Math | `dtls` substitution | gid 74 through adopted `MathGsubPlan` | 262144 | 19.174 | 19.197 |

Capability differences are intentional observations from the committed fixtures: STIX Two Math supplied every probed operation; Libertinus Math had no MathKernInfo and no usable `flac`/`dtls` substitution for the probe; Fira Math had no MathKernInfo and no usable `flac` substitution.

## E8 regression investigation

E8 originally introduced a significant relative layout regression even though absolute targets still passed. The adopted lazy per-operation GSUB plan removed most of it, but E8 still recorded residual `plain-*` regressions for later investigation.

The E11 investigation repeated the comparison against E7 (`24d6bde`) using nine paired runs with alternating order on the same Windows 11 / i5-13600KF / Rust 1.95.0 environment. On the current E10 commit (`c351daf`), the historical regression no longer reproduces:

| Workload | CURRENT / E7 median |
| --- | ---: |
| `plain-1` | 0.9928x |
| `plain-32` | 0.9869x |
| `ssty` level 1 | 0.9789x |
| `ssty` level 2 | 0.9383x |
| `ssty` dense | 0.9311x |
| `ssty` deep dense | 0.9071x |
| layout fraction | 0.9346x |
| layout display equation | 0.9147x |

Source audit found a concrete E8 mechanism consistent with the historical regression. Before E10, `glyph_with_context` first called `font.glyph(ch)`, which resolved cmap and extracted advance/bounding-box metrics, then passed only the resulting glyph id to `glyph_id_with_context`; that path applied GSUB and called `font.glyph_id(...)`, extracting advance/bounding-box metrics again. E10 changed the required-scalar path to resolve only `glyph_index(ch)` before GSUB and extract metrics once afterward. The change was required for missing-glyph degradation, and it also removed the duplicate metric extraction.

The allocation hypothesis was tested separately. For `plain-32`, E7 and CURRENT had the same census: 3 allocations, 7 reallocations, and 42672 requested bytes for one warmed layout. An experimental row-storage change reduced this to 3 reallocations and 22848 requested bytes, but timing was mixed: `plain-32` improved modestly while other workloads, including the full display equation, did not. It is therefore not adopted as a runtime optimization. Experimental font fast-path and forced-inline variants likewise failed to produce a consistent paired benefit and are rejected.

## Per-access decision

- `Face::parse`: retain one parsed face view per layout operation. The measured cost is sub-microsecond on all three fixtures and does not justify a persistent self-referential face representation.
- `glyph_index`: direct lookup remains. About 25–26 ns did not justify a cmap mirror/index.
- `glyph_hor_advance`: direct lookup remains. The measured cost was about 0.59 ns.
- `glyph_bounding_box`: direct lookup remains despite being the largest measured per-glyph access (about 122–637 ns). No repeated-hit profile or paired end-to-end cache win was established.
- MathConstants: direct access remains; the measured `axis_height` lookup was about 1.6 ns.
- variants and assemblies: direct MATH construction lookup remains; measured lookups were about 9–12 ns.
- MathKern: direct lookup remains. The available STIX probe was about 25.5 ns; the other two fixtures lacked MathKernInfo.
- mathematical GSUB plan: keep the existing lazy per-operation `MathGsubPlan`. Rebuilding the plan cost about 48–265 ns depending on the font, while substitutions through a prepared plan cost about 19–46 ns. This supports reuse within an operation but does not justify a persistent `MathFont` index.
- no additional font-access cache, persistent lookup map, or owned precomputed table is introduced by E11.
