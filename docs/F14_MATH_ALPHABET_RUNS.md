# F14 — Direct construction of math-alphabet runs

## Decision and scope

**Decision (2026-10-08): adopt.** Relative to TeXpose commit
`5308c23093bdc8998eb9c73c31a4ff9cc712c322`, use the direct run builder
in `src/parser/parse.rs` rather than allocating one `String` per stylable
character and then merging the adjacent one-character nodes.

The implementation retains the `MathNode::MathAlphabet(String, TextStyle)`
representation and the existing semantic boundaries. Consecutive stylable
atoms and single-glyph commands with the same style append to one `String`.
Unstylable operators and symbols, literal text, scripts, fractions, and
radicals retain their distinct AST nodes. `collapse_runs` and the direct path
share the same rule for coalescing adjacent `MathAlphabet` and `LiteralText`
nodes; the direct path never restyles literal text.

## Experiment

- **Hypothesis:** avoiding per-character temporary `String` allocation and
  subsequent concatenation reduces parser cost for mathematical alphabet runs.
- **Alternatives:** existing map-then-collapse path (baseline) versus direct
  append-and-coalesce path (candidate). No new dependencies or unsafe code.
- **Machine and build:** Windows, x86_64-pc-windows-msvc, Cargo optimized
  `bench` profile. Each benchmark invocation gathered 31 calibrated timing
  samples with a target sample duration of 10 ms; three complete invocations
  were captured before and three after the change.
- **Statistic:** median of the three invocation-level medians, nanoseconds
  per `parse()` operation. These are end-to-end parse costs, not an allocation
  counter or a standalone timing of the modified helper.

| Parsed source workload | Baseline (ns) | Candidate (ns) | Time reduction |
| --- | ---: | ---: | ---: |
| `\mathbf{abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789}` | 5,859.4 | 3,930.0 | 32.9% |
| `\mathbf{ab+cd+ef+gh+ij+kl+mn+op+qr+st+uv+wx+yz}` | 4,547.7 | 4,042.6 | 11.1% |
| `\mathbf{\frac{abcde}{fghij}+x_{kl}^{mn}}` | 2,980.6 | 2,585.1 | 13.3% |

Invocation-level medians, in chronological order (ns):

| Workload | Baseline runs | Candidate runs |
| --- | --- | --- |
| Long run | 5,800.9 / 5,928.3 / 5,859.4 | 3,834.5 / 3,949.6 / 3,930.0 |
| Split run | 4,453.8 / 4,547.7 / 4,604.1 | 3,981.3 / 4,087.7 / 4,042.6 |
| Nested run | 2,980.6 / 2,965.8 / 3,036.6 | 2,570.8 / 2,585.1 / 2,612.4 |

The repeated measurements support the decision for these source workloads.
They do not quantify allocations directly, prove faster layout on an already
parsed AST, or establish performance on other hosts or input distributions.
The one-pass allocation reduction is a structural claim about the edited code.

## Correctness and maintenance

The Windows verification log for this candidate recorded:

- `cargo fmt --all -- --check`: passed.
- `cargo test --test math_alphabet_runs`: 3/3 passed.
- `cargo test`: 226 unit tests passed (one ignored), all integration tests
  passed, and 40 doctests passed.
- `cargo clippy --all-targets -- -D warnings`: passed.
- `cargo +1.76.0 check`: passed.
- `git diff --check`: passed.

The permanent tests in `tests/math_alphabet_runs.rs` cover contiguous Latin and
Greek runs, non-stylable separators, literal text, scripts, fractions, and
radicals. The transient F14 benchmark scenarios were removed from
`benches/layout.rs` after this decision; their evidence is retained above.

Reconsider if another input distribution shows a material regression, the
AST coalescing contract changes, or profiling identifies a different dominant
cost in mathematical alphabet parsing. Measure on the actual target before
making new runtime claims.
