# Continuous integration

This document owns the Phase K CI contract. It does not define TeXpose layout semantics; those remain owned by the source contracts and the relevant domain documents.

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

That runner must provide `uv`, Cargo/Rust, LuaLaTeX, and the MiKTeX 26.5 reference environment recorded in `docs/ORACLE_PROFILES.md`. The label is only a scheduler constraint. It is not trusted as proof of identity: every contractual profile run reconstructs the reference fingerprint and fails unless its environment SHA-256 equals the pinned value in `tools/verify.py`.

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

For release-candidate evidence, success is tied to the workflow head SHA. A successful run for an older `main` revision does not cover a later commit, and a queued/cancelled oracle job is not evidence. `docs/RELEASE.md` owns the Phase M sequence and final candidate-evidence checklist.

## Dependency gate

The `dependencies` job in `.github/workflows/ci.yml` executes:

```text
cargo tree -e normal
cargo tree -e dev
cargo tree -e features
```

and uploads the three outputs as `dependency-trees`. These files are evidence of the graph resolved by that CI run. They are not treated as a lockfile and they do not replace manifest review.
