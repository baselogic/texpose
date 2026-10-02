# Layout contract

TeXpose layout is backend-neutral and operates in exact `Dim` values derived from font design units. Rendering, DPI selection, rasterization, and pixel snapping belong to consumers.

## Validated font boundary

`MathFont` construction validates the selected OpenType face, functional-variable-font policy, physical `MATH` table, and mandatory MathConstants. A layout operation therefore receives a font for which MATH and MathConstants are invariants, and parses one temporary face view for the complete operation. Missing or malformed mandatory MATH input is a construction error, not a layout fallback.

## MATH value conversion

OpenType `MathValueRecord` contains a design-unit value plus an optional Device/VariationIndex correction. TeXpose converts the design-unit value to `Dim` and ignores the optional correction. No layout result depends on PPEM, display DPI, or pixel-grid state.

This policy applies consistently to MATH constants and glyph-info values currently consumed by the engine, including italic correction and top-accent attachment.

## Mathematical GSUB selection

Math glyph substitutions are not selected by globally searching the GSUB FeatureList. TeXpose first resolves the `math` Script table, then its `DefaultLangSys`, then only the required/optional feature indices reachable from that language system. Active lookups are applied in LookupList order.

Script and scriptscript glyphs activate `ssty`; over-accent glyphs activate `flac` only when their base is above `flattenedAccentBaseHeight`; and a single mathematical glyph used as an over-accent base activates `dtls`. These contextual features are combined with `ssty` before lookup-order application rather than being applied as independent post-processing passes.

## Missing-glyph degradation

A missing cmap entry for a Unicode scalar required by ordinary math or literal-text layout is recoverable. The diagnostic-aware entry points return `LayoutDiagnostic::MissingGlyph { ch }` in deterministic traversal order and continue layout. The existing convenience entry points (`layout`, `layout_with_em_size_pt`, numbering variants, and `layout_with_max_depth`) preserve their `MathBox` return type and deliberately discard recoverable diagnostics; callers that need to surface degradation must use the corresponding `*_with_diagnostics` entry point.

When cmap has no entry, TeXpose first tries OpenType glyph id 0 (`.notdef`). Glyph 0 is usable for this purpose only when `hmtx` supplies a non-zero horizontal advance. Its advance and available bounding box are scaled exactly like an ordinary glyph, while italic correction is zero. If glyph 0 has no usable horizontal advance, TeXpose emits a non-rendering placeholder box whose width and height are exactly one current math em and whose depth and italic correction are zero. This makes fallback geometry deterministic across runs without inventing font-specific outline data.

This policy applies only to required Unicode scalars. Internal source glyphs used to build mathematical constructions are not silently converted to `.notdef`: delimiter/radical/operator sizing keeps its strict source-glyph policy, accent candidate lists keep trying their documented alternatives, and `\not` keeps its U+0338 then `/` fallback. A missing ordinary glyph that has already degraded to glyph 0 is not used as a seed for MATH variants or assemblies. Direct `MathFont::glyph` remains strict and reports `FontError::MissingGlyph`.
