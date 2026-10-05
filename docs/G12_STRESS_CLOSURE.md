# G12 composite stress closure

This file is the durable closure inventory for Phase G12. It records the pinned
MiKTeX 26.5 differential evidence after commit `1feb13e` (G12 stage 4, vertical
assembly paint order) and separates three different states that must not be
collapsed into one allowlist:

1. specification-owned differential deviations whose exact positioned glyph
   signature is ratified by an existing focused TeXpose contract;
2. unresolved positioned glyph-selection inventory, which remains blocking;
3. geometry above the `0.050em` stress tolerance, which remains blocking unless
   a later stage supplies the G12 deviation evidence tuple required by the
   roadmap.

The stress oracle remains diagnostic by default. `--stress --fail-on-delta` is
now a G12 closure check. It is intentionally not a stable green repository gate
while unresolved inventory remains.

## Governing contracts

G12 does not make LuaLaTeX output authoritative by observation alone. The
applicable ownership order is the project roadmap: TeX/LaTeX semantics,
OpenType MATH / GSUB font data, focused TeXpose contracts, then the pinned
LuaLaTeX observation as independent differential evidence.

Two already-ratified rules explain most remaining glyph-selection differences:

- **G5 horizontal accents**: prebuilt MATH variants are selected by
  `MathGlyphVariantRecord.advanceMeasurement`; `flac`, semantic accent-source
  selection, and MATH constructions remain active where applicable. The focused
  owner is
  `wide_accent_math::wide_accents_select_semantic_math_constructions_by_advance_across_fonts`
  plus the adjacent G5 accent tests. The canonical profiles already pin the
  corresponding one-case signatures.
- **G6 script sizes and `ssty`**: TeXpose uses the selected font's
  `scriptPercentScaleDown` / `scriptScriptPercentScaleDown` and semantic script
  nesting to choose `ssty` level 1 / 2. The focused owners are
  `script_style_alternates::script_scales_and_ssty_alternates_follow_each_real_font`,
  `script_style_alternates::actual_script_nesting_selects_level_one_then_level_two`,
  and
  `fraction_semantics::nested_fraction_keeps_math_script_scale_across_root_em_sizes_and_profiles`.
  The pinned LuaLaTeX environment independently uses its LaTeX math-size policy
  outside the 10pt declaration point; that difference is reference-size-policy
  evidence, not authority to replace the OpenType MATH percentages in TeXpose.

A ratified positioned stress signature does **not** approve outer geometry and
it does not approve any other primitive in the same formula. Exact signatures
are stored as `(paint index, TeXpose glyph ID, reference glyph ID)` and are
stale-sensitive: a repaired, added, removed, or changed pair requires review.

## Stage-4 measurement

Pinned reference fingerprint:

```text
engine:       LuaHBTeX 1.25.7 (MiKTeX 26.5)
distribution: MiKTeX 26.5
LaTeX:        2026-06-01
unicode-math: 2023/08/13 v0.8r
fontspec:     2025/09/29 v2.9g
amsmath:      2026/05/19 v2.18d
environment:  b621bc874d9749432eca9f8a66a8bc8ffd72afda0ef8de8624f6a2c2171acbdf
```

Outer geometry:

| Profile | <= 0.050em | p50 | p90 | p95 | p99 | max |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| STIX Two Math | 77/93 | 0.000012 | 0.261050 | 0.331992 | 0.828115 | 0.828115 |
| Libertinus Math | 79/93 | 0.000012 | 0.056999 | 0.166628 | 0.390829 | 0.390829 |
| Fira Math | 84/93 | 0.000011 | 0.039574 | 0.133440 | 0.422560 | 0.422560 |

Positioned traces:

| Profile | Kind topology | Glyph identity/order aligned | Geometry comparable | p50 | p90 | p95 | p99 | max |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| STIX Two Math | 93/93 | 72/93 | 72/93 | 0.000012 | 0.068988 | 0.138649 | 0.512941 | 0.512941 |
| Libertinus Math | 93/93 | 81/93 | 81/93 | 0.000012 | 0.050336 | 0.137598 | 0.390833 | 0.390833 |
| Fira Math | 93/93 | 92/93 | 92/93 | 0.000011 | 0.105500 | 0.134398 | 0.422560 | 0.422560 |

Stage 4 therefore leaves no kind-topology mismatch in any profile.

## Ratified positioned glyph deviations

### STIX Two Math

Eleven of the twenty-one observed mismatch cases are owned by an already-tested
G5 rule. The exact signatures live in `STIX_STRESS_TRACE_GLYPH_DEVIATIONS`.
The physical-size cases remain blocking inventory: the G6 contract explains the
TeXpose side, but G12 has not yet supplied a per-case minimal differential
reproducer and target reconstruction sufficient to waive each downstream glyph
choice.

G5 accent-selection cases:

```text
accent-hat-j
accent-widehat-j
accent-widehat-xyz
accent-widetilde-xyz
accent-widehat-script
hard-accent-nested
hard-accent-deep-script
hard-matrix-fractions
hard-aligned-model
hard-overset-fraction
hard-stat-r2
```

The G6 / reference-size-policy signatures stay in the blocking inventory below.
The existing focused Rust tests establish TeXpose's MATH-scale contract, but that
by itself does not prove that every downstream radical/delimiter glyph difference
is solely caused by the reference size policy. G12 therefore does not waive those
cases yet.

### Libertinus Math

Eleven of the twelve observed mismatch cases are G5 accent-selection cases and
are pinned in `LIBERTINUS_STRESS_TRACE_GLYPH_DEVIATIONS`:

```text
accent-widehat-j
accent-widehat-xyz
accent-widetilde-xyz
accent-widehat-script
hard-accent-nested
hard-accent-fraction
hard-accent-deep-script
hard-matrix-fractions
hard-aligned-model
hard-overset-fraction
hard-brutal-core
```

### Fira Math

No remaining positioned glyph mismatch is ratified at stage 5. Fira's one
remaining case stays in the unresolved inventory below.

## Blocking positioned glyph inventory

These signatures are preserved exactly but are **not deviations**. A contractual
stress run must fail while they remain in `*_STRESS_TRACE_GLYPH_INVENTORY`.

### STIX `hard-brutal-core`

Current stress reproducer: the `hard-brutal-core` row in
`tests/fixtures/math_compare_stress.tsv`.

```text
primitive 22: 1397 / 1396   circumflex.s3 / circumflex.s2
primitive 28: 1409 / 1408   tilde.s5 / tilde.s4
primitive 53: 1304 / 1305   parenleft.s4 / parenleft.s5
primitive 77: 1316 / 1317   parenright.s4 / parenright.s5
```

The first two pairs are consistent with the already-ratified G5 accent rule, but
primitives 53 and 77 introduce delimiter-variant selection. The whole case stays
blocking until the delimiter pair has a minimal reproducer and its target extent
is reconstructed. Do not infer that the G2 `advanceMeasurement` rule alone
explains the observed pair: a different requested target can produce the same
symptom.

### STIX physical-size sweep

The following exact signatures remain blocking even though the upstream G6 size
policy is already known to differ from the pinned reference:

```text
size-frac-6pt
size-nested-frac-6pt
size-nested-frac-20pt
size-nested-frac-40pt
size-delim-6pt
size-radical-frac-6pt
size-indexed-radical-6pt
size-indexed-radical-20pt
size-indexed-radical-40pt
```

The minimal TeXpose contract reproducers are the G6 script/`ssty` tests and the
G9 root-size fraction test. What is still missing for a G12 deviation is the
minimal **differential** reproducer for each downstream selection, including the
requested variant extent where a radical or delimiter glyph changes. Until that
causal link is measured, these signatures remain inventory rather than waivers.

### Libertinus `hard-stat-r2`

Current stress reproducer: the `hard-stat-r2` row in the stress corpus.

```text
primitive 10: 3798 / 9      parenleft.size1 / parenleft
primitive 14: 4071 / 701    uni0302.size1 / uni0302
primitive 17: 3799 / 10     parenright.size1 / parenright
```

Primitive 14 is consistent with G5. Primitives 10 and 17 are delimiter selection
and remain unclassified until the target extent is isolated. The first unresolved
primitive is 10.

### Fira `hard-matrix-fractions`

Current stress reproducer: the `hard-matrix-fractions` row in the stress corpus.

```text
primitive 29: 2016 / 2014   uni23DE.size9 / uni23DE.size7
```

This is an overbrace horizontal-variant difference. The existing evidence is not
enough to decide whether the cause is variant-selection policy, a different
requested width, or surrounding matrix/accent geometry. Stage 5 deliberately
preserves that uncertainty. A future fix needs a minimal overbrace reproducer and
the exact TeXpose/reference target width before this pair can move to a deviation
or be repaired.

## Post-stage-8 outer-geometry inventory

Stages 7 and 8 closed `hard-matrix-fractions` and `hard-delim-eval`
respectively. No new geometry waiver was introduced. The remaining raw cases
above `0.050em` are:

STIX:

```text
size-indexed-radical-6pt       0.828115em
hard-logit                     0.512939em
hard-aligned-model             0.356907em
hard-wide-expression           0.349773em
size-frac-6pt                  0.331992em
size-delim-6pt                 0.331991em
size-radical-frac-6pt          0.324993em
size-nested-frac-6pt           0.291390em
size-indexed-radical-20pt      0.261050em
size-indexed-radical-40pt      0.261050em
hard-stat-r2                   0.237959em
hard-depth-typography          0.103974em
accent-widehat-j               0.079999em  (existing bounded canonical/G5 deviation)
hard-script-on-delimited       0.061649em
```

Libertinus:

```text
size-indexed-radical-6pt       0.390829em
hard-aligned-model             0.276717em
size-indexed-radical-20pt      0.167502em
size-indexed-radical-40pt      0.167500em
hard-stat-r2                   0.162499em
hard-depth-typography          0.126376em
hard-script-extreme            0.076198em
accent-widehat-j               0.056999em  (existing bounded canonical/G5 deviation)
size-nested-frac-20pt          0.050606em
size-nested-frac-40pt          0.050603em
hard-sum-substack              0.050373em
hard-logit                     0.050335em
```

Fira:

```text
size-indexed-radical-6pt       0.422560em
hard-aligned-model             0.169117em
size-indexed-radical-20pt      0.133440em
size-indexed-radical-40pt      0.133440em
hard-script-extreme            0.127000em
size-nested-frac-6pt           0.072184em
hard-logit                     0.052344em
```

The cross-profile `hard-aligned-model`, `hard-logit`, and indexed-radical
families remain the highest-value causal targets. Their repetition across fonts
is evidence for investigation, not proof of a shared root cause. The
`accent-widehat-j` entries above are already-bounded G5 deviations rather than
new G12 blockers.

## Closure protocol

After every semantic G12 repair:

```powershell
cargo fmt --check
cargo test
cargo clippy --all-targets -- -D warnings
uv run --script tools\verify.py self-test
uv run --script tools\verify.py math --profile stix --fail-on-delta
uv run --script tools\verify.py math --profile libertinus --fail-on-delta
uv run --script tools\verify.py math --profile fira --fail-on-delta
uv run --script tools\verify.py math --profile stix --stress --top-worst 100
uv run --script tools\verify.py math --profile libertinus --stress --top-worst 100
uv run --script tools\verify.py math --profile fira --stress --top-worst 100
```

`--stress --fail-on-delta` is the final G12 closure check, not yet a repository
fast gate. It rejects exact inventory signatures until they are either repaired
or reclassified with the complete roadmap evidence tuple, and it still rejects
unbounded geometry above profile tolerance.

## Stage-6 causal explanation workflow

G12 investigations must not infer a primitive cause from an outer-box maximum.
The verifier therefore provides a diagnostic-only `--explain-case CASE` option.
It does not filter the corpus, alter the profile census, relax any contract, or
change `--fail-on-delta`. The normal complete run is still executed and
validated. After comparison, each requested case additionally reports:

```text
source formula
signed width/ascent/descent as TeXpose - LuaLaTeX
reference text/script/scriptscript math sizes
complete positioned primitive sequence
for every glyph: ID, x, baseline, scale and signed deltas
for every rule: x, bottom, width, height and signed deltas
```

The option is repeatable so one full stress execution can gather several
candidate roots without weakening census evidence. Example:

```powershell
uv run --script tools\verify.py math --profile fira --stress `
  --explain-case hard-aligned-model `
  --explain-case hard-delim-eval `
  --explain-case hard-matrix-fractions `
  --explain-case size-indexed-radical-6pt `
  --explain-case hard-logit
```

Signed values are diagnostic evidence, not tolerances. A positive value means
TeXpose is farther right/larger/higher than the reference for that field; a
negative value means the converse. Glyph-selection mismatches still withhold
identity-based geometry in the contractual comparator. The explanation output
shows paint-index coordinates only to reconstruct the first causal divergence;
it does not silently declare unlike glyphs geometrically equivalent.

## Stage-7 environment-cell glue closure

`hard-matrix-fractions` exposed a TeXpose parser bug rather than matrix-grid
geometry. The minimal reproducer is:

```tex
\begin{matrix}a\,b\end{matrix}
```

`peel_row_meta` recursively removed nodes through the generic `is_empty_node`
predicate. That predicate intentionally treats `MathNode::Space(_)` as empty for
other parser contracts, so using it while peeling row metadata discarded explicit
math glue from every environment cell. The repair removes only the empty-row
placeholder produced by row-metadata extraction and leaves ordinary math-space
nodes intact.

The focused owner is
`tests/amsmath_grid.rs::environment_cells_preserve_explicit_math_glue`. Across
STIX Two Math, Libertinus Math, and Fira Math it requires a one-cell matrix
containing `a\,b` to be exactly `3mu` wider than the corresponding `ab` matrix.

Post-fix stress measurements for `hard-matrix-fractions` are:

```text
profile      width signed     ascent signed    descent signed   <=0.050em census
STIX         +0.000036em       -0.000001em      -0.000001em      78/93
Libertinus   +0.000039em       -0.000002em      +0.000001em      80/93
Fira         +0.000034em       +0.014500em      +0.014501em      85/93
```

Before the repair the width deficit was approximately `-0.16663em` in all three
profiles, matching the lost `3mu`. The outer-width mismatch is therefore closed.
Positioned glyph identity counts remain unchanged (`72/93`, `81/93`, `92/93`):
STIX and Libertinus still contain their already-classified G5 accent selections,
and Fira still has the blocking overbrace pair `2016/2014`. Stage 7 does not
reclassify those glyph-selection mismatches.

## Stage-8 null-delimiter-space closure

`hard-delim-eval` exposed a physical-spacing bug in `\left...\right`. The
minimal reproducer is:

```tex
\left.x\right|
```

TeX/LuaTeX materializes an empty vertical delimiter with
`\nulldelimiterspace`; TeXpose previously represented `Delimiter::Empty` as a
zero-width `MathBox::empty()` in `delimited()`. The repair materializes the
existing typed physical `null_delimiter_space` as a horizontal kern for empty
left/right delimiters. It does not change visible delimiter sizing.

The focused owner is
`tests/delimiter_sizing.rs::left_right_null_delimiters_keep_physical_nulldelimiterspace`.
Across STIX Two Math, Libertinus Math, and Fira Math, at root-em sizes
6/10/20/40pt, it requires both `\left.` and `\right.` to contribute exactly
`1.2pt / root_em`. The pre-existing evaluation-delimiter assembly test now
requires the null side to be a kern while retaining its original visible
assembly checks.

Post-fix stress measurements for `hard-delim-eval` are:

```text
profile      width signed     ascent signed    descent signed   <=0.050em census
STIX         -0.000000em       +0.000000em      +0.000516em      79/93
Libertinus   -0.017597em       +0.000000em      -0.000000em      81/93
Fira         -0.014397em       +0.000000em      +0.000001em      86/93
```

Before the repair the width deficits were approximately `-0.120001em`,
`-0.137597em`, and `-0.134397em` respectively. The common `0.120000em`
component is the missing `1.2pt` at the 10pt root size. After restoration all
three cases are below the `0.050em` profile tolerance. The residual Libertinus
and Fira width deltas are therefore separate downstream geometry evidence, not
null-delimiter-space failures.

Canonical runs remain unchanged at STIX 24/25 with its existing approved G5
geometry deviation, Libertinus 24/25 with its existing approved G5 geometry
deviation, and Fira 25/25 with no approved geometry deviation. Stage 8 does not
change positioned glyph identity/order classifications.
