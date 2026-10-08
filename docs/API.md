# Public API

## Parser

## Entry points

The supported parser entry points are:

```rust
parse(source: &str) -> Result<MathNode, ParseError>
parse_with_options(source: &str, options: &ParseOptions) -> Result<MathNode, ParseError>
```

`parse` is the normal entry point. `parse_with_options` changes only parser resource budgets. Both return the same typed `MathNode` tree. Color definitions are parse-local state: `\definecolor` affects later color specifications in the same source, and resolved `Color` values are stored directly in the returned AST. The mutable color table is not a second parse result and is not public parser state.

The following former implementation/compatibility surfaces are deliberately not public API:

```text
parse_with_colors
preprocess
tokenize / tokenize_spanned
Token / SpannedToken
format_tokens
parser implementation modules
```

There is no semantic preprocessing stage. The parser tokenizes the exact source string it receives. This keeps every `SourceSpan` directly meaningful without a rewrite map.

## ParseOptions

`ParseOptions` is a read-only configuration value. Construct it with `ParseOptions::new()` or `Default`, change one budget with a `with_*` builder, and inspect configured values with getters. Its fields are private so callers cannot depend on representation layout and future validation can remain inside the owning type.

The five current budgets are:

```text
max_depth
max_ast_nodes
max_environment_rows
max_environment_cells
max_tokens
```

Their default constants remain public because callers may need to size an admission policy relative to the crate defaults. Raising `max_depth` can increase required thread stack; the existing depth contract remains unchanged.

## SourceSpan and ParseError

`SourceSpan` is a TeXpose-produced, read-only half-open byte range `[start, end)` into the exact UTF-8 source string supplied by the caller. Consumers inspect it with:

```text
start()
end()
len()
is_empty()
```

Consumers do not construct or mutate spans. This prevents fabricated provenance from being confused with parser-produced evidence.

`ParseError` remains structured data. `kind()`, `span()`, and `detail()` are the stable inspection points. `ParseErrorKind`, `ParseErrorDetail`, and `ParseResource` are non-exhaustive: callers must retain a fallback arm when matching so a future typed diagnostic or resource category can be added without forcing an API break.

Error display text is for humans. Branch on the typed kind/detail values, not on `Display` strings.

## Lexer boundary

Lexical tokens are parser implementation data, not consumer API. `Token`, `SpannedToken`, `tokenize`, `tokenize_spanned`, and `format_tokens` remain private to the crate/tests. Consumers that need source provenance receive it through typed `ParseError` spans. This prevents the lexer grammar and token-shape choices from becoming a second compatibility contract beside the typed AST.

## AST surface

`MathNode` is the supported parsed syntax tree. Its typed component enums and records are re-exported at the crate root because callers may inspect parsed syntax and may construct values where the public variants/constructors permit it. The public AST enums are non-exhaustive. Downstream exhaustive matching is therefore intentionally rejected; consumers must preserve an unknown/future-variant path.

`FractionSpec` remains a concrete semantic record because its fields are the complete current generalized-fraction contract. `EnvRow::cells` and `FractionSpec::ordinary` remain explicit construction helpers.

`MathNode::gold` is not public API. The S-expression is repository regression evidence and may change when the internal golden schema changes. Consumer code should inspect typed variants instead of parsing or depending on that test representation.

Parser-only control-sequence classifiers such as `DelimSize::from_command` and `DelimSize::class_from_command` are private. Layout-policy conveniences such as `MatrixStyle::{is_display_env,numbers_rows,numbers_once}` and `ColSpec::is_rule` are private as well. Consumers receive the already typed values in parsed nodes and should not duplicate parser/layout classification helpers as part of their compatibility contract.

## Module boundary

The `parser` module itself is private. Supported parser items are re-exported from the crate root. This keeps source organization free to change without creating a second path-shaped compatibility contract such as `texpose::parser::...`.

Use: use crate-root parse functions and typed AST/error values only, treat options/spans as read-only values, and match public parser enums non-exhaustively.

## Font

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
