# F1: aligned leading-Ord spacing without measurement relayout

## Context and decision

- Baseline: `beb8cc013671de828841f1f20c786aa1558b0da6` (after F13).
- Scope: right-hand fields (odd-numbered columns) in the `aligned` math environment.
- Decision: keep the existing first layout of the cell, but compute the width
  introduced by the implicit empty `Ord` directly from normalized TeX noad
  classes and glue. Do not lay out a cloned, prefixed syntax tree to measure it.

In `aligned`, the right field behaves as though an empty ordinary noad precedes
its content. This is not simply an `Ord`-to-first-noad kern: the initial `Ord`
can also preserve a leading binary operator as `Bin` instead of normalizing it
to `Ord`. Explicit glue, control nodes, and style declarations can occur before
the first effective noad. A shortcut based only on the first syntax node is
therefore insufficient.

## Alternatives and invariant

The previous conservative implementation cloned the field AST, prepended an
empty `Ord`, laid out the prefixed field, and subtracted the already measured
bare width. It restored the numbering cursor and truncated extra diagnostics,
but still duplicated the layout traversal and its intermediate effects.

The definitive implementation reuses the same semantic normalization used by
row layout. It compares noad spacing totals for the ordinary sequence and for
the sequence with an implicit preceding `Ord`, including the resulting binary
reclassification, and adds only their nonnegative width difference to the
already laid-out field. Both totals follow style changes and TeX's
`atom_space_mu` / `space_width` rules. The implicit `Ord` has no glyph or other
geometry of its own.

This calculation does not clone the cell AST or invoke a second `layout` pass.
That is a structural claim from the implementation, **not** a measured runtime
speedup. No F1 performance benchmark is claimed.

## Verification of the candidate

On Windows, after the follow-up one-line caller-arity correction:

- `cargo fmt --all -- --check`: passed.
- Focused normalization test for the implicit versus explicit `Ord`: passed.
- Focused `aligned` geometry test against an explicit empty `Ord`, including
  ten leading-expression forms at 10pt and 20pt: passed.
- `cargo test`: 230 unit tests passed, 1 ignored; all integration tests and
  40 doctests passed.
- `cargo clippy --all-targets -- -D warnings`: passed.
- `cargo +1.76.0 check`: passed.
- `git diff --check`: no whitespace errors (Git reported LF/CRLF conversion
  notices on Windows).
- `uv run --script tools/verify.py self-test`: passed.
- `uv run --script tools/verify.py math --profile stix --fail-on-delta`:
  completed with 24/25 cases within 0.050em and one approved exception.
- Corresponding `libertinus` profile: 24/25, one approved exception.
- Corresponding `fira` profile: 25/25, no approved exceptions.

The approved STIX and Libertinus deviations concern `accent-widehat-j`, not
`aligned`. The oracle's `display-aligned` width discrepancy remained small
(0.000014em STIX, 0.000016em Libertinus, 0.000013em Fira). This supports the
exercised rendering contract; it is not an exhaustive proof for all syntax.

## Retention and reopening

Keep the production semantic implementation and its permanent regressions.
The old AST cloning / prefixed measurement route and its side-effect rollback
are not retained as an alternate backend or live experimental probe.

Reopen the decision if a new TeX noad class, style/control transition, or
alignment cell shape demonstrates different geometry between an implicit
leading `Ord` and a genuine explicit empty `Ord`, or if profiling identifies
material overhead in the semantic spacing pass.
