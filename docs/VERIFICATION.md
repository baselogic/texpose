# Verification and release

CI and oracle workflows are authoritative for execution. This document records the required gate and release evidence, not a certification for arbitrary later commits.

## Core pull-request gate

`.github/workflows/ci.yml` runs the stable toolchain on Windows, Linux, and macOS. Every stable job executes:

```text
cargo fmt --check
cargo test
cargo clippy --all-targets -- -D warnings
```

The same workflow runs the MSRV matrix on Rust 1.76.0 across the same three operating systems:

```text
cargo check
cargo test
```

Rust 1.76 / Edition 2021 remains the repository contract until a separately reviewed toolchain change updates `Cargo.toml`, this document, and the CI matrix together.

## Pinned oracle runner

The canonical and stress oracle workflows do not bootstrap an arbitrary current TeX distribution. They require a self-hosted Windows x64 runner with the label:

```text
texpose-oracle-miktex-26-5
```

That runner must provide `uv`, Cargo/Rust, LuaLaTeX, and the MiKTeX 26.5 reference environment pinned by `tools/verify.py`. The label is only a scheduler constraint. It is not trusted as proof of identity: every contractual profile run reconstructs the reference fingerprint and fails unless its environment SHA-256 equals the pinned value in `tools/verify.py`.

Updating MiKTeX, LuaHBTeX, the LaTeX format, `unicode-math`, `fontspec`, or `amsmath` is an oracle-baseline change. It requires separate review and updated measured evidence. CI must not silently update those components as part of a normal pull request. The self-hosted runner should be dedicated/restricted and must not execute untrusted fork code.

## Canonical oracle

`.github/workflows/oracle.yml` runs for same-repository pull requests that touch code, mathematical data/golds, tests, oracle tooling, the oracle profile contract, or the workflow itself. It can also be launched manually. Fork pull requests are deliberately skipped on the self-hosted runner; after review, a maintainer must run the canonical workflow from trusted repository code before accepting a mathematical change from a fork.

It runs the verifier self-test and all three contractual canonical profiles with `--fail-on-delta`. Each successful profile writes one deterministic `texpose-oracle-evidence-v1` JSON file. The workflow uploads those files as the `canonical-oracle-evidence` artifact.

The JSON evidence records the profile/scope, font SHA-256 and face index, corpus revision and census hashes/counts, full reference fingerprint, outer-geometry summary/maxima, and positioned topology/glyph/geometry mismatches.

## Stress oracle

`.github/workflows/stress-oracle.yml` runs all three stress profiles nightly, on a GitHub prerelease event, and by manual dispatch. GitHub can report a prerelease published from a draft as `published`, so the workflow listens for both `prereleased` and `published` and filters the latter to `release.prerelease == true`. It uses `--stress --fail-on-delta` and uploads `texpose-oracle-evidence-v1` JSON as the `stress-oracle-evidence` artifact.

The scheduled time is 03:17 UTC. The exact clock time has no semantic meaning; the contract is one scheduled run per day.

A failed oracle run remains a failed gate. Artifact upload uses `if: always()` so evidence produced before another profile fails is retained, but `tools/verify.py` writes a profile JSON only after that profile passes its complete contractual checks. CI logs remain the failure evidence for a profile that aborts before successful evidence publication.

## Trusted direct-main release-candidate runs

All three workflows expose `workflow_dispatch`. A trusted `main` revision may therefore be verified without creating a release branch: dispatch `Core CI`, `Canonical oracle`, and `Stress oracle` against the exact candidate revision. The oracle jobs still require the pinned self-hosted runner.

For release-candidate evidence, success is tied to the workflow head SHA. A successful run for an older `main` revision does not cover a later commit, and a queued/cancelled oracle job is not evidence. Candidate publication requirements are below.

## Dependency gate

The `dependencies` job in `.github/workflows/ci.yml` executes:

```text
cargo tree -e normal
cargo tree -e dev
cargo tree -e features
```

and uploads the three outputs as `dependency-trees`. These files are evidence of the graph resolved by that CI run. They are not treated as a lockfile and they do not replace manifest review.

## Optional Windows font check

Windows installations with Cambria Math may run additional font/layout tests using `%WINDIR%\Fonts\cambria.ttc`. Resolve the `Cambria Math` face by name rather than assuming a collection index. Do not commit or redistribute the system font.

## Oracle reference and corpus

## Reference environment identity

The first measured contractual environment is the Windows MiKTeX installation
used on 2026-10-02:

| Component | Recorded value |
| --- | --- |
| engine | `This is LuaHBTeX, Version 1.25.7 (MiKTeX 26.5)` |
| distribution | `MiKTeX 26.5` |
| LaTeX format | `2026-06-01` |
| `unicode-math` | `2023/08/13 v0.8r Unicode maths in XeLaTeX and LuaLaTeX` |
| `fontspec` | `2025/09/29 v2.9g Font selection for XeLaTeX and LuaLaTeX` |
| `amsmath` | `2026/05/19 v2.18d AMS math features` |
| environment SHA-256 | `b621bc874d9749432eca9f8a66a8bc8ffd72afda0ef8de8624f6a2c2171acbdf` |

The environment hash covers those six version strings, not the selected font or
corpus identity. Font SHA-256, face index, profile revision, measurement census,
and alias census are validated independently on every run.

A named profile rejects a different reference-environment hash. This makes an
engine/package update a reviewed baseline change instead of silently changing
`texpose-oracle-miktex-26-5`. The label selects the intended snapshot; the
verifier still reconstructs and checks the full environment hash on every run,
so an incorrectly provisioned or drifted runner fails closed.


## Profiles and corpus ownership

The named profiles are `stix`, `libertinus`, and `fira`. Each profile pins the
fixture path, SHA-256, face index, required corpus capabilities, canonical and
stress census hashes, tolerances, bounded documented deviations, and reference
environment identity.

All three profiles currently use:

- canonical census: 25 measurements / 25 aliases;
- stress census: 93 measurements / 98 aliases;
- canonical tolerance: `0.050em`;
- stress diagnostic tolerance: `0.050em`.

`--tolerance` cannot override a named profile during `--fail-on-delta`. Missing,
duplicate, unexpected, or silently skipped cases are evidence failures rather
than tolerance events.


## Collection-face identity

The generic oracle accepts `--font` and `--face-index`. TeXpose parses exactly
that face index. LuaLaTeX receives the corresponding `FontIndex`; for TTC/OTC
runs the verifier requires LuaTeX `subfont == face_index + 1`.

Glyph identity is deliberately not used as the collection-face proof. OpenType
`ssty` can substitute a different glyph from the same face: STIX Two Math maps
the text-style italic-x glyph `3354` through `ssty` alternates including `4699`,
which was observed in the 6pt reference control. Treating that substitution as a
face mismatch would be a false failure.


## Gate separation

F10 keeps three evidence levels distinct:

Fast PR gate:

```powershell
cargo fmt --check
cargo test
cargo clippy --all-targets -- -D warnings
```

Canonical external gate for semantic math, font handling, layout, or oracle
changes:

```powershell
uv run --script tools\verify.py self-test
uv run --script tools\verify.py math --profile stix --fail-on-delta
uv run --script tools\verify.py math --profile libertinus --fail-on-delta
uv run --script tools\verify.py math --profile fira --fail-on-delta
```

Stress (`--stress`) remains investigation/nightly/release-candidate evidence and
does not acquire a blanket allowlist. `--stress --fail-on-delta` validates exact
ratified glyph signatures, bounded stress-only outer/positioned deviations, rejects
the explicit unresolved glyph inventory, and rejects every other unbounded
positioned/outer geometry difference. Stress-only deviations are not visible to
canonical runs.

runner described in `docs/VERIFICATION.md`. Contractual CI requests deterministic
`texpose-oracle-evidence-v1` JSON with `--evidence-json`; that artifact records the
reference fingerprint, font identity, corpus census, aggregate geometry and
positioned mismatch evidence without changing normal interactive verifier output.


## Contractual commands

Canonical profile runs are:

```powershell
uv run --script tools\verify.py self-test
uv run --script tools\verify.py math --profile stix --fail-on-delta
uv run --script tools\verify.py math --profile libertinus --fail-on-delta
uv run --script tools\verify.py math --profile fira --fail-on-delta
```

CI adds `--evidence-json <path>` to each named run and uploads the resulting JSON.
The option is explicit and non-default: ordinary verification remains
non-mutating.

Stress can be run with `--stress`; raw deltas remain visible rather than being
rewritten by deviations. Positioned traces are part of the canonical contractual
gate. A named stress run with `--fail-on-delta` enforces the pinned deviation inventory and rejects unresolved glyph selection or unbounded geometry.

## Fuzzing

`fuzz/` contains the maintained cargo-fuzz targets (`parser_tokenizer`, `parser_syntax`, `dim_arithmetic`, `dim_decimal`, `dim_units`). Run from `fuzz/` using `cargo +nightly fuzz run <target>`. Fuzzing is supplementary evidence; a clean run is not proof of absence of defects.

## Candidate publication boundary

A release candidate requires a clean reviewed tree, local format/test/Clippy checks, Rust 1.76 check and tests, verifier self-test, successful canonical and stress oracle runs for the exact candidate SHA, and a dependency graph review. Verify the CI results and uploaded artifacts against that SHA before publication. Changes after verification require new evidence when they affect the candidate. The crate remains unpublished until an explicit release decision.

## Verified historical candidate (2026-10-08)

Candidate `87a9de99562acbe9557f4a43a12eb81c02ba19f8` passed [Core CI](https://github.com/baselogic/texpose/actions/runs/37734754576), [Canonical oracle](https://github.com/baselogic/texpose/actions/runs/37734814143), and [Stress oracle](https://github.com/baselogic/texpose/actions/runs/37734826850). Artifacts: `dependency-trees`, `canonical-oracle-evidence`, `stress-oracle-evidence`. The artifacts were present and unexpired when checked on 2026-10-08; their ZIP contents were not independently audited in that check. This is evidence for that SHA only, not the subsequent documentation commits.
