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

## MathValueRecord Device policy

For every supported `MathValueRecord`, TeXpose uses only the signed design-unit `Value` field. The optional Device table is deliberately ignored in the first stable core.

This makes mathematical layout independent of pixels-per-em, display DPI, and pixel-grid hinting. A future policy that applies Device or VariationIndex corrections would be a semantic layout change and must update this document and its contract tests.
