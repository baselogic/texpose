# F13 — Borrowed control-sequence names in the tokenizer

## Decision and scope

**Decision (2026-10-08): adopt.** Relative to TeXpose commit
`c83220a54071e73ee39494471c4b74b7c46d336e`, represent internal
control-sequence tokens as `Token::Command(&'input str)` instead of
`Token::Command(String)`. The tokenizer borrows the original UTF-8 source
slice, and the parser carries that lifetime without exposing token types or
borrowed data in the public AST.

The baseline allocated an owned `String` for every control word and
control symbol. The candidate uses source ranges for those token names.
Owning strings are still constructed where the AST or diagnostics must
outlive the input. The lexer still materializes a vector of tokens;
this is not streaming or a claim of allocation-free parsing.

## Experiment and measured evidence

- **Hypothesis:** eliminating per-command name allocation and subsequent
  name cloning lowers the cost of command-dense parsing.
- **Alternatives:** keep owned `String` command tokens, or carry borrowed
  `&str` tokens with a parser-scoped input lifetime.
- **Environment:** Windows `x86_64-pc-windows-msvc`, Cargo optimized `bench`
  profile, existing `benches/layout.rs` harness (`SAMPLE_COUNT=31`, calibrated
  batches targeting approximately 10 ms per sample).
- **Metric:** median nanoseconds per full `parse()` call, including AST
  construction. One before and one after benchmark invocation are recorded;
  the baseline medians were supplied in the earlier local benchmark report.

| Workload | Input construction (outside measurement) | Before median | After median | Reduction |
| --- | --- | ---: | ---: | ---: |
| Repeated commands | `r"\alpha+\beta+\gamma+\delta+\theta+\omega".repeat(64)` | 129,701.6 ns | 53,836.7 ns | 58.5% |
| Control symbols | `r"\,\;\!\ ".repeat(128)` | 85,775.0 ns | 22,409.2 ns | 73.9% |

The after-run distributions are part of the Windows verification log:

| Workload | Min (ns) | Median (ns) | p95 (ns) | Max (ns) |
| --- | ---: | ---: | ---: | ---: |
| Repeated commands | 52,396.9 | 53,836.7 | 93,129.7 | 94,546.9 |
| Control symbols | 22,365.6 | 22,409.2 | 23,068.4 | 24,210.5 |

The wide high tail of the repeated-commands candidate and single-run
comparison limit statistical confidence. This evidence supports the
specified command-heavy workloads; it does not establish a general
speedup for all inputs or a measured count of heap allocations.
The eliminated per-command `String` construction is a structural result
of source inspection, not an allocator-instrumentation measurement.

## Correctness and maintenance

The Windows candidate verification reported:

- `cargo fmt --all -- --check`: passed.
- `cargo test parser::tests::borrowed_commands`: 2 tests passed.
- `cargo test`: 228 unit tests passed, one ignored; integration tests
  passed; 40 doctests passed.
- `cargo clippy --all-targets -- -D warnings`: passed.
- `cargo +1.76.0 check`: passed.
- `git diff --check`: passed.

Permanent tests in `src/parser/tests/borrowed_commands.rs` establish that
control-word and control-symbol slices point into the exact source bytes.
The existing token-span tests and parser goldens protect original-source
UTF-8 spans, command whitespace behavior, public AST values, and structured
errors. The change introduces neither `unsafe` nor a new dependency.

After recording the decision, the two temporary F13 benchmark cases were
removed from `benches/layout.rs`; that file is restored to its preceding
content. Reopen this decision if performance on representative real-world
input regresses, source-lifetime constraints change, or independent
allocation profiling contradicts the expected mechanism. Measurements on
other hardware and direct allocation counts remain open evidence gaps.
