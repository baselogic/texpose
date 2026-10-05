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
the oracle. The current evidence establishes the verifier-side environment pin.
Repository CI provisioning of that pinned reference environment is owned by
Phase K3, not Phase F10.

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
| STIX Two Math | 20/21 | 0.000011 | 0.001001 | 0.015001 | 0.079999 | 0.079999 |
| Libertinus Math | 20/21 | 0.000006 | 0.000014 | 0.001001 | 0.056999 | 0.056999 |
| Fira Math | 21/21 | 0.000001 | 0.000013 | 0.000013 | 0.035988 | 0.035988 |

The raw count remains visible even when a bounded deviation is documented. A
profile exemption therefore cannot turn an out-of-tolerance raw case into a
misleading all-green count.

### Bounded canonical deviations

These entries freeze bounded cross-engine differences so the contractual oracle can
detect new regressions without overriding stronger external evidence. The remaining
entries are the G5 `accent-widehat-j` reference-engine differences: OpenType MATH
defines horizontal variant extent by `MathGlyphVariantRecord.advanceMeasurement`,
and the pinned fonts expose a combining-circumflex construction that the current
LuaLaTeX reference does not select for this one-character case. Those ceilings record
the resulting reference-engine divergence; they do not weaken TeXpose's MATH variant
selection contract. G9 removed the former Libertinus `display-nested-fraction`
geometry waiver after the measured delta fell below canonical tolerance.

| Profile | Case | Observed maximum | Contract ceiling | Phase G owner |
| --- | --- | ---: | ---: | --- |
| STIX | `accent-widehat-j` | 0.079999em | 0.080100em | G5 horizontal accent variants |
| Libertinus | `accent-widehat-j` | 0.056999em | 0.057100em | G5 horizontal accent variants |

No canonical structural mismatch is documented for these cases. A new glyph- or
rule-count mismatch still fails. A documented geometry case also fails if it
exceeds its ceiling. If a contractual canonical run no longer observes a listed
deviation, the verifier fails and requires the profile to remove the stale
exemption rather than carrying historical allowance indefinitely.

## Stress evidence

Stress remains diagnostic evidence for primitive investigation, nightly runs,
and release candidates. The table below is the pre-G9 whole-corpus snapshot; it is
kept as investigation context rather than presented as the current post-G9 fraction
measurement. G12 owns the next complete stress-baseline refresh. The measured state
is intentionally not normalized into large profile exemptions:

| Profile | Raw cases <= 0.050em | p50 | p90 | p95 | p99 | max | structural mismatches |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| STIX Two Math | 67/89 | 0.000020 | 0.304690 | 0.349773 | 0.828115 | 0.828115 | 2 |
| Libertinus Math | 68/89 | 0.000014 | 0.126376 | 0.167502 | 0.390829 | 0.390829 | 2 |
| Fira Math | 75/89 | 0.000012 | 0.119971 | 0.134397 | 0.422560 | 0.422560 | 2 |

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

### G9 physical-size fraction sweep

G9 keeps TeXpose script scaling owned by the selected font's OpenType MATH
`scriptPercentScaleDown` and `scriptScriptPercentScaleDown` values, as established
by G6. The OpenType `ssty` contract likewise assumes those MATH percentages are the
scaling factors applied by the math engine. Root-em size therefore changes physical
`pt`/`bp` quantities such as null-delimiter spacing, but does not replace the font's
style-scale ratios inside TeXpose. Focused fraction tests protect that distinction at
6pt, 10pt, 20pt, and 40pt, including nested fractions.

The post-G9 10pt fraction geometry is effectively at numeric noise in all three
profiles: the affected stress fraction family maxima are `0.000013em` (STIX),
`0.000016em` (Libertinus), and `0.000012em` (Fira). Canonical
`display-nested-fraction` is likewise `0.000013em`, `0.000014em`, and `0.000012em`
respectively.

The remaining physical-size sweep differences are reference-size-policy evidence,
not a reason to replace TeXpose's MATH scaling contract. At 6pt the pinned reference
clamps script and scriptscript to 5pt. At 20pt and 40pt, the positioned trace reports
`0.100003em` glyph-scale deltas for Libertinus nested fractions and `0.020003em` for
Fira; STIX has selection differences that make those traces non-comparable. These
size-sweep cases remain explicit G12 profile-classification inventory. G9 adds no
stress waiver or global tolerance increase for them.

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

The 2026-10-04 canonical measurement on the pinned MiKTeX reference environment
produced:

| Profile | Kind topology | Glyph identity/order aligned | Geometry comparable | p50 | p90 | p95 | p99 | max |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| STIX Two Math | 21/21 | 14/21 | 16/21 | 0.000011 | 0.001500 | 0.014000 | 0.014000 | 0.014000 |
| Libertinus Math | 21/21 | 14/21 | 16/21 | 0.000011 | 0.008799 | 0.062399 | 0.062399 | 0.062399 |
| Fira Math | 21/21 | 14/21 | 20/21 | 0.000001 | 0.000013 | 0.007199 | 0.036000 | 0.036000 |

The percentile/max columns cover geometry-comparable traces after the verifier's
strict unique-glyph realignment for pure reorders. Realignment does not erase the
paint-order mismatch itself. A
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
STIX, Libertinus, and Fira now contain 20 glyph/order mismatch positions each
across the same seven case names. Under those exact signatures,
STIX has two paint-order-only cases and five glyph-selection cases; Libertinus has
three paint-order-only cases and four glyph-selection cases; Fira has seven
paint-order-only cases.
This is intentionally more discriminating than a case-name allowlist while avoiding
an opaque whole-trace hash.

For every geometry-comparable canonical trace the normal `0.050em` tolerance
applies. A pure reorder is geometry-comparable only under the strict glyph-only,
unique-ID rule above. A positioned ceiling may overlap a documented glyph/order
mismatch only when the runtime comparison proves such an identity-realigned reorder.
Selection changes and ambiguous reorders remain non-comparable, cannot consume a
ceiling, and therefore make any ceiling on that case fail as stale.
Libertinus and Fira have no bounded positioned deviation in the current canonical
measurement. G9 removed the former Libertinus `display-nested-fraction` ceiling after
the identity-aligned maximum fell to `0.000014em`. The former Fira
`accent-widehat-script` ceiling was already removed because G5 reduced the
identity-realigned maximum to `0.040000em`; retaining either exception would make the
profile stale.

A ceiling that is exceeded fails. A ceiling that is no longer needed also fails
as stale, forcing its removal instead of preserving historical tolerance.

Stress remains diagnostic and deliberately has no large positioned-trace
allowlist. The same measurement produced:

| Profile | Kind topology | Glyph identity/order aligned | Geometry comparable | p50 | p90 | p95 | p99 | max |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| STIX Two Math | 84/89 | 52/84 | 56/84 | 0.000012 | 0.349772 | 1.032257 | 1.032257 | 1.032257 |
| Libertinus Math | 84/89 | 62/84 | 66/84 | 0.000013 | 0.137598 | 0.190000 | 0.390833 | 0.390833 |
| Fira Math | 84/89 | 62/84 | 72/84 | 0.000012 | 0.127000 | 0.134398 | 0.422560 | 0.422560 |

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

## Goldens versus the external oracle

The repository gold files and the LuaLaTeX oracle have different authority.
Goldens are deterministic regression contracts for TeXpose on the pinned font
fixtures. They detect unintended changes precisely, but a stored `w/h/d` value is
not independent evidence that the layout matches TeX, LuaTeX, or OpenType MATH.
External compatibility requires the applicable TeX/OpenType contract plus genuinely
independent evidence such as the pinned LuaLaTeX reference.

Do not resolve an oracle disagreement by regenerating a golden from current
TeXpose output, and do not treat LuaLaTeX as stronger than an applicable external
specification. First identify the owning rule and evidence: font facts from the
pinned font, TeX/OpenType layout semantics, and the pinned LuaLaTeX reference where
applicable. A deliberately retained divergence may remain a regression golden, but
it must not be described as proof of external correctness.

The generated LuaLaTeX probe deliberately does not load `microtype` and contains a
hard failure if `microtype` is already loaded after begin-document hooks. The oracle
measures natural math boxes; microtypographic protrusion/expansion is outside that
contract and must not become an ambient input. This exclusion is part of profile
protocol `oracle-v9`.

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

The verifier-side environment identity is pinned. Repository CI provisioning of
the pinned reference environment is a Phase K3 task. The current Windows evidence
is MiKTeX 26.5 and must not be represented as completion of K3.

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
