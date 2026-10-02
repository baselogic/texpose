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

## Mathematical GSUB features

TeXpose resolves mathematical GSUB substitutions only through the `math` Script table and its `DefaultLangSys`. A feature that exists in the GSUB FeatureList but is not referenced by that language system is not active for math layout, and the generic `DFLT` script is not used as a substitute for the required `math` script.

The selected language system contributes both its `requiredFeatureIndex`, when present, and its ordinary feature indices. When more than one active mathematical feature references lookups for the same glyph, the selected lookup set is applied in LookupList order.

The first stable core supports:

- `ssty`: script level 1 and 2 through Alternate Substitution; Single Substitution is accepted as the fallback form when one script form serves both levels.
- `flac`: Single Substitution for over-accent glyphs when the laid-out base exceeds `flattenedAccentBaseHeight`.
- `dtls`: Single Substitution for a single mathematical glyph used as an over-accent base. Multi-glyph accent nuclei are left unchanged by `dtls`.

Other substitution formats for these three features are ignored. Variable-feature substitutions are outside this core because functional variable fonts are rejected during `MathFont` construction.
