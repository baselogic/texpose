# Fuzzing contract

This document owns the maintained fuzzing procedure for roadmap Phase H. Fuzz
campaigns are verification tooling, not production functionality. A discovered
bug is complete only after its minimized input is promoted to a deterministic
regression at the layer that owns the violated contract.

## Tooling boundary

The fuzz package is `fuzz/` and is intentionally separate from the production
crate. `libfuzzer-sys`, `num-bigint`, `num-rational`, and `num-traits` are
verification-only dependencies. The normal TeXpose dependency graph remains
unchanged.

Campaigns use `cargo-fuzz`/libFuzzer on a current Rust nightly under Linux. On
Windows development hosts, run them in WSL/Linux rather than changing TeXpose's
portable production code to accommodate a host-specific fuzz runner.

The committed `*.seed` files are small starting inputs. libFuzzer-generated
corpus entries, artifacts, and coverage output are ignored. `fuzz/Cargo.lock` is
committed after the first successful build so the verification dependency
graph stays reproducible. Preserve a generated input only while diagnosing it;
once the cause is understood, add the smallest deterministic regression that
protects the real contract and remove temporary campaign artifacts.

## H1 — `Dim`

H1 uses an independent arbitrary-precision rational oracle (`BigRational`) for
every accepted exact operation. The oracle does not call TeXpose arithmetic.
All target inputs are explicitly bounded.

| Target | Input domain | Protected contract | Bound |
| --- | --- | --- | --- |
| `dim_decimal` | UTF-8 strings | accepted decimal value and canonical `Dim` representation | 512 bytes |
| `dim_arithmetic` | two exact rational operands reconstructed from fixed-width bytes | checked add/sub/mul/div, total comparison, normalization | 64 significant bytes |
| `dim_units` | fixed-width font-unit and physical-length operands | font-unit conversion plus `pt`/`bp` resolution through the public layout boundary | 75 significant bytes |

`dim_decimal` deliberately compares only values accepted by `Dim::parse`.
Rejected text remains governed by the typed parsing contract; the fuzz target
does not redefine which out-of-range textual encodings TeXpose must accept.

`dim_arithmetic` constructs operands through public `Dim` operations while
spanning the full valid signed `i128` magnitude (excluding the forbidden
`i128::MIN` normalized numerator). An oracle result that fits the documented
`Dim` range must succeed exactly; an exact result outside that range must report
`NumericError::ArithmeticOverflow`. Division by zero must report
`NumericError::ZeroDenominator`.

`dim_units` checks `Dim::from_font_units` directly. Physical `pt` and `bp`
resolution is exercised through `layout_with_em_size_pt` using a single
`MathNode::Space` and the pinned STIX Two Math verification fixture, so the fuzz
campaign reaches the production owner of physical-unit conversion without
introducing a test-only visibility escape hatch. The oracle applies the exact
contracts `pt / root_em` and `bp * 7227/7200 / root_em`. The same public layout
entry point first normalizes TeX's physical `\nulldelimiterspace` (6/5 pt) and
delimiter shortfall (5 pt); if either exact normalization exceeds `Dim`, the
target expects the public boundary to return `NumericError::ArithmeticOverflow`
before the requested `Hspace` is reached.

### Float-conversion roadmap dependency

The H1 roadmap text also names the final `Dim -> f32` boundary. That boundary
does not exist in production yet: roadmap J2/J3 introduce the flat display list
and J5 owns the float-conversion contract. H1 therefore does not add a synthetic
or fuzz-only float API merely to satisfy a future target. When J5 introduces the
real boundary, its property/fuzz coverage must be added here against that
production implementation before J5 can close.

## Running H1

Install the runner once in the Linux/WSL environment:

```bash
rustup toolchain install nightly-2026-09-07
cargo install cargo-fuzz --version 0.13.2 --locked
```

Run bounded campaigns from the repository root:

```bash
cargo +nightly-2026-09-07 fuzz run dim_decimal -- -max_len=512 -max_total_time=300
cargo +nightly-2026-09-07 fuzz run dim_arithmetic -- -max_len=64 -max_total_time=300
cargo +nightly-2026-09-07 fuzz run dim_units -- -max_len=75 -max_total_time=300
```

For a longer campaign, increase `-max_total_time`; do not increase `-max_len`
without first changing and documenting the target's input/resource contract.

To reproduce a saved libFuzzer artifact, pass that exact file after the target
name, for example:

```bash
cargo +nightly-2026-09-07 fuzz run dim_arithmetic fuzz/artifacts/dim_arithmetic/<artifact>
```

Minimize before turning a finding into a regression:

```bash
cargo +nightly-2026-09-07 fuzz tmin dim_arithmetic fuzz/artifacts/dim_arithmetic/<artifact>
```

## H2 — Tokenization and parsing

H2 fuzzes only valid UTF-8 because the public tokenizer/parser boundary is
`&str`; arbitrary byte strings that are not valid UTF-8 are outside that API's
input domain and are discarded by the harness. Each libFuzzer input is capped at
4096 bytes inside the target as well as by the campaign command.

| Target | Input domain | Protected contract | Bound |
| --- | --- | --- | --- |
| `parser_tokenizer` | valid UTF-8 math source | deterministic token stream, ordered/non-overlapping source spans, typed trailing-backslash failure, no panic | 4096 bytes |
| `parser_syntax` | valid UTF-8 math source | deterministic AST/error, valid error spans, coherent typed resource-limit metadata, no panic | 4096 bytes |

`parser_syntax` uses the public `parse_with_options` boundary with deliberately
smaller budgets than production defaults: depth 24, 512 AST nodes, 64 total
environment rows, 256 total environment cells, and 1024 lexical tokens. The
committed corpus includes accepted examples for groups, generalized fractions,
environments, dimensions, delimiters, and scripts, plus inputs that cross each
resource budget. The fuzz target does not invent a second parser as an oracle;
syntax semantics remain covered by TeXpose's parser golds. H2 instead checks the
hostile-input invariants the roadmap owns: bounded work, typed failure,
determinism, source-span validity, and absence of panics.

Run bounded campaigns from the repository root:

```bash
cargo +nightly-2026-09-07 fuzz run parser_tokenizer -- -max_len=4096 -max_total_time=300
cargo +nightly-2026-09-07 fuzz run parser_syntax -- -max_len=4096 -max_total_time=300
```

Reproduce or minimize a saved parser artifact with the same `cargo fuzz run` /
`cargo fuzz tmin` procedure documented for H1, substituting the H2 target name.
An important finding is complete only after its minimized input becomes a
deterministic regression at the tokenizer/parser layer that owns the violated
contract.

## H3–H6

Later Phase H targets must add their own domain, invariant, oracle/reference,
resource bounds, corpus policy, and reproduction procedure here. Do not reuse H1
or H2 bounds as implicit policy for semantic, font, MATH-construction, or layout
fuzzing.
