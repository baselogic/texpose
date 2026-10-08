# Mathematical-core release gates and release record

TeXpose currently has `publish = false`; this document defines mathematical-core
release-candidate verification and durable release documentation. It does not
grant authority to publish, tag, sign, push a release object, or change package
publication settings.

The release candidate is a repository revision, not a prose claim. All required
evidence must refer to the same final candidate revision after the last semantic
or contractual mutation. Do not edit tracked source merely to paste a final
commit SHA or workflow run ID after verification; that would create a different
revision and invalidate the evidence. Candidate identity is taken from the
selected Git/GitHub revision and the head SHA recorded by each workflow run and
artifact.

## Authoritative contract index

Phase M2 requires the release documentation to record the following contracts.
This table is the release index; the linked domain document remains the semantic
authority so the same rule is not reimplemented in multiple prose files.

| Release record | Authoritative source | Mathematical-core contract |
| --- | --- | --- |
| Supported syntax | `docs/SYNTAX.md` | Explicit TeX/LaTeX math subset; undocumented incidental parser acceptance is not a compatibility promise. |
| Parser API | `docs/PARSER_API.md` | `parse` / `parse_with_options`, typed spans/errors/options, supported public AST. |
| TeX/LaTeX semantic scope | `docs/SYNTAX.md`, `docs/LAYOUT.md` | TeX-style noad/layout semantics plus explicitly supported LaTeX/amsmath constructs. |
| Numeric/unit contract | `docs/COMPATIBILITY.md`, `docs/LAYOUT.md` | Exact internal `Dim`; distinct `em`, `mu`, TeX `pt`, and `bp`; positive physical root em; root-em public coordinates. |
| Font contract | `docs/FONT_API.md` | Caller-owned immutable bytes, explicit face index, validated static MATH face, stable read-only identity. |
| Static-font policy | `docs/COMPATIBILITY.md`, `docs/FONT_API.md` | One static math face per formula; functional variable faces rejected. |
| OpenType MATH coverage | `docs/MATH_COVERAGE.md` | Per-field owner/status/test and explicit unsupported/degradation policy. |
| Literal-text contract | `docs/SYNTAX.md`, `docs/COMPATIBILITY.md` | Same math face, direct scalar mapping, no shaping/kerning/bidi/ligature/font fallback contract. |
| Graceful degradation | `docs/COMPATIBILITY.md`, `docs/LAYOUT.md` | Missing Unicode glyph and unusable extensible assembly are deterministic, diagnostic-bearing recoveries; other unsupported/invalid states fail typed. |
| Display-list contract | `docs/LAYOUT.md` | Font-bound, paint-ordered, root-em-normalized `MathLayout`; exact geometry flattened once before binary32 conversion. |
| MSRV | `Cargo.toml`, `docs/CI.md` | Rust 1.76 / Edition 2021 until separately reviewed. |
| Normal dependencies | `Cargo.toml`, this file | One direct normal dependency: `ttf-parser` with defaults disabled and only required features. |
| Oracle reference fingerprint | `docs/ORACLE_PROFILES.md`, `tools/verify.py` | Pinned MiKTeX/LuaHBTeX/LaTeX package identity and environment hash. |
| Oracle font profiles | `docs/ORACLE_PROFILES.md` | Canonical/stress profiles `stix`, `libertinus`, `fira`; exact font hashes/face indexes/corpora. |
| Stress results and deviations | `docs/ORACLE_PROFILES.md`, `docs/G12_STRESS_CLOSURE.md` | Ratified stale-sensitive stress evidence; no blanket tolerance increase. |
| Performance baseline | `docs/PERFORMANCE.md`, `docs/FONT_ACCESS.md` | Maintained benchmark surface, measured local baseline, durable optimization decisions/reopen conditions. |
| CI/release execution | `docs/CI.md`, this file | Core/MSRV matrix, canonical oracle, stress oracle, dependency inventory, final-state review. |

## Compatibility policy frozen for the first stable core

Phase L4 is represented by `docs/COMPATIBILITY.md`. The release candidate must
preserve all of these cross-cutting policies unless a separately reviewed change
updates the owning implementation/tests and release evidence:

```text
one static math face per formula
literal text is not a general shaping/fallback engine
em/mu remain style-relative
TeX pt/bp remain physical until root-em resolution
MATH Device/VariationIndex corrections are ignored
functional variable-font faces are rejected
recoverable degradation is narrow, deterministic, and diagnostic-bearing
canonical differential profiles are stix/libertinus/fira on the pinned environment
```

The compatibility document intentionally distinguishes policy from mechanism.
Detailed syntax, geometry, font-table ownership, and oracle deviation records
remain in their domain documents.

## Normal dependency inventory

The production manifest has one direct normal dependency:

| Crate | Requirement | Default features | Enabled features | Purpose |
| --- | --- | --- | --- | --- |
| `ttf-parser` | `0.25` | disabled | `std`, `opentype-layout` | OpenType face parsing, metrics, MATH and layout tables |

The root crate intentionally has no normal dependency on the fuzz package or
oracle tooling. Because the library does not commit a root `Cargo.lock`, an exact
resolved patch version is build evidence rather than a permanent source
invariant. Every candidate therefore retains the `cargo tree -e normal`,
`cargo tree -e dev`, and `cargo tree -e features` outputs from the dependency
gate. Any new normal dependency or transitive normal node requires explicit
review and an update to this inventory/rationale.

## Pinned external-reference identity

The contractual oracle environment is owned by `docs/ORACLE_PROFILES.md`. The
current reference fingerprint is:

| Component | Contractual value |
| --- | --- |
| Engine | `This is LuaHBTeX, Version 1.25.7 (MiKTeX 26.5)` |
| Distribution | `MiKTeX 26.5` |
| LaTeX format | `2026-06-01` |
| `unicode-math` | `2023/08/13 v0.8r Unicode maths in XeLaTeX and LuaLaTeX` |
| `fontspec` | `2025/09/29 v2.9g Font selection for XeLaTeX and LuaLaTeX` |
| `amsmath` | `2026/05/19 v2.18d AMS math features` |
| Environment SHA-256 | `b621bc874d9749432eca9f8a66a8bc8ffd72afda0ef8de8624f6a2c2171acbdf` |

A candidate run that reconstructs a different environment hash fails. Updating
any component above is an oracle-baseline change, not routine tool bootstrap.

## Current ratified oracle baseline

`docs/ORACLE_PROFILES.md` owns the raw canonical/stress measurement summaries,
while `docs/G12_STRESS_CLOSURE.md` owns the exact stress-only deviation evidence
and reopening conditions. This release record deliberately does not copy those
case counts, ceilings, or glyph signatures into a second authority.

For release-candidate purposes, the required stress result is unambiguous: all
three named profiles must pass `--stress --fail-on-delta` on the final candidate
revision, and the resulting `stress-oracle-evidence` artifact must identify that
same head SHA. The verifier keeps raw deltas visible, applies only the exact
stale-sensitive records owned by the oracle documents, and rejects every other
unbounded structural, glyph, or geometry difference. Historical G12 evidence
cannot substitute for the final candidate run.

## Performance baseline

`docs/PERFORMANCE.md` owns the maintained benchmark contract and the latest
recorded local measurement snapshot (2026-10-07). It covers font construction,
face parsing, parser end-to-end cost, simple/complex/stress layout, and the
scripts/fractions/radicals/delimiters/assembly/large-operator/matrix/aligned
primitive workloads.

The snapshot is not a cross-machine latency threshold because the transcript did
not capture the complete CPU/toolchain/target identity. Its release value is the
workload census and the durable decisions it supports: no persistent font lookup
index, no speculative layout cache/special-case redesign, and no performance API
added without a causal profile and paired measurement. A material optimization
claim requires the reopen conditions in `docs/PERFORMANCE.md` and
`docs/FONT_ACCESS.md`.

## Phase M1 release-verification sequence

Run this sequence only after the last semantic or contractual mutation intended
for the candidate. These commands are verification; they do not mutate source.
A failure blocks release-candidate status until its cause is repaired and the
applicable evidence is rerun on the repaired revision.

Core Rust gate:

```powershell
cargo fmt --check
cargo test
cargo clippy --all-targets -- -D warnings
```

MSRV gate:

```powershell
cargo +1.76.0 check
cargo +1.76.0 test
```

Verifier integrity plus canonical profiles:

```powershell
uv run --script tools\verify.py self-test
uv run --script tools\verify.py math --profile stix --fail-on-delta
uv run --script tools\verify.py math --profile libertinus --fail-on-delta
uv run --script tools\verify.py math --profile fira --fail-on-delta
```

Stress profiles:

```powershell
uv run --script tools\verify.py math --profile stix --stress --fail-on-delta
uv run --script tools\verify.py math --profile libertinus --stress --fail-on-delta
uv run --script tools\verify.py math --profile fira --stress --fail-on-delta
```

Dependency inventory:

```powershell
cargo tree -e normal
cargo tree -e dev
cargo tree -e features
```

The repository's MSRV CI matrix remains authoritative for cross-platform Rust
1.76 evidence. A local MSRV run is useful focused evidence but does not replace
that matrix.

## GitHub candidate evidence

A release candidate requires successful evidence for the same final head SHA:

| Evidence | Workflow/job | Required result/artifact |
| --- | --- | --- |
| Stable core | `Core CI` stable matrix | Windows/Linux/macOS `fmt`, tests, Clippy green |
| MSRV | `Core CI` Rust 1.76 matrix | Windows/Linux/macOS check/tests green |
| Dependency inventory | `Core CI` dependency gate | `dependency-trees` artifact |
| Canonical external oracle | `Canonical oracle` | `canonical-oracle-evidence` artifact; all three profiles green |
| Stress external oracle | `Stress oracle` | `stress-oracle-evidence` artifact; all three profiles green |

For trusted direct-`main` development, all three workflows support manual
dispatch. The self-hosted oracle runner must be online for canonical/stress jobs.
A queued job is not evidence. A cancelled job is not evidence. A successful run
for an earlier SHA is not evidence for the final candidate.

Artifacts must identify the workflow head SHA and remain unexpired while they
are being used as release evidence. Oracle JSON must retain the exact profile,
font hash, face index, corpus census, reference fingerprint, and mismatch summary
produced by `tools/verify.py`. Dependency trees describe the graph actually
resolved by that run and do not replace manifest review.

## Final-state review

After all commands and workflows above pass, inspect the final candidate without
changing it:

```powershell
git status --short
git diff HEAD^..HEAD --check
git log -1 --oneline
cargo tree -e normal
cargo tree -e features
```

`git status --short` must be empty. Review the final diff/worktree after the
last semantic mutation, as required by Phase M1. If finalization changes source,
configuration, profile data, dependencies, oracle baselines, or any other
semantic/contractual input, the previous release evidence is stale and the
applicable gates must run again.

A documentation-only correction after M1 is still a new revision. Whether it
requires the external oracle depends on whether it changes the mathematical
contract, but the final candidate identity and core verification must describe
the actual final revision.

## Phase M2 release-documentation acceptance

Before calling the mathematical core a release candidate, review this checklist
against the final revision:

- [ ] `docs/SYNTAX.md` reflects the supported syntax and unsupported forms.
- [ ] `docs/PARSER_API.md` reflects the stable parser boundary.
- [ ] `docs/COMPATIBILITY.md` reflects the single-face, literal-text, unit,
      Device-correction, variable-font, degradation, and oracle-profile policy.
- [ ] `docs/FONT_API.md` reflects shared bytes, face index, units-per-em, typed
      font errors, and renderer face identity.
- [ ] `docs/MATH_COVERAGE.md` reflects current OpenType MATH field ownership,
      status, tests, and degradation.
- [ ] `docs/LAYOUT.md` reflects the public `MathLayout`/`MathOp` coordinate,
      paint-order, float-conversion, color, diagnostics, and numbering contract.
- [ ] `Cargo.toml` and `docs/CI.md` agree on Rust 1.76 MSRV.
- [ ] the normal dependency inventory above matches `Cargo.toml` and the final
      dependency-tree evidence.
- [ ] `docs/ORACLE_PROFILES.md` matches the verifier's pinned reference
      fingerprint and the `stix`/`libertinus`/`fira` profile identities.
- [ ] `docs/G12_STRESS_CLOSURE.md` still describes every ratified stress-only
      deviation/reopening condition used by the verifier.
- [ ] `docs/PERFORMANCE.md` and `docs/FONT_ACCESS.md` reflect the maintained
      benchmark surface, baseline limits, adopted decisions, and reopen
      conditions.
- [ ] Core CI, MSRV, canonical oracle, stress oracle, and dependency artifacts
      all refer to the final candidate revision.
- [ ] no known critical defect is being hidden by a golden, tolerance,
      diagnostic fallback, or successful internal-only test.

## Publication boundary

Passing this document's gates establishes evidence for a mathematical-core
release candidate. It does not authorize publication. Publishing, tagging,
signing, changing `publish = false`, creating a GitHub release, or otherwise
making an irreversible/shared release action requires separate explicit
authority.
