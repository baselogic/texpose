# Verification font profiles

This document owns the committed verification-font census used by the multi-font smoke matrix. It records source-font capabilities, not TeXpose feature support. Engine support and degradation policy remain owned by [`MATH_COVERAGE.md`](MATH_COVERAGE.md).

## E12 smoke contract

`src/layout/internal_tests/multi_font_smoke.rs` runs one common corpus through every committed verification profile. No profile or corpus row is silently skipped. For each profile the test requires:

- successful `MathFont` construction at the pinned face index;
- successful parsing and layout for every corpus row;
- no recoverable missing-glyph diagnostics in the common corpus;
- valid exact `Dim` values throughout the returned box tree;
- identical glyph-ID sequences across two independently constructed fonts/layouts;
- exact deterministic internal layout geometry across those independent runs.

Semantic normalization remains private to the layout layer. The smoke test exercises it through the public boundary: `a+b` and `a\textstyle+b` in text style must produce identical output, proving that a non-spacing same-style control does not perturb binary-operator normalization or spacing. Exact semantic class rules remain owned by the focused tests in `src/layout/semantic.rs`.

## Profile identity

All current verification faces are standalone fonts at face index 0. Fixture replacement requires updating the corresponding `SOURCE.md` provenance/hash and rerunning the smoke matrix before the profile census can change.

| Profile | Fixture | SHA-256 | Face index |
| --- | --- | --- | ---: |
| `stix` | `tests/fixtures/fonts/stix-two-math/STIXTwoMath-Regular.otf` | `f2076b9f1676438439dd41e23676f5ab99056e83d6b8f8c27841591ef2ccfa72` | 0 |
| `libertinus` | `tests/fixtures/fonts/libertinus-math/LibertinusMath-Regular.otf` | `e81bd44acbb7119c8f00128b36fecc5d980e10d2450a226ba52402ccf4da9d32` | 0 |
| `fira` | `tests/fixtures/fonts/fira-math/FiraMath-Regular.otf` | `2028cbd3dd4d8c0cf1608520eb4759956a83a67931d7b6d8e7c313520186e35b` | 0 |
| `dejavu` | `tests/fixtures/fonts/dejavu-math/DejaVuMathTeXGyre.ttf` | `f7a5e6bcc7747e7488634c8b94684a71596b1b3bc1d39f3c4600fc7677458f9e` | 0 |

The provenance hashes are enforced by the existing font-fixture/static-font contract tests; E12 does not create a second hash authority.

## Common corpus

| Family | Input | Root style |
| --- | --- | --- |
| ordinary symbols | `x+y=\alpha` | text |
| math alphabets | `\mathrm{x}+\mathbf{x}+\mathit{x}` | text |
| scripts | `x_i^2+y_{j_k}` | text |
| fractions | `\frac{a+b}{c+d}` | display |
| radicals | `\sqrt[3]{x^2+y^2}` | display |
| delimiters | `\left(\frac{a+b}{c+d}\right)` | display |
| operators | `\sum_{i=1}^{n}i^2` | display |
| integrals | `\int_0^1 x^2\,dx` | display |
| accents | `\widehat{xyz}` | text |
| matrices | `\begin{pmatrix}a&b\\c&d\end{pmatrix}` | display |
| aligned | `\begin{aligned}a&=b+c\\d&=e-f\end{aligned}` | display |
| spacing | `a+b\,c\!d\quad e` | text |

The corpus is intentionally a smoke matrix rather than an external geometry oracle. Primitive geometry remains protected by focused tests and, where required by the roadmap, the LuaLaTeX differential oracle.

## Source-font capability census

The table below is a physical-font census for capabilities relevant to the current math engine. `Present` means the committed fixture physically exposes the table/feature. `Missing` means the fixture does not expose it through the indicated OpenType path; it does not mean TeXpose should synthesize a substitute.

GSUB entries refer specifically to features reachable from `ScriptList["math"]` / `DefaultLangSys`, matching TeXpose's E8 contract. Math construction/assembly counts come from the committed `MATH` table. The profile hash above pins these observations to exact bytes.

| Profile | MathKernInfo glyphs | ExtendedShapeCoverage glyphs | active `ssty` | active `flac` | active `dtls` | vertical constructions / assemblies | horizontal constructions / assemblies |
| --- | ---: | ---: | --- | --- | --- | ---: | ---: |
| `stix` | 233 | 494 | Present | Present | Present | 118 / 32 | 47 / 37 |
| `libertinus` | 0 | 268 | Present | **Missing** | **Missing** | 59 / 15 | 27 / 22 |
| `fira` | 0 | 388 | Present | **Missing** | Present | 40 / 18 | 6 / 6 |
| `dejavu` | 0 | 301 | Present | **Missing** | Present | 95 / 47 | 86 / 71 |

For G7, STIX provides positive `MathKernInfo` evidence while Libertinus, Fira, and DejaVu provide the zero-kern degradation path. All four profiles expose non-empty `ExtendedShapeCoverage`; the focused vertical-placement contract uses STIX to distinguish a covered `|` glyph from an uncovered tall ordinary glyph.

For the G5 circumflex/tilde contract, the pinned `stix`, `libertinus`, and `dejavu` fixtures expose horizontal MATH constructions for combining U+0302/U+0303, while their spacing U+02C6/U+02DC candidates do not. The pinned `fira` fixture exposes neither U+0302 nor U+0303 as a horizontal construction; its six horizontal constructions are U+23B4, U+23B5, and U+23DC–U+23DF. G5 therefore uses Fira to prove deterministic base-glyph fallback rather than synthetic stretching.

For G6, the pinned fixtures intentionally exercise different script-style contracts. `stix` uses 70% / 55% script scales and two-level Alternate Substitution forms; `dejavu` uses 80% / 65% and two-level alternates; `fira` uses 72% / 58% and two-level alternates for prime glyphs; `libertinus` uses 80% / 60% and Single Substitution for its prime script forms, so the same alternate serves both script levels. All four fixtures leave U+002A ASTERISK outside active `ssty` coverage; the focused G6 test uses it to prove original-glyph fallback while script scaling remains active.

Profile-specific absence is data, not a test exemption. The common E12 corpus must still construct, parse, normalize, and lay out deterministically for every profile. When a future corpus row genuinely depends on a capability that a profile lacks, the exclusion/degradation must be named in this census and asserted explicitly rather than implemented as a silent skip.
