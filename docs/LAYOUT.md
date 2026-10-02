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
