# Font API contract

`MathFont` is the stable public boundary between caller-owned OpenType data and TeXpose layout. The concrete OpenType parser used internally is an implementation detail and is not re-exported.

## Construction and ownership

The canonical ownership constructor is:

```rust
MathFont::from_shared_bytes(raw: Arc<[u8]>, face_index: u32)
```

It retains the supplied allocation by reference counting without copying. `MathFont::clone` is therefore cheap and preserves byte identity. `from_bytes` and `from_bytes_at_index` are convenience constructors for borrowed slices; they copy into the same shared representation. `from_bytes` accepts only a standalone OTF/TTF face and reports `FontError::CollectionFaceIndexRequired` for TTC/OTC input so collection selection cannot be implicit. `from_bytes_at_index` and `from_shared_bytes` accept an explicit face index for standalone fonts and collections.

Construction validates the selected static face, rejects functional variable-font axes, validates a physical OpenType `MATH` table, validates mandatory MathConstants, and proves a non-zero `unitsPerEm`. A successfully constructed `MathFont` therefore carries those invariants for every later layout operation.

## Stable read-only identity

The stable inspection surface is:

```rust
MathFont::bytes(&self) -> &[u8]
MathFont::shared_bytes(&self) -> Arc<[u8]>
MathFont::face_index(&self) -> u32
MathFont::units_per_em(&self) -> u16
```

The bytes and face index form the exact OpenType face identity. `units_per_em` is the validated denominator needed to convert emitted glyph outlines from font design units into root-em units. These values are read-only; callers cannot mutate the validated face through `MathFont`.

`MathLayout` owns a cheap clone of the exact `MathFont` used during layout. A native renderer obtains the bytes, face index, and units-per-em through `MathLayout::font()` and resolves every emitted glyph id against that exact face. The renderer may use any OpenType library that can parse those bytes and address the retained face index.

## What is intentionally private

TeXpose does not expose `ttf_parser::Face`, re-export the `ttf-parser` crate, or publish raw cmap, glyph-metric, MATH variant, MATH assembly, MathKern, accent-attachment, or GSUB query helpers. Those operations are layout implementation details. Keeping them private avoids coupling consumers to a dependency version or to intermediate representations that TeXpose may change without altering the display-list contract.

The public `MathOp::Glyph` contract is sufficient for rendering: it supplies the final OpenType glyph id, position, scale, and color, while the layout-associated `MathFont` supplies exact face identity.

## Errors and diagnostics

`FontError` is typed and non-exhaustive. Current construction failures distinguish invalid face data, missing collection selection, an out-of-range face index, unsupported variable faces, missing versus malformed `MATH`, and missing versus malformed MathConstants. `MissingGlyph { ch }` remains the strict internal glyph-lookup failure carried by `Error::Font` when a layout path requires a source glyph that cannot degrade.

Font-data problems discovered during public layout can be recoverable diagnostics rather than construction failures. Missing cmap entries are reported as `LayoutDiagnostic::MissingGlyph { ch }`. Unusable extensible MATH assembly data degrades to the largest valid ready-made variant and reports `LayoutDiagnostic::ExtensibleFallback { ch }`. Neither event mutates or invalidates the retained `MathFont`; consumers must inspect `MathLayout::diagnostics()` for recoverable events.

Because `FontError` is non-exhaustive, external matches must retain a fallback arm. Error text is diagnostic presentation; callers that need semantics should match typed variants instead of parsing `Display` strings.

## Stability boundary

The stable contract is the validated immutable font handle, its construction errors, exact face identity, units-per-em, and the font-bound `MathLayout` consumer path. Internal OpenType parser types and direct metric/query helpers are deliberately outside that contract.
