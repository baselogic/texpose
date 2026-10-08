# Parser API contract

This document owns the Phase L1 public parser boundary. It defines which parser interfaces are consumer API and which parser implementation details intentionally remain private. Syntax acceptance and malformed-input behavior remain owned by `docs/SYNTAX.md`.

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

The Phase L1 rule is therefore: use crate-root parse functions and typed AST/error values only, treat options/spans as read-only values, and match public parser enums non-exhaustively.
