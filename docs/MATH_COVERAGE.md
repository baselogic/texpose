# OpenType MATH coverage

TeXpose treats the OpenType `MATH` table as validated font input rather than optional layout state. `MathFont` construction inspects the selected face's raw SFNT directory before layout begins.

## Construction contract

The first stable core distinguishes these conditions:

- `FontError::InvalidFace`: the selected bytes are not a usable OpenType face for reasons outside the more specific cases below.
- `FontError::FaceIndexOutOfBounds`: the requested standalone/collection face index does not exist.
- `FontError::VariableFontUnsupported`: the selected face contains functional `fvar` axes.
- `FontError::MissingMathTable`: the selected face has no physical `MATH` table record.
- `FontError::MalformedMathTable`: a `MATH` record exists but its range/header/version cannot be validated or the parser cannot expose it as a MATH table.
- `FontError::MissingMathConstants`: the MATH header has no MathConstants offset.
- `FontError::MalformedMathConstants`: the MathConstants offset/range is invalid or the fixed 214-byte MathConstants payload required by MATH 1.0 is truncated/unparseable.

After successful construction, the layout engine may rely on a MATH 1.0 table with MathConstants being present and parseable. Optional MATH substructures such as glyph info and variants remain optional and are handled according to the operation that consumes them.

## Coverage matrix

Status vocabulary:

- **Implemented** — the field/table is read and affects supported layout.
- **Partial** — only the listed subset/direction is consumed; the degradation column names the unsupported remainder.
- **Not consumed** — TeXpose currently does not read the field/table; supported syntax follows the documented non-MATH fallback instead.
- **Construction invariant** — validated before layout; failure rejects `MathFont` construction rather than degrading later.
- **Intentionally ignored** — parsed/representable input whose effect is deliberately excluded by an existing documented policy.

The matrix is an inventory, not a promise that every OpenType MATH facility is supported. A row marked **Not consumed** or **Partial** is an explicit capability gap, not silent fallback to unspecified behavior. `skewedFractionHorizontalGap` and `skewedFractionVerticalGap` are outside this matrix because TeXpose has no skewed-fraction construct.

G1 normalizes construction records at the font boundary: ready-made variants carry their OpenType growth-direction advance as `Dim`, while assembly parts carry typed connector lengths/full advance and the assembly carries typed italic correction. Raw design-unit tuples are not a layout representation. G2 uses the typed vertical `advanceMeasurement` for ready-made vertical variant selection. G3 adds one bounded exact solver for horizontal and vertical `GlyphAssembly` data, including `minConnectorOverlap`, uniform extender rounds, checked legal-advance ranges, and deterministic fallback diagnostics. G5 applies the same `advanceMeasurement` rule to horizontal variants, resolves accent source candidates against the current face, and falls back to the selected base accent glyph when no horizontal construction exists.

| OpenType field/table | TeXpose construct | Status | Default / degradation policy | Primary test | Oracle family |
| --- | --- | --- | --- | --- | --- |
| MATH header + `mathConstantsOffset` | all math layout | Construction invariant | MATH 1.0 and the fixed MathConstants payload must be present and parseable; construction fails otherwise. Optional glyph-info/variants offsets may be absent. | `tests/font_math_contract.rs` | font construction / all families |
| `scriptPercentScaleDown` | level-1 scripts | Implemented | Required MathConstants scalar; scales script style. | `tests/script_scale.rs` | scripts |
| `scriptScriptPercentScaleDown` | level-2 scripts | Implemented | Required MathConstants scalar; scales scriptscript style. | `tests/script_scale.rs` | scripts |
| `delimitedSubFormulaMinHeight` | `\left...\right`, named/character delimiters | Not consumed | Delimiter target is computed from TeX delimiter factor/shortfall and body geometry; this MATH minimum is not applied. | `tests/delimiter_sizing.rs` (current sizing policy) | delimiters |
| `displayOperatorMinHeight` | display large operators and integrals | Implemented | Select the tightest available vertical variant meeting the requested display minimum; if none meets it, retain the largest available/base glyph. | `tests/large_operator_limits.rs`, `tests/integral_scripts.rs` | operators, integrals |
| `mathLeading` | inter-line mathematical leading | Not consumed | No global MATH leading is added. Matrices/aligned/substack use their explicit TeX/AMSmath spacing rules instead. | `tests/amsmath_grid.rs`, `tests/amsmath_substack.rs` (current spacing policy) | matrices, aligned, operators |
| `axisHeight` | delimiter centering, large operators/integrals, fractions | Implemented | Required MathConstants value; used as the mathematical axis for `\left...\right`, explicit `\big`-family delimiters, operators, and fractions. Explicit-big construction uses the textstyle axis because amsmath measures the fixed delimiter in a fresh inline math formula. Device correction is ignored. | `tests/delimiter_sizing.rs`, `tests/large_operator_limits.rs` | delimiters, operators, fractions |
| `accentBaseHeight` | over-accents | Implemented | Bases at or below the threshold need no diacritic raise; missing per-glyph attachment data still uses geometric horizontal fallback. | `tests/wide_accent_math.rs`, `tests/nested_accent_geometry.rs` | accents |
| `flattenedAccentBaseHeight` | GSUB `flac` eligibility | Implemented | Used only to decide whether an over-accent glyph is eligible for `flac`. Vertical accent placement remains governed by `accentBaseHeight`; `flac` changes shape, not position. | `tests/math_gsub_features.rs`, `tests/wide_accent_math.rs`, `tests/nested_accent_geometry.rs` | accents |
| Subscript constants: `subscriptShiftDown`, `subscriptTopMax`, `subscriptBaselineDropMin` | side subscripts on ordinary glyphs/operators/integrals | Implemented | Constraints are combined as minima/maxima; no synthetic font-independent replacement is used when MathFont construction succeeds. Device corrections are ignored. | `tests/script_placement.rs`, `tests/integral_scripts.rs` | scripts, integrals |
| Superscript constants: `superscriptShiftUp`, `superscriptShiftUpCramped`, `superscriptBottomMin`, `superscriptBaselineDropMax`, `superscriptBottomMaxWithSubscript`, `subSuperscriptGapMin`, `spaceAfterScript` | superscripts, paired scripts, post-script spacing | Implemented | Constraints are combined with base/script geometry. Device corrections are ignored. | `tests/script_placement.rs`, `tests/script_space_after.rs`, `tests/integral_scripts.rs` | scripts, integrals |
| Fraction constants: `fractionNumeratorShiftUp`, `fractionNumeratorDisplayStyleShiftUp`, `fractionDenominatorShiftDown`, `fractionDenominatorDisplayStyleShiftDown`, `fractionNumeratorGapMin`, `fractionNumDisplayStyleGapMin`, `fractionRuleThickness`, `fractionDenominatorGapMin`, `fractionDenomDisplayStyleGapMin` | ruled generalized fractions | Implemented | Required MathConstants values drive ruled fractions; explicit TeX rule thickness overrides only the rule thickness where syntax requests it. Device corrections are ignored. | `tests/fraction_semantics.rs` | fractions |
| Stack constants: `stackTopShiftUp`, `stackTopDisplayStyleShiftUp`, `stackBottomShiftDown`, `stackBottomDisplayStyleShiftDown`, `stackGapMin`, `stackDisplayStyleGapMin` | ruleless generalized fractions; `\substack` row spacing | Implemented | Ruleless fractions use stack shifts/gaps; `\substack` derives its baseline/line spacing from the non-display stack values at script scale. Device corrections are ignored. | `tests/fraction_semantics.rs`, `tests/amsmath_substack.rs` | fractions, operators |
| Stretch-stack constants: `stretchStackTopShiftUp`, `stretchStackBottomShiftDown`, `stretchStackGapAboveMin`, `stretchStackGapBelowMin` | `\overset`, `\underset`, annotated braces/arrows | Not consumed | Over/under placement currently uses upper/lower-limit constants after optional horizontal stretching; stretch-stack constants are not applied. | — | overunder, accents |
| Upper/lower limit constants: `upperLimitGapMin`, `upperLimitBaselineRiseMin`, `lowerLimitGapMin`, `lowerLimitBaselineDropMin` | `\limits`, large-operator limits, `\overset`/`\underset` path | Implemented | Edge-gap and baseline rise/drop constraints are enforced from the base font. Device corrections are ignored. | `tests/large_operator_limits.rs` | operators, overunder |
| Overbar constants: `overbarVerticalGap`, `overbarRuleThickness`, `overbarExtraAscender` | `\overline` | Implemented | Rule/gap/extra ascender come from MATH; Device corrections are ignored. | `tests/accent_golds.rs`, `tests/layout_golds.rs` | accents |
| Underbar constants: `underbarVerticalGap`, `underbarRuleThickness`, `underbarExtraDescender` | `\underline` | Implemented | Rule/gap/extra descender come from MATH for the underline rule path. Under-arrows and underbraces use their selected accent glyph/assembly geometry directly and do not consume underbar constants. Device corrections are ignored. | `tests/accent_golds.rs`, `tests/wide_accent_math.rs` | accents |
| Radical constants: `radicalVerticalGap`, `radicalDisplayStyleVerticalGap`, `radicalRuleThickness`, `radicalExtraAscender`, `radicalKernBeforeDegree`, `radicalKernAfterDegree`, `radicalDegreeBottomRaisePercent` | square roots and indexed radicals | Implemented | Required constants drive rule/gap/degree placement. The bar preserves the radicand terminal italic extent exactly once; discrete variant/assembly slack is redistributed into gap/descent; narrow degrees clamp the negative after-kern before it can pull the surd left. Ready-made variants and the bounded vertical assembly solver share the same radical-specific geometry. Device corrections are ignored. | `tests/radical_geometry.rs`, `tests/radical_variant.rs`, canonical/stress positioned traces | radicals, radical-degree |
| `MathItalicsCorrectionInfo` | row italic kerns and side-script attachment | Partial | Missing table/coverage/value means zero italic correction. TeXpose uses the value for row/script placement, including integral side scripts, but does not apply the OpenType half-correction rule to above/below large-operator limits. Device correction is ignored. | `tests/row_math_italic.rs`, `tests/integral_scripts.rs` | scripts, integrals |
| `MathTopAccentAttachment` | horizontal accent attachment | Implemented | Missing table/coverage/value falls back to the OpenType geometric attachment: the center of the glyph/box advance width on each side. Assemblies therefore center by their solved width. Device correction is ignored. | `tests/wide_accent_math.rs`, `tests/nested_accent_geometry.rs` | accents |
| `ExtendedShapeCoverage` | extended-shape classification | Not consumed | No behavior currently branches on ExtendedShape coverage. Stretching is driven directly by syntax plus MathVariants data. | — | delimiters, radicals, accents |
| `MathKernInfo` | corner math kerning for scripts | Not consumed | No MATH corner-kern adjustment is applied; scripts use constant-based vertical constraints plus italic correction. | — | scripts |
| `MathVariants.minConnectorOverlap` | glyph assemblies | Implemented | Every solved connection must support at least the font-wide minimum overlap. The solver ranges each repetition count from maximum legal connector overlap to this minimum and rejects connector-invalid construction data before materialization. | `layout::assembly::tests`, `tests/glyph_assembly.rs` | delimiters, accents, overunder |
| Vertical `MathGlyphConstruction` variants | delimiters, radicals, display operators/integrals | Implemented | Ready-made vertical variants are typed as `(glyph ID, advanceMeasurement)` in exact `Dim`. Selection uses the MATH advance in the growth direction, chooses the tightest valid variant meeting the target, and otherwise retains the largest available/base glyph as deterministic degradation. | `tests/delimiter_sizing.rs`, `tests/radical_variant.rs`, `tests/large_operator_limits.rs`, `layout::engine::tests::variant_measure_prefers_tightest_satisfying_or_largest_short_candidate` | delimiters, radicals, operators, integrals |
| Horizontal `MathGlyphConstruction` variants | wide accents, arrows/braces and other horizontally stretchy glyphs | Implemented | Ready-made horizontal variants are selected by the typed MATH `advanceMeasurement`: choose the tightest variant meeting the target. If all ready-made variants are short, the engine tries the horizontal `GlyphAssembly`; absent or unusable assembly data retains the largest valid/base glyph. Circumflex/tilde source candidates are resolved semantically against the current face. If no candidate exposes a horizontal construction, layout retains the selected base accent glyph and does not synthesize stretching. | `tests/wide_accent_math.rs`, `tests/glyph_assembly.rs`, `layout::engine::tests::variant_measure_prefers_tightest_satisfying_or_largest_short_candidate` | accents, overunder |
| `GlyphAssembly` horizontal parts | horizontally stretchy glyphs after variants are insufficient | Implemented | Typed parts are validated and solved with exact full advances, reserved-flag rejection, legal connector overlap, uniform extender rounds, monotonic bounded search, and a hard `MAX_ASSEMBLY_PARTS` allocation budget. Invalid/unusable assemblies retain the largest valid prebuilt and emit `ExtensibleFallback`; valid assemblies preserve their MATH italics correction. | `layout::assembly::tests`, `tests/glyph_assembly.rs`, canonical positioned traces | accents, overunder |
| `GlyphAssembly` vertical parts | vertically stretchy delimiters/radicals/operators | Implemented for generic assembly solving | The same bounded solver consumes bottom-to-top vertical parts after ready-made variants are insufficient; a declared-but-unparseable raw assembly is distinguished from a genuinely absent assembly and degrades diagnostically. Delimiters center the result on `axisHeight`; radicals consume the solved extent through their G4 gap/rule/degree geometry; G8 still owns large-operator-specific placement. Invalid/unusable assemblies retain the largest valid prebuilt and emit `ExtensibleFallback`. | `layout::assembly::tests`, `tests/glyph_assembly.rs`, `tests/radical_geometry.rs`, stress positioned traces | delimiters, radicals, operators |
| `MathValueRecord.deviceOffset` | every implemented MathValueRecord above | Intentionally ignored | Only the signed design-unit value is used; PPEM/device corrections do not affect layout. | `tests/font_math_contract.rs` | all affected families |

### Gaps exposed by the inventory

E9 deliberately records incomplete MATH support rather than silently treating it as complete. The current gaps relevant to supported syntax are:

- `delimitedSubFormulaMinHeight`, `mathLeading`, stretch-stack constants, `ExtendedShapeCoverage`, and `MathKernInfo` are not consumed.
- `MathItalicsCorrectionInfo` is only partially consumed: above/below large-operator limits do not apply the OpenType half-correction placement rule.
- Generic assembly solving consumes `minConnectorOverlap`, validates legal overlap intervals, enforces `MAX_ASSEMBLY_PARTS`, and supports both growth directions. G4 and G5 now own the radical-specific and horizontal-extensible geometry around that solver; G8 still owns large-operator-specific geometry.

These entries are inventory facts for later roadmap work. E9 does not change their behavior.

## MathValueRecord Device policy

For every supported `MathValueRecord`, TeXpose uses only the signed design-unit `Value` field. The optional Device table is deliberately ignored in the first stable core.

This makes mathematical layout independent of pixels-per-em, display DPI, and pixel-grid hinting. A future policy that applies Device or VariationIndex corrections would be a semantic layout change and must update this document and its contract tests.

## Mathematical GSUB features

TeXpose resolves mathematical GSUB substitutions only through the `math` Script table and its `DefaultLangSys`. A feature that exists in the GSUB FeatureList but is not referenced by that language system is not active for math layout, and the generic `DFLT` script is not used as a substitute for the required `math` script.

The selected language system contributes both its `requiredFeatureIndex`, when present, and its ordinary feature indices. When more than one active mathematical feature references lookups for the same glyph, the selected lookup set is applied in LookupList order.

The first stable core supports:

- `ssty`: script level 1 and 2 through Alternate Substitution; Single Substitution is accepted as the fallback form when one script form serves both levels.
- `flac`: Single Substitution for over-accent glyphs when the laid-out base exceeds `flattenedAccentBaseHeight`.
- `dtls`: Single Substitution for a single mathematical glyph used as an over-accent base. Multi-glyph accent nuclei are left unchanged by `dtls`.

Other substitution formats for these three features are ignored. Variable-feature substitutions are outside this core because functional variable fonts are rejected during `MathFont` construction.
