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
a G12 closure check. It is intentionally not a stable repository gate merely to
hide corpus debt; final G12 acceptance still requires every pinned profile to pass
with only the exact stale-sensitive records documented here.

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
- **G7 MathKern**: the published OpenType MATH algorithm evaluates two
  correction-height sums for each script attachment and applies their minimum.
  The focused owners are the STIX first/second correction-height tests in
  `src/layout/internal_tests/math_kern.rs`; the font-level lookup tests separately pin interval
  boundary behavior. The pinned LuaLaTeX reference differs on STIX
  `hard-logit`, but that observation does not override the published MATH rule.

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

All twenty-one observed mismatch cases are now ratified by tested G5 or G6
contracts. The exact signatures live in `STIX_STRESS_TRACE_GLYPH_DEVIATIONS`.

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
hard-brutal-core
```

G6 reference-size-policy cases ratified by Stage 11:

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

The Stage-11 section below records the differential reproducers, first divergent
primitive, measured style scales, focused contract tests, and bounded outer
geometry required to move those nine signatures out of inventory.

### Libertinus Math

All twelve observed mismatch cases are now classified. Eleven are the existing G5
accent-selection cases and the twelfth, `hard-stat-r2`, is the composite G5/G2/G7
signature reconstructed in the final cross-profile closure block below. Exact
signatures are pinned in `LIBERTINUS_STRESS_TRACE_GLYPH_DEVIATIONS`:

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

The Stage-11 contractual rerun shows that the stored Libertinus
`hard-brutal-core` signature was incomplete. Its two observed G5
accent-selection mismatches are:

```text
primitive 22: 4073 / 4072
primitive 28: 4217 / 4216
```

Primitive 28 carries the same `4217 / 4216` wide-tilde pair already ratified by
the G5 `accent-widetilde-xyz` contract. The final cross-profile closure separately
classifies `hard-stat-r2`; there is no remaining Libertinus glyph inventory.

### Fira Math

The sole Fira mismatch, `hard-matrix-fractions` primitive 29 (`2016/2014`), is
now an exact G5 horizontal-variant deviation. The final cross-profile closure
records the minimal overbrace reproducer and focused contract test. There is no
remaining Fira glyph inventory.

## Stage-12b STIX `hard-brutal-core` closure

`hard-brutal-core` was the last STIX positioned glyph-selection inventory case.
Its stress reproducer is the `hard-brutal-core` row in
`tests/fixtures/math_compare_stress.tsv`.

### Pre-fix signature and differential diagnosis

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

Stage-12b reconstructed that target and **refuted** the initial
`advanceMeasurement`-versus-ink-box hypothesis. The temporary STIX probe measured
the TeXpose inner-parenthesis request as `1.8468698em`; for `parenleft.s4` the
MATH advance is `1.907em` and the glyph ink span is `1.906em`, while `s5` is
`2.145em` / `2.144em`. Both selection policies therefore accept `s4` at the
TeXpose target. The probe was removed after this result.

The independent LuaLaTeX probe against the pinned reference fingerprint measured
the same inner body at `1.3510498046875em` height and `0.61720123291016em` depth,
with `AxisHeight = 0.25800018310547em`. LuaTeX consequently computes a delimiter
target of `1.9687377929687em`, which is larger than the `s4` ink span and selects
`s5`. The delimiter pair is therefore a downstream symptom of earlier vertical
geometry, not a G2 variant-measure disagreement.

The causal boundary is the cramped-script integral in the inner fraction. STIX
uses `ssty` for that operator. Its base integral glyph is covered by
`ExtendedShapeCoverage`, but the selected level-1 `ssty` alternate is not.
TeXpose previously tested `ExtendedShapeCoverage` only on the post-GSUB direct
glyph, so an `Op` nucleus could lose `superscriptBaselineDropMax` /
`subscriptBaselineDropMin` after substitution. LuaTeX `op_noad` semantics box the
operator nucleus before side-script placement and retain those baseline-drop
constraints. Stage-12b therefore treats an `Op` nucleus with side scripts as
box-like for baseline-drop purposes while leaving ordinary direct-glyph
`ExtendedShapeCoverage` semantics unchanged. A focused regression requires the
STIX `ssty` alternate to be outside `ExtendedShapeCoverage` and still verifies
the operator ink-box baseline-drop equations.

The STIX constants make the repair numerically discriminating. With the level-1
`ssty` integral at script scale, the old ordinary-glyph path settles the lower
script baseline at `0.2185em`, yielding integral depth `0.22675em`. Applying the
operator ink-box baseline-drop constraint moves that baseline to `0.28665em` and
yields depth `0.29490em`, an increase of `0.06815em`. The reconstructed LuaTeX
maximum axis distance (`716341 / 655360 = 1.093049621582...em`) exceeds the
pre-fix TeXpose distance (`1.0249em`) by `0.068149621582...em`. The local operator
error therefore accounts for the delimiter-target discrepancy at the causal
quantity, before any variant is selected.

The contractual post-fix rerun proves the predicted repair. Primitives 53 and 77
now match the pinned reference exactly (`1305/1305` and `1317/1317`). The exact
remaining signature is only the already-ratified G5 accent pair:

```text
primitive 22: 1397 / 1396   circumflex.s3 / circumflex.s2
primitive 28: 1409 / 1408   tilde.s5 / tilde.s4
```

The repaired case has `97/97` glyphs and `4/4` rules. Its outer deltas are
`0.000036em` width, `0.001001em` ascent, and `0.001001em` descent. In the full
positioned explanation, the formerly divergent delimiters are identity-aligned
and differ only by numerical noise at roughly `0.000001em` baseline and
`0.000017-0.000036em` x. This is direct evidence that the operator baseline-drop
repair removed the delimiter-selection symptom without weakening G2.

Stage 12b therefore moves the two-pair `hard-brutal-core` signature from
`STIX_STRESS_TRACE_GLYPH_INVENTORY` to `STIX_STRESS_TRACE_GLYPH_DEVIATIONS`.
The STIX positioned glyph-selection inventory is now empty. This classification
does not approve any unrelated outer or positioned geometry residual; the next
contractual run must expose those independently. Reopen this diagnosis if the
focused operator regression fails or if primitives 53/77 reappear.

## Resolved positioned glyph inventory

The former blocking signatures are now exact, stale-sensitive deviations rather
than open inventory. This does not make their whole formulas geometry-comparable;
it only records the classified glyph-selection chain.

### Libertinus `hard-stat-r2`

The stress reproducer is:

```tex
R^2=1-\frac{\sum_{i=1}^{n}\left(y_i-\widehat y_i\right)^2}
{\sum_{i=1}^{n}\left(y_i-\bar y\right)^2}
```

The exact signature is:

```text
primitive 10: 3798 / 9      parenleft.size1 / parenleft
primitive 14: 4071 / 701    uni0302.size1 / uni0302
primitive 17: 3799 / 10     parenright.size1 / parenright
```

Primitive 14 is the already-ratified G5 widehat selection. That taller selected
accent changes the enclosed `\left...\right` target and the pinned reference
therefore takes the base parentheses while TeXpose takes the first vertical
variants under its established G2 delimiter target/`advanceMeasurement` rule.
The superscript on the resulting parenthesized box additionally exposes the same
G7 box/ExtendedShape baseline-drop policy split measured elsewhere. The formula
therefore remains non-comparable as a positioned whole; only the exact signature
and the bounded outer propagation are approved. The focused
`delimiter_sizing::libertinus_widehat_body_drives_delimiter_variants_by_g2_target`
regression pins the complete TeXpose-side signature; the underlying G5 wide-accent,
G2 delimiter-selection, and G7 script-placement tests remain the primary semantic
owners.

### Fira `hard-matrix-fractions`

The minimal causal fragment is `\overbrace{a+b+c}` from the matrix third row.
The exact mismatch is:

```text
primitive 29: 2016 / 2014   uni23DE.size9 / uni23DE.size7
```

Fira Math 0.3.4 publishes a horizontal U+23DE construction with ordered prebuilt
variants. TeXpose selects from the actual `MathGlyphVariantRecord.advanceMeasurement`
values, as required by G5/OpenType MATH; the pinned LuaLaTeX reference selects the
smaller size7 glyph for this target.
`wide_accent_math::fira_overbrace_prebuilt_variant_follows_math_advance_measurement`
reads the fixture's MATH records directly, derives the variant from the measured
base width, and pins glyph 2016. The full matrix outer geometry was already below
tolerance (`0.000034em` width, `0.014500em` ascent, `0.014501em` descent), so no
outer or positioned-geometry ceiling is granted.

## Post-stage-10 outer-geometry inventory

After Stages 9 and 10 the raw `<=0.050em` census is `79/93` STIX, `85/93`
Libertinus, and `88/93` Fira. Stage 11 does not change those raw measurements; it
classifies only the G6 cases whose first divergence has now been reconstructed.
The following stress-only outer ceilings are bounded profile records:

| Profile | Case | Observed maximum | Ceiling | Cause |
| --- | --- | ---: | ---: | --- |
| STIX | `hard-logit` | 0.449940em | 0.450100em | G7 OpenType MathKern minimum-of-sums |
| STIX | `hard-wide-expression` | 0.349773em | 0.349900em | G7 OpenType MathKern minimum-of-sums |
| STIX | `hard-aligned-model` | 0.214347em | 0.214500em | G5 accent + G7 MathKern/ExtendedShape propagation |
| STIX | `hard-stat-r2` | 0.209959em | 0.210100em | G7 OpenType MathKern minimum-of-sums; G5 glyph split separately ratified |
| STIX | `hard-depth-typography` | 0.068974em | 0.069100em | G7 OpenType MathKern minimum-of-sums |
| STIX | `size-frac-6pt` | 0.331992em | 0.332100em | G6 reference-size policy |
| STIX | `size-nested-frac-6pt` | 0.291390em | 0.291500em | G6 reference-size policy |
| STIX | `size-delim-6pt` | 0.331991em | 0.332100em | G6 reference-size policy |
| STIX | `size-radical-frac-6pt` | 0.324993em | 0.325100em | G6 reference-size policy |
| STIX | `size-indexed-radical-6pt` | 0.828115em | 0.828200em | G6 reference-size policy |
| STIX | `size-indexed-radical-20pt` | 0.261050em | 0.261200em | G6 reference-size policy |
| STIX | `size-indexed-radical-40pt` | 0.261050em | 0.261200em | G6 reference-size policy |
| Libertinus | `hard-aligned-model` | 0.185999em | 0.186200em | G5 accent + G7 ExtendedShape propagation |
| Libertinus | `hard-stat-r2` | 0.162499em | 0.162700em | G5 accent/delimiter + G7 ExtendedShape propagation |
| Libertinus | `size-indexed-radical-6pt` | 0.390829em | 0.391000em | G6 reference-size policy |
| Libertinus | `size-indexed-radical-20pt` | 0.167502em | 0.167700em | G6 reference-size policy |
| Libertinus | `size-indexed-radical-40pt` | 0.167500em | 0.167700em | G6 reference-size policy |
| Libertinus | `size-nested-frac-20pt` | 0.050606em | 0.050800em | G6 reference-size policy |
| Libertinus | `size-nested-frac-40pt` | 0.050603em | 0.050800em | G6 reference-size policy |
| Fira | `hard-aligned-model` | 0.079861em | 0.080100em | G7 ExtendedShape/aligned-stack propagation |
| Fira | `size-indexed-radical-6pt` | 0.422560em | 0.422800em | G6 reference-size policy |
| Fira | `size-indexed-radical-20pt` | 0.133440em | 0.133600em | G6 reference-size policy |
| Fira | `size-indexed-radical-40pt` | 0.133440em | 0.133600em | G6 reference-size policy |
| Fira | `size-nested-frac-6pt` | 0.072184em | 0.072400em | G6 reference-size policy |

STIX `size-nested-frac-20pt` and `size-nested-frac-40pt` are below the outer
tolerance (`0.036408em` and `0.036404em`) but retain exact G6 glyph-selection
signatures, so they require no outer ceiling. STIX's G6 selection differences
make those positioned traces non-comparable by design. Libertinus and Fira keep
identity-comparable size sweeps; their corresponding positioned ceilings are
recorded separately by `stress_trace_deviations`.

After removing the already-bounded canonical G5 case and the Stage-11 G6 cases,
the remaining **unwaived** outer geometry above `0.050em` is:

STIX:

```text
(none)
```

The post-Stage-13e rerun also measures `hard-script-on-delimited` at only
`0.000002em` width, `0.000003em` ascent, and `0.000001em` descent. Its older
`0.061649em` entry was rendered stale by the Stage-12b operator/script repair and
is removed from the blocking inventory without adding a deviation. The current
raw STIX outer census is therefore `80/93 <= 0.050em`; bounded deviations do not
rewrite that raw measurement.

Libertinus and Fira have no deliberately unclassified outer case in this closure
candidate. The final all-profile G12 rerun remains authoritative: any new case,
stale record, changed glyph signature, or delta above a recorded ceiling fails
the closure gate rather than being absorbed into these classifications.

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
`src/layout/internal_tests/amsmath_grid.rs::environment_cells_preserve_explicit_math_glue`. Across
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
`src/layout/internal_tests/delimiter_sizing.rs::left_right_null_delimiters_keep_physical_nulldelimiterspace`.
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

## Stage-9 paired-script-gap closure

`hard-script-extreme` exposed an ordering bug in the paired subscript/superscript
vertical adjustment. The minimal reproducer is:

```tex
A_B^F
```

OpenType MATH defines `SubSuperscriptGapMin` as the minimum ink gap and
`SuperscriptBottomMaxWithSubscript` as the highest superscript-bottom level that
may be used to increase that gap before the subscript is moved down. The latter
is therefore part of repairing an insufficient paired-script gap, not an
independent minimum superscript position. LuaTeX's `make_scripts` follows the
same ordering: it applies the paired-bottom redistribution only inside the
branch that first detects a `SubSuperscriptGapMin` deficit.

TeXpose previously applied `SuperscriptBottomMaxWithSubscript` even when the
initial gap already satisfied `SubSuperscriptGapMin`. Fira Math makes this
observable without another confounder: for `A_B^F` the gap is already legal,
but the old implementation translated the pair by `0.127em`. The repair keeps
the redistribution inside the actual gap-repair branch.

The focused owner is
`src/layout/internal_tests/script_placement.rs::paired_bottom_max_only_redistributes_an_actual_gap_repair`.
It derives the initial Fira positions from the font's MATH constants, proves the
initial gap already satisfies `SubSuperscriptGapMin`, proves the old
paired-bottom adjustment would still have been available, and then requires the
laid-out scripts to retain the initial legal positions. The adjacent STIX test
continues to cover the path where a real gap repair does require the secondary
redistribution.

Post-fix stress measurements for `hard-script-extreme` are:

```text
profile      width signed     ascent signed    descent signed   <=0.050em census
STIX         -0.027499em       +0.000001em      +0.000002em      79/93
Libertinus   -0.076198em       -0.000001em      +0.000003em      81/93
Fira         -0.034800em       -0.000002em      -0.000000em      87/93
```

The Fira positioned baselines now agree to within `0.000002em`, closing the
previous `0.127em` paired-script translation and moving that stress case below
tolerance. STIX and Libertinus also have effectively aligned vertical script
positions; their remaining `hard-script-extreme` width residuals are horizontal
geometry and are not classified by this stage.

Canonical runs remain unchanged at STIX 24/25 with its existing approved G5
geometry deviation, Libertinus 24/25 with its existing approved G5 geometry
deviation, and Fira 25/25 with no approved geometry deviation. Positioned glyph
identity/order classifications also remain unchanged at 72/93, 81/93, and
92/93 respectively.

## Stage-10 clean-script terminal-italic closure

Stage 10 closes a horizontal-width loss in cleaned script boxes. The minimal
reproducer is a scripted box whose one-glyph script has non-zero MATH italics
correction, for example with Fira Math:

```tex
\frac{a}{b}_E
```

TeXpose previously laid out the script through the ordinary script style and
kept the terminal glyph's MATH italics correction only as `MathBox::italic`
metadata. Script attachment then consumed `width` plus `SpaceAfterScript`, so
the terminal italic extent was lost from the cleaned script-box width. LuaTeX's
`clean_box` packages the math list before removing its terminal italic-correction
node, which leaves that correction represented in the packaged box width.

The repair adds the terminal math italic extent to the cleaned script component
and clears the metadata copy while deliberately preserving direct
`BoxContent::Glyph` content. Preserving the direct glyph is part of the contract:
subsequent MathKern lookup must still be able to inspect the script glyph rather
than seeing an opaque box.

The focused owner is
`src/layout/internal_tests/script_space_after.rs::script_clean_box_keeps_terminal_math_italic_in_its_width`.
With Fira Math it checks both subscript and superscript paths and derives the
expected width as the base width plus script glyph width, the script glyph's
terminal MATH italics correction, and parent-style `SpaceAfterScript`. Existing
`src/layout/internal_tests/math_kern.rs` remains green and therefore independently protects the
requirement that this cleaning step not hide a direct script glyph from MathKern.

Post-fix focal stress measurements are:

```text
profile      hard-script-extreme width   hard-logit width       hard-aligned-model width
STIX         +0.000001em                  -0.449940em             -0.209908em
Libertinus   +0.000002em                  +0.000064em             +0.000084em
Fira         +0.000000em                  -0.019945em             +0.000084em
```

The overall `<=0.050em` stress census is now:

```text
STIX         79/93
Libertinus   85/93
Fira         88/93
```

This stage therefore closes the terminal-script-italic component but does not
classify unrelated residuals. In particular, STIX `hard-logit` still contains a
large horizontal MathKern-policy difference, and Libertinus/Fira
`hard-aligned-model` retain vertical differences (`0.185999em` and `0.079861em`
respectively) after their horizontal width is effectively aligned. Those are
separate contracts for later G12 classification.

Canonical runs remain unchanged at STIX 24/25 with its existing approved G5
geometry deviation, Libertinus 24/25 with its existing approved G5 geometry
deviation, and Fira 25/25 with no approved geometry deviation. Positioned glyph
identity/order classifications are unchanged because Stage 10 changes script-box
width only, not glyph selection or paint order.

## Stage-11 G6 reference-size-policy classification

Stage 11 closes the previously blocking **classification** of the physical-size
sweep. It does not change TeXpose layout. The governing TeXpose contract remains
G6: first-level and second-level scripts use the selected font's OpenType MATH
`scriptPercentScaleDown` and `scriptScriptPercentScaleDown`, and semantic script
nesting selects `ssty` level 1 and level 2. OpenType's MATH/`ssty` contract is
recorded at:

```text
https://learn.microsoft.com/en-us/typography/opentype/spec/math
```

The pinned LuaLaTeX reference follows a different path when the stress corpus
changes root size after math-font setup. `unicode-math` reads the current LaTeX
math sizes during `\setmathfont`, derives `\DeclareMathSizes` from MATH
fontdimens 10/11 for that current size, and sets the script/script-script
`SizeFeatures` thresholds there. Source inspected for this decision:

```text
https://github.com/latex3/unicode-math/blob/184a23b0cb259d4dc9848ec3db0aa2cd383cae99/um-code-main.dtx
```

The stress runner subsequently changes root size per case without re-running
`\setmathfont`. The pinned reference therefore measured:

```text
root 6pt:  script 5pt  = 0.833333em, scriptscript 5pt  = 0.833333em
root 20pt: script 14pt = 0.699997em, scriptscript 10pt = 0.500000em
root 40pt: script 28pt = 0.699997em, scriptscript 20pt = 0.500000em
```

TeXpose remains font-owned instead: STIX `0.70/0.55`, Libertinus `0.80/0.60`,
and Fira `0.72/0.58`. The differential evidence now reconstructs the first
divergence to those ratios rather than merely observing downstream glyphs.

### Minimal differential reproducers and first divergence

The size-sweep cases reduce to three small sources:

```tex
\frac{a+b}{c+d}
\frac{1+\frac{a}{b}}{1+\frac{c}{d}}
\sqrt[\frac{1+\alpha}{2}]{x}
```

At 6pt, the first fraction source enters script style with reference
`0.833333em` instead of the font-owned TeXpose ratio; STIX immediately selects
different `ssty` glyphs. `size-delim-6pt` and `size-radical-frac-6pt` inherit the
same already-divergent fraction before delimiter/radical geometry is chosen.

The nested-fraction source exposes the next semantic level directly. Examples
from the Stage-11 positioned traces are Libertinus at 20/40pt
`0.800000/0.699997` and Fira at 6pt `0.720000/0.833333` on the first inner
fraction glyph. STIX 20/40pt reaches the same contract through different `ssty`
glyph identities, so its geometry is intentionally non-comparable rather than
force-paired.

The indexed-radical source is even more direct because the degree is laid out in
script-script style. Its first degree glyph measures:

```text
profile      6pt TeXpose/ref      20pt TeXpose/ref     40pt TeXpose/ref
STIX         0.55 / 0.833333       0.55 / 0.50          0.55 / 0.50
Libertinus   0.60 / 0.833333       0.60 / 0.50          0.60 / 0.50
Fira         0.58 / 0.833333       0.58 / 0.50          0.58 / 0.50
```

That scale difference occurs before the radical's downstream x origin and width
diverge. It is therefore the causal first primitive, not a radical-variant
selection defect.

### Focused TeXpose contracts

The accepted side of the deviation is independently protected by:

```text
src/layout/internal_tests/script_style_alternates.rs::script_scales_and_ssty_alternates_follow_each_real_font
src/layout/internal_tests/script_style_alternates.rs::actual_script_nesting_selects_level_one_then_level_two
src/layout/internal_tests/fraction_semantics.rs::nested_fraction_keeps_math_script_scale_across_root_em_sizes_and_profiles
src/layout/internal_tests/radical_geometry.rs::radical_surd_and_rule_geometry_is_root_em_invariant_across_required_sweep
```

The first two pin the real-font MATH percentages and `ssty` levels. The fraction
test explicitly runs 6/10/20/40pt and rejects root-size-dependent replacement of
font MATH script ratios. The radical sweep independently protects root-em
resolution of the radical itself, preventing this deviation record from masking
a physical radical-scaling regression.

### Profile records and ceilings

STIX's nine exact size-sweep glyph signatures move from blocking inventory to
`STIX_STRESS_TRACE_GLYPH_DEVIATIONS`. Its selection-changing traces remain
non-comparable; seven cases above outer tolerance receive the bounded ceilings
listed in the post-Stage-10 table above.

Libertinus and Fira do not change glyph identity for these measured size-policy
cases, so their positioned geometry remains comparable. Stage 11 pins these
positioned maxima separately:

| Profile | Case | Positioned observed | Ceiling |
| --- | --- | ---: | ---: |
| Libertinus | `size-indexed-radical-6pt` | 0.390833em | 0.391000em |
| Libertinus | `size-indexed-radical-20pt` | 0.167502em | 0.167700em |
| Libertinus | `size-indexed-radical-40pt` | 0.167500em | 0.167700em |
| Libertinus | `size-nested-frac-20pt` | 0.100003em | 0.100200em |
| Libertinus | `size-nested-frac-40pt` | 0.100003em | 0.100200em |
| Fira | `size-indexed-radical-6pt` | 0.422560em | 0.422800em |
| Fira | `size-indexed-radical-20pt` | 0.133441em | 0.133600em |
| Fira | `size-indexed-radical-40pt` | 0.133440em | 0.133600em |
| Fira | `size-nested-frac-6pt` | 0.113333em | 0.113500em |

The verifier protocol becomes `oracle-v11` and gains stress-only outer geometry
records. Canonical/global deviations remain a separate field; `self-test` proves
that a stress-only record cannot leak into canonical scope and that stress
positioned ceilings are enforced independently. All ceilings remain stale-
sensitive.

After this reclassification the unresolved positioned glyph inventory is only:

```text
STIX:       hard-brutal-core
Libertinus: hard-stat-r2
Fira:       hard-matrix-fractions
```

Stage 11 intentionally does **not** classify the remaining G7/MathKern,
ExtendedShape, overbrace, or delimiter-selection residuals. Those require their
own complete G12 evidence tuple and remain blocking.


## Stage-12a STIX MathKern policy classification

Stage 12a closes one G7 **classification** without changing TeXpose layout. The
measured stress case is:

```tex
\operatorname{logit}\left[P(Y_i=1\mid X_i)\right]=\beta_0+\sum_{j=1}^{p}\beta_jX_{ij}
```

The outer geometry is `15.343044em` in TeXpose versus `15.792984em` in the
pinned LuaLaTeX reference, a signed width delta of `-0.449940em`. Glyph and rule
counts are identical (`31/31`, `0/0`). The positioned trace is topology-aligned
and identity/order aligned for all 31 glyphs; its maximum is `0.449941em` in
`glyph-x`.

The first causal divergence is already visible in the minimal subexpression
`Y_i`. Primitive 8, the base `Y`, is aligned to `0.000002em`. Primitive 9, the
script `i`, keeps the same glyph ID (`4430`), baseline (`-0.210000em` versus
`-0.210001em`), and scale (`0.700000`) but moves from `3.857667em` in TeXpose to
`4.198665em` in the reference, a signed x delta of `-0.340998em`. Later script
attachments add further horizontal differences while preserving identity,
baseline, and scale, reaching the final `0.449941em` maximum. This localizes the
root to horizontal script kerning rather than G6 scaling, glyph selection, or
vertical placement.

The governing external contract is OpenType MATH. For a subscript it requires
two correction heights, sums the base bottom-right MathKern with the script
top-left MathKern at each height, and applies the **minimum** of the two sums.
The same minimum rule applies to superscripts. Current specification text:

```text
https://learn.microsoft.com/en-us/typography/opentype/spec/math
```

TeXpose implements that rule directly in `superscript_math_kern` and
`subscript_math_kern` with `first.min_ref(&second)`. Existing focused contracts
prove that STIX can select either candidate when it is the minimum:

```text
src/layout/internal_tests/math_kern.rs::stix_superscript_math_kern_can_select_the_first_correction_height
src/layout/internal_tests/math_kern.rs::stix_superscript_math_kern_can_select_the_second_correction_height
src/layout/internal_tests/math_kern.rs::stix_subscript_math_kern_can_select_the_first_correction_height
src/layout/internal_tests/math_kern.rs::stix_subscript_math_kern_can_select_the_second_correction_height
```

The specification ecosystem itself records an unresolved ambiguity: OpenType
issue 1147 notes that the literal minimum chooses the more-negative kern when
both candidates are negative, while Word uses the smaller-magnitude adjustment.
That issue is evidence of a cross-engine policy split, not authority to replace
the currently published OpenType algorithm:

```text
https://github.com/MicrosoftDocs/typography-issues/issues/1147
```

Stage 12a therefore adds two STIX stress-only, stale-sensitive records for
`hard-logit`: an outer ceiling of `0.450100em` and a positioned-trace ceiling of
`0.450100em`. Neither record permits structural or glyph-selection differences.
The raw stress census remains `79/93`, and the positioned measurement remains
`93/93` kind topology, `72/93` glyph identity/order aligned, and `72/93` geometry
comparable. The record only changes classification of the already-measured G7
difference.

Reopen this decision if the published OpenType rule changes, if TeXpose no longer
selects the specified minimum, if the pinned reference converges, or if the
measured delta exceeds the ceiling. The verifier's stale-ceiling checks force
review if the difference disappears. At Stage 12a, `hard-wide-expression`,
`hard-aligned-model`, `hard-stat-r2`, `hard-depth-typography`,
`hard-script-on-delimited`, and the unresolved glyph inventories remained separate
G12 work; Stage 13a below reclassifies only `hard-wide-expression`.

## Stage-13a STIX repeated-subscript MathKern policy classification

Stage 13a closes a second G7 **classification** without changing TeXpose layout.
The measured stress case is:

```tex
a_1+a_2+a_3+a_4+a_5+a_6+a_7+a_8+a_9+a_{10}+b_1+b_2+b_3+b_4+b_5+b_6+b_7+b_8+b_9+b_{10}
```

Outer geometry is `41.677643em` in TeXpose versus `42.027415em` in the pinned
LuaLaTeX reference, a signed width delta of `-0.349773em`. Ascent and descent
remain aligned within `0.000001em`; glyph and rule counts are identical
(`61/61`, `0/0`). The positioned trace is topology- and identity/order-aligned
for all 61 glyphs, with a maximum `glyph-x` delta of `0.349772em`.

The first causal divergence occurs only after the complete `a_1` through
`a_{10}` prefix. Primitive 31, the first `b`, is aligned to `0.000113em`. At
primitive 32, its `1` subscript keeps the same glyph (`4274`), baseline
(`-0.210000em` versus `-0.210001em`), and scale (`0.700000`) but appears at
`22.184044em` in TeXpose versus `22.218930em` in the reference. Relative to the
base origin, TeXpose places that subscript at `+0.503000em` while the reference
uses `+0.537999em`, exposing an approximately `+0.035em` horizontal MathKern
difference before the following `+` is laid out.

The same step repeats once for every subsequent `b_i`. After `b_2` the cumulative
position difference is approximately `0.070em`, after `b_5` approximately
`0.175em`, and after `b_9` approximately `0.315em`; `b_{10}` reaches the measured
`0.349772em` positioned maximum. `SpaceAfterScript` cannot be the first cause
because the divergence is already present at each subscript glyph before the
following inter-atom content. Base italic correction is likewise not the cause:
the subscript default origin is the base advance, and the extra reference offset
is the height-dependent MathKern adjustment.

The governing rule is the same G7 contract already used for Stage 12a. OpenType
MATH requires the subscript algorithm to evaluate the two bottom-right/top-left
correction-height sums and apply their **minimum**:

```text
https://learn.microsoft.com/en-us/typography/opentype/spec/math
```

TeXpose implements that rule with `first.min_ref(&second)`. Its focused STIX
contracts in `src/layout/internal_tests/math_kern.rs` prove independently that either correction
height can win when it is the smaller sum. The pinned LuaLaTeX reference follows
a different policy. Current LuaTeX source computes the same two sums but replaces
the first when the second is greater than or equal to it (`>=`); the TeX Live
change that introduced this behavior explicitly changed the former `<` comparison
to `>=` for both superscripts and subscripts:

```text
https://ftp.tug.org/pipermail/tex-live-commits/2025-March/033321.html
```

Stage 13a therefore adds two STIX stress-only, stale-sensitive records for
`hard-wide-expression`: an outer ceiling of `0.349900em` and a positioned-trace
ceiling of `0.349900em`. Neither permits structural or glyph-selection
differences. The raw `<=0.050em` census is unchanged by this classification.

Reopen this decision if the published OpenType rule changes, LuaTeX returns to
the minimum-of-sums rule, TeXpose no longer selects the specified minimum, the
measured case ceases to exhibit the repeated per-`b_i` step, or either delta
exceeds its ceiling. All other G12 residuals remain separate work.

## Stage-13b STIX ExtendedShape superscript-policy classification

Stage 13b closes one G7 **classification** without changing TeXpose layout. The
measured stress case is:

```tex
\sum_{\substack{1\le i\le n\\1\le j\le m\\i\ne j}}\frac{x_i-x_j}{1+(x_i-x_j)^2}
```

Outer geometry is already aligned: TeXpose/reference differ by only
`+0.000026em` width, `+0.000000em` ascent, and `-0.000003em` descent, with
identical glyph/rule counts (`29/29`, `1/1`). The positioned trace is also
topology- and identity/order-aligned for all 30 primitives. The first 19
primitives -- the sum, its three-row substack, the fraction numerator, and the
fraction rule -- agree to rounding noise. This rules out G8 lower-limit and G10
substack placement as the source.

The divergence starts in the fraction denominator. Primitives 20 through 28,
which contain `1+(x_i-x_j)`, all share the same signed baseline delta of
`-0.239700em`. The final superscript glyph at primitive 29 does not share that
translation: its baseline is `-0.395700em` in TeXpose versus `-0.409999em` in
the reference. Relative to the right-parenthesis base at primitive 28, TeXpose
therefore raises `2` by exactly `0.506000em`, while the reference raises it by
approximately `0.252001em`. The different superscript height changes the
denominator box height; fraction gap enforcement then moves the ordinary
denominator glyphs in the opposite direction, which is why the outer fraction
geometry remains nearly unchanged.

The STIX Two Math 2.13 fixture makes the policy split explicit. Its MATH table
contains:

```text
superscriptShiftUpCramped   = 252 units = 0.252em
superscriptBaselineDropMax  = 230 units = 0.230em
```

Glyph `1065`, the right parenthesis in the trace, is present in
`ExtendedShapeCoverage`. Its ink height in this context is `0.736em`, so the
published extended-shape baseline-drop constraint gives
`0.736 - 0.230 = 0.506em`, exactly the TeXpose relative superscript shift. The
pinned reference's `0.252001em` relative shift instead matches
`superscriptShiftUpCramped`.

This distinction is specification-owned. OpenType MATH states that
`superscriptBaselineDropMax` is checked for bases treated as a box or extended
shape, and its ExtendedShape section explains that ordinary vertical positioning
algorithms are not appropriate for those glyphs:

```text
https://learn.microsoft.com/en-us/typography/opentype/spec/math
```

TeXpose's G7 contract intentionally integrates `ExtendedShapeCoverage` into
vertical script positioning. Current LuaTeX's `make_scripts` takes a different
path for a glyph node: it skips the base-box baseline-drop initialization and
then applies `sup_shift_up(cur_style)` / `SuperscriptShiftUpCramped` as the
minimum script shift. That behavior explains the pinned LuaHBTeX 1.25.7 trace;
it is reference behavior, not authority to weaken the published G7 contract.

Stage 13b therefore adds only a STIX stress positioned-trace ceiling of
`0.239800em` for `hard-sum-substack`. No outer geometry ceiling is needed, no
structural or glyph-selection difference is permitted, and the global
`0.050em` geometry tolerance is unchanged.

Reopen this decision if the OpenType ExtendedShape script rule changes, if the
STIX parenthesis leaves `ExtendedShapeCoverage`, if the pinned reference begins
using the baseline-drop rule, if TeXpose ceases to apply it, or if the positioned
delta exceeds the ceiling. All other G12 residuals remain separate work.

## Stage-13c STIX radical-index MathKern propagation classification

Stage 13c closes one G7 **classification** without changing TeXpose layout. The
measured stress case is:

```tex
\sqrt[\frac{1+\alpha}{2}]{\frac{x_i^{2n}+y_j^{2n}}{1+\sqrt{1+z^2}}}
```

Outer geometry is already within the global tolerance: TeXpose/reference width
is `7.599739em` versus `7.630711em`, a signed `-0.030972em` delta; ascent and
descent agree within `0.000001em`, with identical glyph/rule counts (`24/24`,
`4/4`). The positioned trace is topology- and identity/order-aligned for all 28
primitives. The radical degree (primitives 0--4) and radical assembly
(primitives 5--8) agree to at most `0.001000em`, so neither degree placement nor
G4 radical sizing is the first cause.

The horizontal difference comes from the radicand. Its fraction rule is
`0.030976em` wider in the reference. The numerator is consequently centered
about `0.01549em` farther right, which explains the common offset on
`x_i^{2n}`, the plus sign, and the `y` base. Primitive 18, the `j` subscript in
`y_j^{2n}`, then adds a distinct local split: its signed x delta is
`-0.084481em`, approximately `0.069001em` beyond the inherited centering shift.
Vertical position, glyph identity, and script scale remain aligned.

The pinned STIX Two Math 2.13 MATH data makes this another instance of the G7
MathKern policy already classified in Stages 12a and 13a. Glyph `3355` (`y`) has
advance `0.510em` and a bottom-right MathKern table with correction heights
`0.150em`, `0.283em` and kern values `-0.045em`, `+0.024em`, `+0.060em`. Glyph
`4432` (the level-1 `j` script alternate) has a constant top-left MathKern of
`-0.080em`, which contributes `-0.056em` after the `0.700000` script scale. The
two candidate subscript sums in this measured geometry are therefore:

```text
first correction-height sum   +0.024 + (-0.056) = -0.032em
second correction-height sum  -0.045 + (-0.056) = -0.101em
```

The positioned trace independently exposes those values. Relative to the `y`
origin, TeXpose places `j` at `+0.409000em`, i.e. `0.510 - 0.101`; the reference
places it at approximately `+0.478002em`, i.e. `0.510 - 0.031998`. The local
reference-minus-TeXpose difference is therefore approximately `0.069002em`.

The governing contract remains the published OpenType MATH MathKern algorithm:
for subscripts, evaluate the two bottom-right/top-left correction-height sums and
apply their **minimum**. TeXpose does so with `first.min_ref(&second)`, and the
focused STIX regressions in `src/layout/internal_tests/math_kern.rs` independently prove both
possible minimum branches. Current LuaTeX instead replaces the first sum when
the second is greater than or equal to it, producing the greater-sum reference
position. This is the same external policy split already ratified for
`hard-logit` and `hard-wide-expression`; the radical and fraction only propagate
that local horizontal difference.

Stage 13c therefore adds only a STIX stress positioned-trace ceiling of
`0.084600em` for `hard-radical-index-complex`. No outer geometry deviation is
needed, no structural or glyph-selection difference is permitted, and the
global `0.050em` geometry tolerance is unchanged.

Reopen this decision if the published OpenType MathKern rule changes, LuaTeX
returns to minimum-of-sums selection, STIX changes the relevant MathKern data,
TeXpose no longer selects the specified minimum, the first causal split moves
away from `y_j`, or the positioned delta exceeds the ceiling. All other G12
residuals remain separate work.

## Stage-13d STIX depth-typography MathKern classification

Stage 13d closes one additional G7 **classification** without changing TeXpose
layout. The measured stress case is:

```tex
\left(\frac{g_j+y_q+p_g}{A^B+C^D}\right)_{gypq}^{ABCDEFGHIJKLMNOPQRSTUVWXYZ}
```

The positioned trace is topology- and identity/order-aligned for all 46
primitives. Primitives 0--4 agree within `0.000012em`; the first material split
is primitive 5, the level-1 `q` subscript in `y_q`. TeXpose places it at
`3.068544em` while the pinned LuaLaTeX reference places it at `3.137534em`, a
signed `-0.068989em` x delta. That displacement then propagates through the
remaining numerator, fraction rule, closing delimiter, and outer scripts.
Vertical positions and script scale remain aligned.

The pinned STIX Two Math 2.13 MATH data makes the local cause exact. Glyph
`3355` (`y`) has advance `0.510em` and bottom-right MathKern values
`-0.045em`, `+0.024em`, `+0.060em` across its two correction-height boundaries.
Glyph `4440` (the level-1 `q` script alternate) contributes no top-left
MathKern, so the two candidate subscript sums in this geometry are simply
`-0.045em` and `+0.024em`. Relative to the `y` origin, TeXpose places `q` at
`+0.465000em`, exactly `0.510 - 0.045`; the reference places it at approximately
`+0.534002em`, exactly the greater-sum path up to reference rounding. The local
reference-minus-TeXpose split is therefore approximately `0.069002em`.

The governing contract remains the published OpenType MATH MathKern algorithm:
for subscripts, evaluate the two bottom-right/top-left correction-height sums and
apply their **minimum**. TeXpose keeps that rule, while the pinned LuaTeX
reference uses its current greater-sum selection. This is the same external G7
policy split already ratified for `hard-logit`, `hard-wide-expression`, and
`hard-radical-index-complex`; the surrounding fraction and delimiter only
propagate the local horizontal difference.

Unlike Stage 13c, this propagation also exceeds the global outer tolerance. The
measured outer width is `20.921989em` versus `20.990964em`, a signed
`-0.068974em` delta, while ascent/descent agree within `0.000001em` and glyph/rule
counts remain identical (`45/45`, `1/1`). Stage 13d therefore adds both a STIX
stress outer-geometry ceiling and a positioned-trace ceiling of `0.069100em` for
`hard-depth-typography`. No structural or glyph-selection difference is
permitted, and the global `0.050em` tolerance is unchanged.

Reopen this decision if the published OpenType MathKern rule changes, LuaTeX
returns to minimum-of-sums selection, STIX changes the relevant `y` MathKern
data, TeXpose no longer selects the specified minimum, the first causal split
moves away from `y_q`, or either measured delta exceeds the ceiling. All other
G12 residuals remain separate work.

## Stage-13e STIX radical-scripted MathKern classification

Stage 13e closes the final geometry-comparable STIX positioned residual without
changing TeXpose layout. The measured stress case is:

```tex
\sqrt[7]{\frac{x_i^2+y_j^2}{1+\alpha}}
```

The positioned trace is topology- and identity/order-aligned for all 14
primitives. Primitives 0--8 agree within `0.000013em`; the first material split
is primitive 9, the level-1 `j` subscript in `y_j`. TeXpose places it at
`3.865144em` while the pinned LuaLaTeX reference places it at `3.934132em`, a
signed `-0.068988em` x delta. Baseline and scale are identical. The following
fraction rule and denominator return to rounding-level agreement, so the radical
geometry itself is not the causal source.

The local pair is the same STIX MathKern case reconstructed in Stage 13c. Glyph
`3355` (`y`) has advance `0.510em`; glyph `4432` is the level-1 `j` script
alternate. Relative to the `y` origin, TeXpose places `j` at `+0.409000em`,
corresponding to the `-0.101em` minimum candidate, while the reference places it
at approximately `+0.478001em`, corresponding to the `-0.032em` greater-sum
path up to reference rounding. The local split is therefore approximately
`0.069001em`.

The governing contract remains the published OpenType MATH MathKern algorithm:
evaluate the two correction-height sums and apply their **minimum**. TeXpose
keeps that rule; the pinned LuaTeX reference uses its current greater-sum
selection. This is the same external G7 policy split already ratified for
`hard-logit`, `hard-wide-expression`, `hard-radical-index-complex`, and
`hard-depth-typography`.

The outer box is already within the global tolerance: width differs by only
`0.000015em`, ascent by `0.000001em`, and descent by `0.000002em`, with identical
glyph/rule counts (`12/12`, `2/2`). Stage 13e therefore adds only a STIX stress
positioned-trace ceiling of `0.069100em` for `hard-radical-scripted`. No outer
geometry deviation, structural difference, or glyph-selection difference is
permitted, and the global `0.050em` tolerance is unchanged.

Reopen this decision if the published OpenType MathKern rule changes, LuaTeX
returns to minimum-of-sums selection, STIX changes the relevant `y`/`j` MathKern
data, TeXpose no longer selects the specified minimum, the first causal split
moves away from `y_j`, or the positioned delta exceeds the ceiling. The
remaining G12 work is the non-comparable/outer inventory and cross-profile
closure, not another geometry-comparable STIX positioned residual.

## Stage-14a STIX non-comparable outer-geometry closure

Stage 14a closes the remaining STIX **outer** geometry inventory without changing
TeXpose layout. Both cases already contain ratified G5 glyph-selection
differences, so their full positioned traces remain formally non-comparable and
Stage 14a adds **no** positioned-trace ceiling. The outer deltas can nevertheless
be decomposed into independently owned G5/G7 contracts.

### `hard-stat-r2`

The measured stress case is:

```tex
R^2=1-\frac{\sum_{i=1}^{n}\left(y_i-\widehat y_i\right)^2}
{\sum_{i=1}^{n}\left(y_i-\bar y\right)^2}
```

Outer geometry is `10.278011em` by `1.861000em`/`1.313000em` in TeXpose versus
`10.487970em` by `1.829002em`/`1.313002em` in the pinned reference. The signed
deltas are therefore `-0.209959em` width, `+0.031998em` ascent, and
`-0.000002em` descent. Glyph/rule counts agree (`32/32`, `1/1`). Primitive 14
retains the already-ratified G5 widehat pair `1395/732`, which is why the whole
case is not eligible for positioned-geometry comparison.

Before that G5 split, the first material horizontal divergence is already
isolated at primitive 12, the level-1 `i` in the first `y_i`. Relative to the
`y` origin, TeXpose places `i` at `+0.465000em`; the reference places it at
approximately `+0.570002em`. STIX gives `y` an advance of `0.510em` and its
bottom-right MathKern table exposes `-0.045em` and `+0.060em` at the two
correction heights relevant to this attachment; the script alternate contributes
no top-left MathKern. TeXpose therefore selects the OpenType-specified minimum
(`0.510 - 0.045 = 0.465em`), while LuaTeX selects the greater sum
(`0.510 + 0.060 ~= 0.570em`). The same step occurs again on the
`\widehat y_i` term, producing the approximately `0.210em` numerator/outer-width
difference. The `0.031998em` ascent difference remains below the global outer
tolerance and needs no separate policy record.

Stage 14a therefore adds only a stale-sensitive STIX **outer** ceiling of
`0.210100em` for `hard-stat-r2`. The existing G5 accent contract and the focused
MathKern tests in `src/layout/internal_tests/math_kern.rs` remain the primary owners of the underlying
behavior.

### `hard-aligned-model`

The measured stress case is:

```tex
\begin{aligned}
\widehat{J}_G&=b_0+b_EE+b_RR+b_PP+b_DD+b_VV+\varepsilon,\\
R^2&=1-\frac{\sum_i(y_i-\widehat y_i)^2}{\sum_i(y_i-\bar y)^2}.
\end{aligned}
```

Outer geometry is `19.659622em` by `2.656500em`/`2.140500em` in TeXpose versus
`19.869530em` by `2.442154em`/`1.926154em` in the pinned reference. The signed
deltas are `-0.209908em` width, `+0.214346em` ascent, and `+0.214347em`
descent, with matching glyph/rule counts (`56/56`, `1/1`). Primitives 0 and 40
carry the already-ratified G5 widehat pair `1395/732`, so no whole-case
positioned ceiling is permitted.

The horizontal contribution is independently visible in same-glyph subexpressions.
At the first `b_0`, primitive 4 (`b`) is aligned within `0.000015em`; primitive 5
(the subscript) is then `0.034983em` left of the reference. Relative to the `b`
origin, TeXpose uses `+0.503000em` while the reference uses `+0.537999em`. STIX
gives `b` an advance of `0.518em` and bottom-right MathKern candidates
`-0.015em` and `+0.020em`, so this is again the OpenType minimum versus LuaTeX
greater-sum split. The step repeats for `b_E`, `b_R`, `b_P`, `b_D`, and `b_V`,
reaching approximately `0.210em` by the end of the first row.

The vertical contribution is likewise an already-isolated G7 policy split. In
the second-row denominator, primitives 54/55 are the plain right parenthesis and
its superscript `2`. TeXpose raises `2` by exactly `0.506000em`
(`-1.211200 - (-1.717200)`), while the reference raises it by approximately
`0.252001em` (`-1.250851 - (-1.502852)`). This is the same
`ExtendedShapeCoverage` / `superscriptBaselineDropMax` versus
`superscriptShiftUpCramped` split reconstructed in Stage 13b. The first-row
widehat selection is independently owned by the G5 accent contract.

Those child-box differences are then propagated by the established aligned-stack
policy: `center_amsmath_stack` centers the complete stack on the math axis and
`aligned_interrow_gaps` derives the inter-row gap from the laid-out row extents.
`src/layout/internal_tests/amsmath_grid.rs::aligned_uses_jot_lineskip_and_centers_complete_stack`
protects that behavior. There is therefore no independent alignment-policy bug to
repair.

Stage 14a adds a stale-sensitive STIX **outer** ceiling of `0.214500em` for
`hard-aligned-model`. It does not permit new glyph-selection differences or any
positioned geometry mismatch beyond the separately ratified G5 signatures.

The governing external rules remain OpenType MATH: MathKern uses the minimum of
the two correction-height sums, and `superscriptBaselineDropMax` applies to bases
treated as a box or extended shape. Reopen either record if those published
rules change, LuaTeX converges to them, the relevant STIX MATH data changes,
TeXpose stops satisfying the focused G5/G7/aligned-stack contracts, the causal
split moves, or either measured outer delta exceeds its ceiling.

## Final Libertinus/Fira cross-profile closure block

This is one G12 closure block grouped by causal contract, not a sequence of
case-by-case waivers. The pre-fix full-profile traces left three kinds of work:
real TeXpose math-alphabet/spacing defects in Fira, already-established G7
ExtendedShape policy splits, and two exact G5 glyph-selection signatures.

### Semantic repairs: Fira math alphabets and control-space

`hard-brutal-core` exposed a real direct-nucleus bug in
`\widehat{\mathbf X_{ij}^{2}}`. The bold `X` itself (primitive 23) matched the
reference, but TeXpose had packed every `MathAlphabet` into an `HList`. G7 then
misclassified the one-character `\mathbf X` as a box and applied the box
baseline-drop path. The accent and attached scripts at primitives 22, 24, 25 and
26 were consequently `0.194000--0.194001em` below the reference even though the
base glyph was aligned. A one-character non-`\pmb` math alphabet now remains a
direct glyph nucleus; the focused regression compares paired script shifts for
the direct U+1D417 bold `X` and `\mathbf X` in Fira `TextCramped`. No deviation is
added for this case.

`hard-overbrace`, source
`\overbrace{a+b+c+d+e}^{n\ \mathrm{terms}}`, exposed two independent horizontal
defects in the annotation. The reference `t` begins about `0.765em` after `n`;
after the scaled `n` glyph contribution, the residual is one root interword space
of approximately `0.333em`. TeXpose had incorrectly style-scaled its fixed
one-third-em control-space approximation to `0.24em` at Fira's `0.72` script
scale. In the same run, Fira gives upright `t` and `r` a 20-unit MATH italic
correction; at `0.72` scale each contributes `0.0144em`, exactly the two later
reference steps that TeXpose had dropped. Control-space therefore keeps the root
one-third-em approximation, and multi-character math alphabets materialize each
completed glyph's MATH italic correction once. Focused spacing and row-italic
regressions own both repairs. No deviation is added for `hard-overbrace`.

### G7 ExtendedShape propagation across profiles

The remaining identity-comparable vertical residuals are the same published
OpenType MATH baseline-drop policy split already reconstructed for STIX
`hard-sum-substack`: TeXpose applies `superscriptBaselineDropMax` to a box or
`ExtendedShapeCoverage` base, while the pinned LuaTeX glyph/simple path uses the
ordinary cramped superscript shift on the affected `(...)^2`.

The bounded records are deliberately narrow:

| Profile | Case | Scope | Observed | Ceiling |
| --- | --- | --- | ---: | ---: |
| Libertinus | `hard-sum-substack` | positioned | 0.190000em | 0.190200em |
| Libertinus | `hard-aligned-model` | outer only | 0.185999em | 0.186200em |
| Libertinus | `hard-stat-r2` | outer only | 0.162499em | 0.162700em |
| Fira | `hard-sum-substack` | positioned | 0.144721em | 0.144900em |
| Fira | `hard-aligned-model` | outer + positioned | 0.079863em | 0.080100em |

The two Libertinus outer-only cases contain the separately pinned G5 glyph
signatures above and therefore receive no whole-case positioned ceiling. Fira
`hard-aligned-model` is identity-aligned, so the same bounded value may protect
both outer and positioned geometry. The aligned-stack behavior remains owned by
`amsmath_grid::aligned_uses_jot_lineskip_and_centers_complete_stack`; G7 remains
owned by `script_placement::extended_shapes_and_box_bases_use_ink_box_baseline_drop_constraints`.

### Acceptance and reopening

The closure candidate is accepted only if canonical and stress `--fail-on-delta`
runs pass for STIX, Libertinus, and Fira after these semantic repairs. The exact
glyph records are stale-sensitive and the geometry ceilings do not rewrite raw
measurements. Reopen this block if a classified glyph pair changes/disappears, a
recorded delta exceeds its ceiling, the OpenType MATH contracts change, LuaTeX
converges to those contracts, or either focused semantic regression fails.
