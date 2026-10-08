# Mathematical-core compatibility policy

This document owns the Phase L4 cross-cutting compatibility policy for the first
stable mathematical core. It does not replace the domain contracts that define
syntax, parser API, font construction, exact layout, OpenType MATH coverage, or
differential-oracle evidence. Instead it states which of those behaviors are
part of the supported compatibility boundary and which adjacent capabilities are
intentionally outside it.

The detailed authorities remain:

- `docs/SYNTAX.md` for accepted source syntax and literal-text grammar;
- `docs/PARSER_API.md` for the public parser boundary;
- `docs/FONT_API.md` for font ownership, face identity, and construction errors;
- `docs/LAYOUT.md` for exact-to-public geometry and display-list semantics;
- `docs/MATH_COVERAGE.md` for OpenType MATH field ownership and degradation;
- `docs/ORACLE_PROFILES.md` for the pinned differential-reference profiles;
- `docs/CI.md` and `docs/RELEASE.md` for verification and release evidence.

If this summary and an owning domain contract disagree, the owning domain
contract and executable behavior must be reconciled before release. This file is
not authority to silently weaken a stronger TeX, LaTeX, amsmath, or OpenType
contract already adopted by the project.

## Stable compatibility matrix

| Area | First stable mathematical-core policy |
| --- | --- |
| Formula font | One caller-supplied static OpenType math face per layout operation/formula. |
| Font identity | Exact immutable bytes plus explicit face index; emitted glyph IDs always belong to that retained face. |
| Collections | TTC/OTC are supported only with an explicit face index. |
| Variable fonts | A selected face with functional `fvar` axes is rejected with `FontError::VariableFontUnsupported`. |
| Text inside math | `\text`, `\mbox`, and supported `\intertext` are literal scalar runs on the same selected math face; no shaping/fallback text engine is implied. |
| Relative units | `em` and `mu` are resolved against the current mathematical style. |
| Physical units | TeX `pt` and PostScript `bp` are preserved through parsing and resolved against a validated root-em physical size. |
| Default physical root em | Entry points without an explicit physical size use a 10 TeX pt root em. |
| Device corrections | OpenType MATH Device/VariationIndex corrections are ignored; design-unit values are authoritative for this core. |
| Missing Unicode glyph | Recoverable deterministic layout with `LayoutDiagnostic::MissingGlyph`; no cross-font fallback. |
| Unusable extensible assembly | Retain the largest valid ready-made variant and emit `LayoutDiagnostic::ExtensibleFallback`. |
| Unsupported/malformed input | Typed failure; unsupported semantics are not guessed from adjacent TeX/LaTeX behavior. |
| Differential oracle | Canonical profiles are `stix`, `libertinus`, and `fira` on the pinned MiKTeX 26.5 environment. |
| Stress oracle | Same three profiles, with exact stale-sensitive documented deviations; stress evidence is release-candidate/nightly evidence, not a looser semantic mode. |
| Rendering | Outside TeXpose. Consumers render the backend-neutral `MathLayout` using the exact retained font identity. |

## One static math face per formula

Every public layout entry point accepts exactly one `MathFont`. `MathFont`
retains one validated face identity: immutable shared bytes plus a `u32` face
index. A successful `MathLayout` owns a cheap clone of that same handle and all
`MathOp::Glyph` IDs are resolved against it.

There is no implicit font-set, text-font, symbol-font, or missing-glyph fallback
chain. A formula cannot silently switch to another face because a scalar is
missing or because a literal-text run would shape better elsewhere. A consumer
that needs multi-font composition must perform that composition outside this
mathematical-core contract and must not reinterpret glyph IDs from one
`MathLayout` against another face.

Static TTC/OTC collections are supported by explicit index. `MathFont::from_bytes`
rejects collection input with `CollectionFaceIndexRequired`; callers use
`from_bytes_at_index` or `from_shared_bytes` to select the intended face.

This policy keeps glyph identity, MATH tables, GSUB selection, metrics, and
outline lookup under one auditable face identity for a complete operation.

## Literal text is deliberately not a general text engine

The supported literal-text constructs are `\text{...}`, `\mbox{...}`, and the
supported `\intertext{...}` environment row form. Their exact grammar and
unsupported control-sequence behavior are owned by `docs/SYNTAX.md`.

For compatibility purposes a literal-text run means:

- the same selected `MathFont` used by the formula;
- direct Unicode-scalar-to-cmap lookup in source order;
- U+0020 measured from that same face;
- left-to-right placement of the resulting glyph advances;
- mathematical style scaling of the completed run.

It does **not** promise script shaping, kerning, ligatures, bidi reordering,
language/script itemization, Unicode normalization, OpenType text-feature
selection, NFSS/text-family switching, or fallback to another font. Those are
renderer/text-system responsibilities outside the first stable mathematical
core.

Math alphabets are not literal text. `\mathrm`, `\mathbf`, and the other
supported math alphabets remain mathematical semantic constructs and continue to
use the MATH/GSUB behavior documented in the layout and coverage contracts.

## Physical and style-relative units

The parser preserves four supported length domains instead of collapsing them
into a single number:

- `em`: current mathematical-style em;
- `mu`: current mathematical-style math unit;
- TeX `pt`: physical TeX point;
- `bp`: physical PostScript big point.

`em` and `mu` are style-relative and therefore can change when the same parsed
length is resolved in text, script, or scriptscript style. `pt` and `bp` are
physical. Layout resolves them only after a positive root-em physical size is
known.

`layout_with_em_size_pt` and
`layout_with_numbering_and_em_size_pt` accept that root-em size in exact TeX
points and reject zero or negative values. Entry points without an explicit
size use the project default of 10 TeX pt. One `bp` is converted exactly as
`7227/7200` TeX pt before division by the root-em size.

The public display list remains normalized to root-em coordinates regardless of
which physical size produced it. Consumers choose the eventual device-space em
size when rendering; they do not reinterpret a TeXpose `pt` quantity after
layout has already resolved it.

## OpenType Device and VariationIndex corrections

OpenType MATH `MathValueRecord` can carry a design-unit value plus an optional
Device table or VariationIndex correction. The first stable core consumes the
design-unit value and deliberately ignores that optional correction.

Consequences of this policy:

- mathematical geometry is independent of PPEM, display DPI, pixel grid, and
  rasterizer state;
- layout remains backend-neutral and exact until the final `f32` display-list
  conversion;
- a consumer must not apply MATH Device corrections a second time to TeXpose
  positions;
- changing this policy is a compatibility change requiring new external-oracle
  evidence and focused tests.

This policy is distinct from variable-font support. Variable faces are rejected
at construction, while Device corrections can also exist in otherwise static
faces and are still ignored.

## Variable-font policy

The selected OpenType face must be static. A physical `fvar` table with
`axisCount > 0` is a functional variable face and construction fails with
`FontError::VariableFontUnsupported`. An `fvar` table whose axis count is zero
does not make the face variable for this contract.

No public API accepts design-space coordinates, named instances, or variation
settings. No implied default-instance compatibility is promised for a variable
font. Supporting variable math fonts in the future therefore requires an
explicit API and oracle-policy revision rather than silently accepting current
font bytes under default coordinates.

## Graceful degradation is narrow and observable

Recoverable degradation is permitted only where the layout contract names it,
and it is observable through `MathLayout::diagnostics()`.

For an ordinary or literal-text Unicode scalar missing from cmap, TeXpose emits
`LayoutDiagnostic::MissingGlyph { ch }`. It first uses glyph ID 0 only when that
glyph has a usable non-zero horizontal advance. Otherwise it emits a non-drawing
placeholder whose width and height are one current math em and whose depth and
italic correction are zero. This produces deterministic geometry without
inventing an outline or changing fonts.

For a requested extensible glyph, malformed or unusable assembly data does not
create an unchecked synthetic construction. TeXpose retains the largest valid
ready-made variant and emits `LayoutDiagnostic::ExtensibleFallback { ch }`.
Focused accent candidate lists and the documented `\not` fallback remain
construct-owned behavior rather than a general fallback mechanism.

The following are **not** graceful-degradation cases:

- invalid or unsupported source syntax;
- invalid parser/layout options;
- invalid font bytes or face index;
- a missing or malformed mandatory OpenType MATH table/MathConstants contract;
- exact arithmetic failure;
- unsupported variable-font input.

Those conditions return typed errors. A successful display list therefore never
means that arbitrary unsupported input was silently approximated.

## Oracle compatibility profiles

The contractual differential profiles are `stix`, `libertinus`, and `fira`.
Each profile pins font bytes/hash, face index, corpus identity, tolerance policy,
documented deviations, and the reference-environment identity defined in
`docs/ORACLE_PROFILES.md`.

The pinned reference environment is MiKTeX 26.5 / LuaHBTeX 1.25.7 with
environment SHA-256:

```text
b621bc874d9749432eca9f8a66a8bc8ffd72afda0ef8de8624f6a2c2171acbdf
```

The environment hash is not a substitute for font/corpus identity; those are
validated separately. Updating LuaHBTeX, MiKTeX, LaTeX, `unicode-math`,
`fontspec`, or `amsmath` is a reviewed oracle-baseline change.

`DejaVu Math TeX Gyre` participates in the committed multi-font smoke matrix but
is not a named canonical/stress differential profile. Smoke coverage and
external differential evidence have different owners and must not be conflated.

Canonical and stress deviations are stale-sensitive. A documented ceiling or
glyph signature must fail when exceeded and must also be removed when the
underlying difference disappears. The oracle is independent evidence, not
permission to override a stronger applicable TeX/OpenType contract.

## Compatibility changes

A change to any policy in this document is structural/public-contract work. At a
minimum it requires the affected focused contract tests, the core CI gate, and
any applicable canonical external oracle. A change to oracle profiles,
font-selection policy, MATH semantics, physical-size semantics, variable-font
policy, or graceful-degradation behavior also requires review of the release
evidence described in `docs/RELEASE.md`.

Pure wording corrections that do not alter the contract remain documentation
maintenance, but they must not be used to redefine behavior after verification.
