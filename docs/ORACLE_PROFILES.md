# Differential oracle profiles

This document records the durable Phase F contract for the LuaLaTeX differential
oracle. The oracle is independent evidence: `tools/verify.py` owns the selected
font file, copies it into an isolated workspace, and gives the same immutable
bytes and face index to the TeXpose probe and the LuaLaTeX reference.

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
the oracle. Repository CI provisioning of this pinned environment is still an
F10 release-gate task; the current evidence establishes the verifier-side pin,
not a claim that CI provisioning already exists.

## Profiles and corpus ownership

The named profiles are `stix`, `libertinus`, and `fira`. Each profile pins the
fixture path, SHA-256, face index, required corpus capabilities, canonical and
stress census hashes, tolerances, bounded documented deviations, and reference
environment identity.

All three profiles currently use:

- canonical census: 21 measurements / 21 aliases;
- stress census: 89 measurements / 94 aliases;
- canonical tolerance: `0.050em`;
- stress diagnostic tolerance: `0.050em`.

`--tolerance` cannot override a named profile during `--fail-on-delta`. Missing,
duplicate, unexpected, or silently skipped cases are evidence failures rather
than tolerance events.

## Canonical evidence

Measured on the reference environment above:

| Profile | Raw cases <= 0.050em | p50 | p90 | p95 | p99 | max |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| STIX Two Math | 21/21 | 0.000011 | 0.010000 | 0.013287 | 0.020001 | 0.020001 |
| Libertinus Math | 17/21 | 0.000013 | 0.081000 | 0.081001 | 0.081001 | 0.081001 |
| Fira Math | 20/21 | 0.000011 | 0.040000 | 0.040001 | 0.127001 | 0.127001 |

The raw count remains visible even when a bounded deviation is documented. A
profile exemption therefore cannot turn `17/21` into a misleading `21/21`.

### Bounded canonical deviations

These entries freeze existing cross-engine differences so Phase F can detect new
regressions while Phase G owns the mathematical repair. They are not assertions
that TeXpose is already correct.

| Profile | Case | Observed maximum | Contract ceiling | Phase G owner |
| --- | --- | ---: | ---: | --- |
| Libertinus | `display-nested-fraction` | 0.062386em | 0.062500em | fraction geometry |
| Libertinus | `radical-index` | 0.081000em | 0.081100em | radical geometry |
| Libertinus | `radical-index-compound` | 0.081001em | 0.081100em | radical geometry |
| Libertinus | `radical-plain` | 0.081001em | 0.081100em | radical geometry |
| Fira | `accent-widehat-script` | 0.127001em | 0.127100em | script-style accent geometry |

No canonical structural mismatch is documented for these cases. A new glyph- or
rule-count mismatch still fails. A documented geometry case also fails if it
exceeds its ceiling. If a contractual canonical run no longer observes a listed
deviation, the verifier fails and requires the profile to remove the stale
exemption rather than carrying historical allowance indefinitely.

## Stress evidence

Stress remains diagnostic evidence for primitive investigation, nightly runs,
and release candidates. The measured state is intentionally not normalized into
large profile exemptions:

| Profile | Raw cases <= 0.050em | p50 | p90 | p95 | p99 | max | structural mismatches |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| STIX Two Math | 62/89 | 0.000045 | 0.325402 | 0.489101 | 0.838115 | 0.838115 | 12 |
| Libertinus Math | 62/89 | 0.000026 | 0.224717 | 0.447999 | 0.591483 | 0.591483 | 7 |
| Fira Math | 68/89 | 0.000013 | 0.196001 | 0.405520 | 0.677298 | 0.677298 | 5 |

The large stress deltas are work inventory for Phase G, not justification to
raise the global tolerance. In particular, 6pt indexed radicals, brace/underbrace
construction, matrices/aligned material, and composite-script geometry remain
strong discriminators for later primitive fixes.

## Independent style sizes

TeXpose no longer supplies script or scriptscript percentages to the reference.
LuaLaTeX measures textstyle, scriptstyle, and scriptscriptstyle font sizes from
its own final math lists. The verifier requires positive monotonic sizes with a
real text-to-script transition. Equality between script and scriptscript is
allowed because the reference engine can clamp both to the same physical floor;
MiKTeX 26.5 observed `6pt -> 5pt -> 5pt` for the 6pt stress sweep.

## Collection-face identity

The generic oracle accepts `--font` and `--face-index`. TeXpose parses exactly
that face index. LuaLaTeX receives the corresponding `FontIndex`; for TTC/OTC
runs the verifier requires LuaTeX `subfont == face_index + 1`.

Glyph identity is deliberately not used as the collection-face proof. OpenType
`ssty` can substitute a different glyph from the same face: STIX Two Math maps
the text-style italic-x glyph `3354` through `ssty` alternates including `4699`,
which was observed in the 6pt reference control. Treating that substitution as a
face mismatch would be a false failure.

## Positioned primitive trace evidence

F8 adds an exact positioned trace on both sides. TeXpose flattens the `MathBox`
tree into glyph/rule primitives in exact `Dim` coordinates. LuaTeX walks the
final hlist/vlist, resolves the selected glyph index, and reports coordinates in
scaled points normalized by the independently measured root math em. Paint order
is the trace index. A primitive-count or primitive-kind difference is a kind-topology
mismatch. Matching kinds alone do not establish glyph correspondence: paint-order
and glyph-selection differences remain first-class trace evidence. Exact paint-index
geometry is compared when glyph identity/order agrees. A glyph-only pure reorder can
also compare x/baseline/scale after realigning unique glyph IDs; the paint-order
mismatch itself remains contractual. Selection changes, duplicate glyph IDs, and
reordered mixed glyph/rule traces remain non-comparable rather than inventing a
coordinate or rule-rectangle pairing.

The 2026-10-02 canonical measurement on the pinned MiKTeX reference environment
produced:

| Profile | Kind topology | Glyph identity/order aligned | p50 | p90 | p95 | p99 | max |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| STIX Two Math | 21/21 | 14/21 | 0.000012 | 0.010000 | 0.014000 | 0.014000 | 0.014000 |
| Libertinus Math | 21/21 | 14/21 | 0.000013 | 0.081000 | 0.081000 | 0.081000 | 0.081000 |
| Fira Math | 21/21 | 14/21 | 0.000012 | 0.040000 | 0.040000 | 0.040000 | 0.040000 |

The table percentile/max columns are the original identity/order-aligned measurement.
The current verifier additionally reports realigned glyph geometry for a pure reorder
only when every primitive is a glyph and every glyph ID is unique. Those cases enter
the runtime geometry tolerance gate without erasing their paint-order mismatch. A
selection change, duplicate glyph ID, or reordered mixed glyph/rule trace contributes
no fabricated numeric geometry. Geometry ambiguity by itself does not fail a canonical
profile whose exact mismatch signature is already documented; a new, repaired, or
changed signature still fails. The exact mismatch signatures remain contractual. A
diagnostic glyph pair is always written as `TeXpose/reference`.

The same seven canonical cases are currently not glyph identity/order aligned for
all three profiles:

```text
display-sum-limits
display-sum
accent-hat-j
accent-widehat-j
accent-widehat-xyz
accent-widetilde-xyz
accent-widehat-script
```

These are Phase G work inventory, not assertions that either paint order or glyph
selection is already correct. Contractual canonical runs pin the exact typed mismatch
signature for each profile as `(paint index, TeXpose glyph ID, reference glyph
ID)`. A new case, repaired case, added/removed mismatch within an existing case,
or changed glyph pair fails until the profile baseline is reviewed explicitly.
The three measured profiles currently contain 18 glyph/order mismatch positions
across the same seven case names. Under those exact signatures, STIX and Libertinus
each have four cases with the same glyph multiset in a different paint order and
three cases where glyph selection differs; Fira has seven paint-order-only cases.
This is intentionally more discriminating than a case-name allowlist while avoiding
an opaque whole-trace hash.

For every geometry-comparable canonical trace the normal `0.050em` tolerance
applies. A pure reorder is geometry-comparable only under the strict glyph-only,
unique-ID rule above. A positioned ceiling may overlap a documented glyph/order
mismatch only when the runtime comparison proves such an identity-realigned reorder.
Selection changes and ambiguous reorders remain non-comparable, cannot consume a
ceiling, and therefore make any ceiling on that case fail as stale.
Libertinus has these bounded positioned deviations measured before Phase G:

| Case | Observed maximum | Contract ceiling |
| --- | ---: | ---: |
| `display-nested-fraction` | 0.062399em | 0.062500em |
| `radical-index` | 0.081000em | 0.081100em |
| `radical-index-compound` | 0.081000em | 0.081100em |
| `radical-plain` | 0.081000em | 0.081100em |
| `text-scripts` | 0.080999em | 0.081100em |

Fira has one bounded positioned deviation exposed by identity realignment:

| Case | Observed maximum | Contract ceiling | Rationale |
| --- | ---: | ---: | --- |
| `accent-widehat-script` | 0.127001em | 0.127100em | Same script-style accent divergence already bounded in aggregate canonical geometry; Phase G owns the repair. |

A ceiling that is exceeded fails. A ceiling that is no longer needed also fails
as stale, forcing its removal instead of preserving historical tolerance.

Stress remains diagnostic and deliberately has no large positioned-trace
allowlist. The same measurement produced:

| Profile | Kind topology | Glyph identity/order aligned | p50 | p90 | p95 | p99 | max |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| STIX Two Math | 75/89 | 46/75 | 0.000020 | 0.145026 | 0.299701 | 0.781701 | 0.781701 |
| Libertinus Math | 80/89 | 60/80 | 0.000047 | 0.167500 | 0.390833 | 0.822900 | 0.822900 |
| Fira Math | 82/89 | 60/82 | 0.000041 | 0.128500 | 0.422560 | 0.935500 | 0.935500 |

The large stress topology, identity, and placement deltas remain discriminating
evidence for the primitive-by-primitive repairs in Phase G.

## Evidence parser hardening

F9 `self-test` owns the machine-evidence grammar and exact canonical mismatch signatures. It rejects malformed,
duplicate, missing, unexpected, and non-finite records; invalid exact `Dim` and
LuaTeX dimensions; wrong font hash, face, profile, census, or alias census; an
unknown pinned reference fingerprint; truncated or duplicate Lua results; and
invalid/nonsequential positioned traces. The positioned comparator regression also
proves that multiple glyph/order mismatches in one case are all retained, that pure
paint-order differences are distinguished from glyph-selection differences, that
glyph-only unique-ID reorders are realigned by glyph identity, and that duplicate-ID
or mixed glyph/rule reorders remain non-comparable instead of fabricating geometry.

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
does not acquire a blanket allowlist merely to make the current Phase G backlog
green.

The verifier-side environment identity is pinned, but repository CI provisioning
of the roadmap-required pinned TeX Live reference environment is still open. The
current Windows evidence is MiKTeX 26.5; it must not be represented as completion
of that TeX Live/CI requirement.

## Contractual commands

Canonical profile runs are:

```powershell
uv run --script tools\verify.py self-test
uv run --script tools\verify.py math --profile stix --fail-on-delta
uv run --script tools\verify.py math --profile libertinus --fail-on-delta
uv run --script tools\verify.py math --profile fira --fail-on-delta
```

Stress can be run with `--stress`; its current deltas are intentionally left
visible rather than blanket-approved. Positioned traces are now part of the
canonical contractual gate as described above, while stress remains diagnostic.
