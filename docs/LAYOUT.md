# Layout contract

TeXpose layout is backend-neutral and operates in exact `Dim` values derived from font design units. Rendering, DPI selection, rasterization, and pixel snapping belong to consumers.

## Validated font boundary

`MathFont` construction validates the selected OpenType face, functional-variable-font policy, physical `MATH` table, and mandatory MathConstants. A layout operation therefore receives a font for which MATH and MathConstants are invariants, and parses one temporary face view for the complete operation. Missing or malformed mandatory MATH input is a construction error, not a layout fallback.

## MATH value conversion

OpenType `MathValueRecord` contains a design-unit value plus an optional Device/VariationIndex correction. TeXpose converts the design-unit value to `Dim` and ignores the optional correction. No layout result depends on PPEM, display DPI, or pixel-grid state.

All validated face design-unit conversion used by layout goes through one `MathFontView` conversion boundary. This includes ordinary glyph metrics, MATH constants, glyph-info values, `MathGlyphVariantRecord.advanceMeasurement`, and glyph-assembly connector/full-advance/italic-correction values. Construction data is represented internally as typed `MathVariant`, `AssemblyPart`, and `GlyphAssembly` values carrying `Dim`; layout does not reinterpret assembly tuples in raw font units.

The public `MathFont::horizontal_assembly_parts` method remains a compatibility view that explicitly returns raw font units. It is not used by layout and is not a second layout-unit conversion path. Both that compatibility view and the typed layout path reject assembly part counts above the internal `MAX_ASSEMBLY_PARTS` budget before proportional allocation.

## Bounded glyph assemblies

Horizontal and vertical OpenType MATH assemblies share one exact bounded solver. For each requested extent, TeXpose validates every part's full advance, connector lengths, and reserved part flags, enforces `MathVariants.minConnectorOverlap`, rejects constructions without an extender, and computes the legal minimum/maximum assembly advance for an extender repetition count without materializing the repeated sequence. Extenders are added in uniform rounds, preserving the OpenType part order. The solver establishes monotonic legal growth, then uses a bounded binary search for the first repetition count whose legal interval reaches the target.

Within the selected repetition count, maximum connector overlap gives the compact limit and `minConnectorOverlap` gives the expanded limit. TeXpose uses exact `Dim` arithmetic to choose the interpolation ratio needed by the target, materializes only the final validated sequence, and never allocates more than `MAX_ASSEMBLY_PARTS`. Horizontal placement advances by each part's MATH `fullAdvance` minus its solved overlap rather than by incidental `hmtx` width. Vertical parts are consumed bottom-to-top and use the same solved growth advances; the generic assembly exposes its solved extent and each owning construct assigns that extent to its own baseline geometry.

Malformed (including declared-but-unparseable), connector-invalid, non-growing, non-monotonic, arithmetic-invalid, missing-part, reserved-flag, or over-budget assembly data is recoverable. The owning construct keeps its largest valid ready-made variant and emits `LayoutDiagnostic::ExtensibleFallback { ch }`. A glyph with no assembly data simply keeps its ready-made fallback without this diagnostic.

## Vertical MATH variants and delimiters

Ready-made vertical variants are selected by `MathGlyphVariantRecord.advanceMeasurement`, which is the OpenType growth-direction measurement, not by the glyph ink bounding box. The base glyph remains the initial degradation candidate. For prepared variants, TeXpose selects the smallest valid advance meeting the requested extent; if none meets it, it retains the largest available candidate. Internal construction glyph IDs remain strict source glyphs under the missing-glyph policy.

`\left...\right` derives its requested extent from the TeX delimiter-factor and physical delimiter-shortfall rules, then centers the selected glyph on `AxisHeight`. The explicit `\big` / `\Big` / `\bigg` / `\Bigg` family follows the amsmath fixed-delimiter construction: `big@size` is 1.2 times the textstyle math-parenthesis height+depth, the four commands apply factors 1 / 1.5 / 2 / 2.5, and the resulting centered extent is passed through the same TeX delimiter-factor/shortfall rule before selecting a MATH variant. Because amsmath measures this construction in a fresh inline math formula, explicit-big glyph sizing and axis centering remain textstyle even when the surrounding formula is scriptstyle. Empty delimiters remain zero-size boxes. `delimitedSubFormulaMinHeight` remains an explicit coverage gap; OpenType defines the value but does not specify a complete delimiter-sizing algorithm, so G2 does not silently replace the project-owned TeX target rule with that ambiguous constant.

## Radical geometry

Radicals size U+221A from the cramped radicand span plus the style-appropriate `RadicalVerticalGap` / `RadicalDisplayStyleVerticalGap` and `RadicalRuleThickness`. `RadicalExtraAscender` is reserved above the finished rule and is not part of the MATH variant/assembly target. If the selected ready-made variant or vertical assembly is taller than the minimum request, the excess is split into the effective gap and surd descent using the TeX radical construction rule.

The radical bar spans the complete terminal horizontal extent of the radicand. A direct math-character box carries its terminal MATH italic correction separately, while a packed row has already materialized that correction as a kern; the radical boundary therefore adds the box-level italic correction exactly once when establishing the bar and radical width.

Indexed radicals use `RadicalKernBeforeDegree`, `RadicalKernAfterDegree`, and `RadicalDegreeBottomRaisePercent`. The degree is laid out in scriptscript style. A negative after-degree kern is bounded so `before + degree width + after` cannot become negative, preventing a narrow degree from pulling the radical sign left of the degree origin. The vertical raise is measured from the bottom of the corrected surd span after radical-gap redistribution. These rules apply identically when U+221A is a ready-made glyph or a solved vertical assembly.

Root-em changes do not alter normalized MATH geometry: the radical glyph/assembly and rule remain expressed in em units at 6pt, 10pt, 20pt, and 40pt. Physical-length constructs elsewhere continue to resolve through the root-em boundary.

## Mathematical GSUB selection

Math glyph substitutions are not selected by globally searching the GSUB FeatureList. TeXpose first resolves the `math` Script table, then its `DefaultLangSys`, then only the required/optional feature indices reachable from that language system. Active lookups are applied in LookupList order.

Script and scriptscript glyphs activate `ssty`; over-accent glyphs activate `flac` only when their base is above `flattenedAccentBaseHeight`; and a single mathematical glyph used as an over-accent base activates `dtls`. These contextual features are combined with `ssty` before lookup-order application rather than being applied as independent post-processing passes.

For `ssty`, level 1 selects the first Alternate Substitution form and level 2 selects the second. A Single Substitution is the documented fallback when one script form serves both levels. Glyphs outside active `ssty` coverage retain their original glyph ID. In all cases TeXpose resolves the substitution before reading the final glyph advance, bounding box, and italic correction, then applies `scriptPercentScaleDown` or `scriptScriptPercentScaleDown` to those selected-glyph metrics.

## Script positioning and MathKern

Vertical script positions are established from the MATH script constants before horizontal kerning. `superscriptBottomMin` and `subscriptTopMax` apply to every script. A composite base, or a direct glyph covered by `ExtendedShapeCoverage`, also applies `superscriptBaselineDropMax` / `subscriptBaselineDropMin` against the base ink box. An ordinary direct glyph does not acquire those box-only baseline-drop constraints merely because it is tall.

TeXpose then applies OpenType `MathKernInfo` to each direct-glyph side of the attachment. Superscripts start after the base advance plus its MATH italic correction; subscripts start immediately after the base advance. At each of the two OpenType correction lines, the appropriate base and script corner values are added, and the smaller of the two sums is used as the horizontal kern. Correction-height boundaries are converted to the physical layout scale of the glyph they belong to before comparison; equality belongs to the interval above that boundary. Missing glyph coverage or a missing corner table contributes zero. A boxed side contributes zero for its own corner while a direct glyph on the other side still contributes its height-dependent corner kern, as permitted by OpenType.

Superscript and subscript horizontal origins are represented independently. A base italic correction therefore never shifts a paired subscript merely because a superscript is also present.

## Missing-glyph degradation

A missing cmap entry for a Unicode scalar required by ordinary math or literal-text layout is recoverable. The diagnostic-aware entry points return `LayoutDiagnostic::MissingGlyph { ch }` in deterministic traversal order and continue layout. The existing convenience entry points (`layout`, `layout_with_em_size_pt`, numbering variants, and `layout_with_max_depth`) preserve their `MathBox` return type and deliberately discard recoverable diagnostics; callers that need to surface degradation must use the corresponding `*_with_diagnostics` entry point.

When cmap has no entry, TeXpose first tries OpenType glyph id 0 (`.notdef`). Glyph 0 is usable for this purpose only when `hmtx` supplies a non-zero horizontal advance. Its advance and available bounding box are scaled exactly like an ordinary glyph, while italic correction is zero. If glyph 0 has no usable horizontal advance, TeXpose emits a non-rendering placeholder box whose width and height are exactly one current math em and whose depth and italic correction are zero. This makes fallback geometry deterministic across runs without inventing font-specific outline data.

This policy applies only to required Unicode scalars. Internal source glyphs used to build mathematical constructions are not silently converted to `.notdef`: a missing ready-made variant is skipped, a missing glyph-assembly part abandons that assembly with `ExtensibleFallback`, accent candidate lists keep trying their documented alternatives, and `\not` keeps its U+0338 then `/` fallback. A missing ordinary glyph that has already degraded to glyph 0 is not used as a seed for MATH variants or assemblies. Direct `MathFont::glyph` remains strict and reports `FontError::MissingGlyph`.
