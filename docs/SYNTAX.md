# TeXpose supported math syntax

This document is the contract map for the syntax supported by the portable TeXpose core. It describes a subset of TeX/LaTeX, not those languages in general. Only forms documented here are contractual; incidental parser acceptance outside this file is not a supported semantic promise and may be tightened as later semantic phases land. Documented unsupported input fails explicitly rather than being assigned guessed semantics. Cross-cutting first-stable compatibility policy, including literal-text and unit policy, is indexed by `docs/COMPATIBILITY.md`.

The parser preserves original-source byte positions in `SourceSpan`. `ParseError` exposes a typed `ParseErrorKind`, the source span associated with the failure, and construct-specific `ParseErrorDetail` where applicable. `ParseOptions` bounds nesting depth, returned AST nodes, environment rows, environment cells, and lexical tokens; budget exhaustion is `ParseErrorKind::ResourceLimit`.

Authority names below follow the project hierarchy: TeX82 owns core math-list semantics; LaTeX/amsmath own supported higher-level math constructs; OpenType MATH owns font-specific layout data rather than source syntax; behavior that is intentionally narrower than those systems is explicit TeXpose policy. Focused Rust tests are executable contracts after the governing authority has been identified.

Primary references used by this contract map:

- Donald E. Knuth, **TeX82** (`tex.web` / *TeX: The Program*) for TeX math-list, style, unit, primitive, and delimiter semantics.
- The **LaTeX Project documentation** (`https://www.latex-project.org/help/documentation/`) for LaTeX command contracts.
- The LaTeX Project **amsmath User's Guide** (`https://mirrors.ctan.org/macros/latex/required/amsmath/amsldoc.pdf`) for the supported amsmath fraction and environment forms.
- Microsoft/OpenType **MATH table specification** (`https://learn.microsoft.com/en-us/typography/opentype/spec/math`) for font-supplied math constants, glyph information, and variants. OpenType MATH explicitly supplies font data rather than a complete math-layout algorithm.


## Atoms

- **Accepted syntax:** ordinary Unicode characters and the punctuation/operator characters classified directly by the parser.
- **Semantic representation:** `MathNode::Atom(char, AtomKind)` with `Ord`, `Op`, `Bin`, `Rel`, `Open`, `Close`, `Punct`, or `Inner` where applicable.
- **Malformed-input behavior:** structural tokens that cannot begin a nucleus return `MalformedArgument`; an unexpected `}` returns `UnexpectedGroupEnd`.
- **Unsupported forms:** TeX category-code changes and arbitrary `\mathcode`/`\mathchar` programming are outside this subset.
- **Primary contract test:** `src/parser/tests/parse_golds.rs::parse_golds` and `src/layout/internal_tests/row_math_italic.rs::rows_add_math_italic_correction_without_duplicating_scripted_nuclei`.
- **Governing authority:** TeX82 math-list atom classes, narrowed by the TeXpose parser contract.

## Symbols

- **Accepted syntax:** control sequences present in `data/symbols.tsv`, the parser's explicit aliases (`\le`, `\ge`, `\ne`, `\dots`, `\lnot`, `\dag`, `\ddag`, `\owns`), and the explicit extra-symbol set in `src/parser/parse.rs`.
- **Semantic representation:** `MathNode::Symbol(String)`; the symbol catalog supplies the glyph and atom class.
- **Malformed-input behavior:** a control sequence absent from the supported catalog returns `UnknownCommand`; a catalog entry classified as a container/modifier but used as a bare symbol returns `UnsupportedCommand`.
- **Unsupported forms:** runtime symbol-table mutation and arbitrary TeX symbol definitions are not parsed.
- **Primary contract test:** `src/layout/internal_tests/symbol_golds.rs::symbol_golds` and `src/parser/tests/parse_golds.rs::catalog_commands_parse`.
- **Governing authority:** TeXpose symbol catalog and explicit aliases; TeX82 governs the resulting math atom semantics.

## Math alphabets

- **Accepted syntax:** `\mathrm`, `\mathbf`, `\mathit`, `\mathsf`, `\mathtt`, `\mathbb`, `\mathcal`, `\mathfrak`, `\mathscr`, `\boldsymbol`, `\pmb`; group-level plain-TeX switches `\rm`, `\bf`, `\cal`, `\it`, `\sf`, `\tt` are parsed structurally.
- **Semantic representation:** math alphabets use `MathNode::MathAlphabet(String, TextStyle)` for supported characters. `\pmb` keeps the argument as a `MathNode::Pmb` paint decorator. Literal text is not restyled by math alphabets.
- **`\boldsymbol` boundary:** only mathematical characters with a distinct Unicode bold variant are supported, including characters in groups and scripts. An unsupported operator, relation, delimiter, decoration, or literal-text argument returns `UnsupportedCommand` rather than silently drawing regular weight. The active font must contain the mapped glyph or layout reports `MissingGlyph`; the single-face engine does not select another font. `\pmb` remains the separate simulated-bold mechanism.
- **Malformed-input behavior:** missing arguments return `MalformedArgument`; unknown `\math...` style commands return `UnsupportedCommand` rather than being treated as symbols.
- **Unsupported forms:** text-font selectors such as `\textrm` / `\textbf`, arbitrary font-family assignment, `\fam`, NFSS declarations, and user-defined alphabet commands are outside this subset and fail as `UnsupportedCommand`.
- **Primary contract test:** `src/layout/internal_tests/fraction_semantics.rs::plain_tex_font_switches_and_mbox_are_parsed_without_preprocessing`, `src/layout/internal_tests/symbol_golds.rs::font_style_letter_classes`, `src/layout/internal_tests/row_math_italic.rs::math_alphabet_runs_materialize_character_italic_corrections`, and `src/layout/internal_tests/script_placement.rs::single_character_math_alphabet_remains_a_direct_script_nucleus`.
- **Governing authority:** TeX/LaTeX math alphabet semantics plus explicit TeXpose supported-subset policy.

## Scripts and math-style declarations

- **Accepted syntax:** postfix `_`, `^`, prime `'`, `\displaystyle`, `\textstyle`, `\scriptstyle`, and `\scriptscriptstyle`; a nucleus may carry one subscript and one superscript, with consecutive primes combined into the superscript path. Style declarations affect following noads in their current math list and remain scoped by grouping.
- **Semantic representation:** `MathNode::Subscript`, `MathNode::Superscript`, or `MathNode::SubSup` records source attachment; `MathNode::Style(MathStyleDeclaration)` retains explicit style declarations. The semantic layout pass resolves the current style and TeX subscript/superscript child styles before geometry. Large operators and integrals retain source lower/upper fields in their dedicated nodes.
- **Malformed-input behavior:** duplicate subscript or duplicate non-prime superscript returns `MalformedArgument`; nesting is subject to the configured depth and AST budgets.
- **Unsupported forms:** arbitrary TeX script-category manipulation and unsupported macro expansion are outside the parser.
- **Primary contract test:** `src/layout/internal_tests/script_placement.rs::paired_scripts_obey_open_type_vertical_constraints`, `src/layout/internal_tests/script_placement.rs::explicit_style_declaration_drives_following_script_semantics`, `src/layout/internal_tests/script_scale.rs::layout_records_script_and_scriptscript_scale`, and parser golds.
- **Governing authority:** TeX82 script and math-style rules; OpenType MATH supplies font-specific placement constants.

## Fractions

- **Accepted syntax:** `\frac`, `\dfrac`, `\tfrac`, `\cfrac`, `\cfrac[l]`, `\cfrac[r]`, `\binom`, `\dbinom`, `\tbinom`, group-level `\over`, group-level `\choose`, and `\genfrac{left}{right}{thickness}{style}{num}{den}` for styles `0..3` or inherited style when empty.
- **Semantic representation:** `MathNode::Fraction(FractionSpec)` records style override, rule policy, left/right delimiters, continued-fraction numerator alignment, numerator, and denominator.
- **Malformed-input behavior:** missing arguments, multiple infix generalized-fraction operators in one group, invalid `\cfrac` alignment, invalid `\genfrac` style, negative explicit rule thickness, or malformed dimensions return typed parser errors. `\over`/`\choose` outside a group return `MalformedArgument`.
- **Unsupported forms:** generalized-fraction styles outside `0..3` and undocumented fraction macros are rejected; there is no semantic string rewrite fallback.
- **Primary contract test:** `src/layout/internal_tests/fraction_semantics.rs`.
- **Governing authority:** TeX82 generalized-fraction semantics plus LaTeX/amsmath fraction commands; OpenType MATH supplies fraction/stack metrics.

## Radicals

- **Accepted syntax:** `\sqrt{body}` and `\sqrt[index]{body}`.
- **Semantic representation:** `MathNode::Radical(Option<Box<MathNode>>, Box<MathNode>)`.
- **Malformed-input behavior:** a missing closing `]`, missing body, or unclosed group returns a typed parser error; parser budgets apply to nested radicals and indices.
- **Unsupported forms:** arbitrary TeX radical primitives and custom radical delimiters are outside the subset.
- **Primary contract test:** `src/layout/internal_tests/radical_geometry.rs`, `src/layout/internal_tests/radical_variant.rs`, and radical records in `src/parser/tests/parse_golds.rs`.
- **Governing authority:** TeX82 radical semantics; OpenType MATH supplies radical constants and variants.

## Delimiters

- **Accepted syntax:** `\left <delim> ... \right <delim>` including `.` null delimiters; literal `()[]|/<>`; named `\{`, `\}`, `\|`, `\langle`, `\rangle`, floors, ceilings, vertical bars, arrow delimiters, backslash, groups and moustaches; explicit `\big`, `\Big`, `\bigg`, `\Bigg` and `l`/`r`/`m` variants.
- **Semantic representation:** `MathNode::Delimited(Delimiter, body, Delimiter)` or `MathNode::SizedDelim(Delimiter, DelimSize, AtomKind)`.
- **Malformed-input behavior:** unmatched fence forms return `UnmatchedDelimiter`; unknown delimiter spellings return `MalformedArgument`.
- **Unsupported forms:** delimiter commands outside the explicit parser list are rejected rather than approximated.
- **Primary contract test:** `src/layout/internal_tests/delimiter_sizing.rs` and delimiter records in `src/parser/tests/parse_golds.rs`.
- **Governing authority:** TeX82 delimiter sizing and null-delimiter semantics; OpenType MATH supplies glyph variants/assemblies.

## Accents

- **Accepted syntax:** `\hat`, `\check`, `\breve`, `\acute`, `\grave`, `\tilde`, `\bar`, `\vec`, `\dot`, `\ddot`, `\dddot`, `\ddddot`, `\widehat`, `\widetilde`, `\overline`, `\underline`, braces/arrows above or below, `\cancel`, `\bcancel`, `\xcancel`, `\cancelto`, `\mathring`, and `\not`.
- **Semantic representation:** `MathNode::Accent`, `MathNode::CancelTo`, or `MathNode::OverUnder` for constructs whose syntax carries an explicit over/under value.
- **Malformed-input behavior:** an empty accent base returns `MalformedArgument`; an unknown `\wide...` accent family returns `UnsupportedCommand`.
- **Unsupported forms:** accent commands not represented by `AccentKind` are not inferred from their names.
- **Primary contract test:** `src/layout/internal_tests/accent_golds.rs::accent_golds`, `src/layout/internal_tests/wide_accent_math.rs`, `src/layout/internal_tests/nested_accent_geometry.rs`, and `src/layout/internal_tests/color_boxes_cancel.rs` for cancel-family overlays.
- **Governing authority:** TeX/LaTeX accent semantics plus supported amsmath-style over/under forms; OpenType MATH owns top-accent attachment and variants. The supported `cancel` family follows cancel.sty default overlap/style semantics, projected to backend-neutral free lines as documented in `LAYOUT.md`.

## Operators

- **Accepted syntax:** `\sum`, `\prod`, `\lim`, the parser's named operator set (`\sin`, `\cos`, `\log`, `\det`, etc.), `\operatorname{...}`, large-operator catalog commands, `\limits`, `\nolimits`, `\overset`, `\underset`, `\stackrel`, `\xrightarrow`, and `\xleftarrow`.
- **Semantic representation:** `MathNode::Sum`, `Product`, `Limit`, `Operator`, `OverUnder`, or `StackRel`; explicit `\limits`/`\nolimits` is retained as `MathNode::Limits(..., LimitMode)`. `\overset`/`\underset` preserve only binary/relation spacing classes and otherwise become operators, while `\stackrel` is always represented as a relation. The semantic layout pass resolves default vs explicit placement and script styles before geometry.
- **Malformed-input behavior:** missing over/under/name arguments, malformed optional x-arrow arguments, or `\limits`/`\nolimits` after a non-operator nucleus return `MalformedArgument`.
- **Unsupported forms:** arbitrary operator declarations and macro-defined operators are outside the parser.
- **Primary contract test:** `src/layout/internal_tests/large_operator_limits.rs`, `src/layout/internal_tests/amsmath_substack.rs`, and operator records in `src/parser/tests/parse_golds.rs`.
- **Governing authority:** TeX82 large-operator/limits semantics and LaTeX/amsmath operator commands; OpenType MATH supplies large-operator metrics.

## Integrals

- **Accepted syntax:** `\int`, `\iint`, `\iiint`, `\oint`, `\oiint` with `_`/`^` scripts and `\limits`/`\nolimits` handling as supported by the layout engine.
- **Semantic representation:** `MathNode::Integral(IntegralKind, lower, upper)` plus an optional `MathNode::Limits(..., LimitMode)` wrapper for an explicit placement override; the semantic layout pass resolves effective placement and script styles.
- **Malformed-input behavior:** malformed scripts return `MalformedArgument`; resource budgets apply to nested script expressions.
- **Unsupported forms:** integral families outside `IntegralKind` are not synthesized from command names.
- **Primary contract test:** `src/layout/internal_tests/integral_scripts.rs`.
- **Governing authority:** TeX82 operator/script semantics plus supported LaTeX/amsmath integral vocabulary; OpenType MATH supplies glyph and italic-correction data.

## Matrices

- **Accepted syntax:** `matrix`, `pmatrix`, `bmatrix`, `vmatrix`, `Vmatrix`, `Bmatrix`, and `array`; rows use `\\` or `\cr`, cells use `&`. `array` preambles accept `l`, `c`, `r`, `|`, and whitespace.
- **Semantic representation:** `MathNode::Matrix(MatrixStyle, Vec<ColSpec>, Vec<EnvRow>)`.
- **Malformed-input behavior:** malformed bodies/preambles return `MalformedMatrix`; begin/end name disagreement returns `MismatchedEnvironment`; known but unsupported environments/preamble constructs return `UnsupportedCommand`; row/cell budgets return `ResourceLimit`.
- **Unsupported forms:** `array` preamble forms `@`, `!`, `>`, `<`, `p`, `m`, `b`, `*` and other environment names are not supported.
- **Primary contract test:** `src/layout/internal_tests/amsmath_grid.rs::matrix_uses_textstyle_physical_array_spacing_and_axis_center` and environment golds in `src/layout/internal_tests/env_golds.rs`.
- **Governing authority:** LaTeX/amsmath matrix/array contracts plus explicit TeXpose preamble subset; TeX82 governs math cells.

## Cases

- **Accepted syntax:** `\begin{cases} ... \end{cases}` with `&`-separated fields and normal environment row separators.
- **Semantic representation:** `MathNode::Matrix(MatrixStyle::Cases, ..., rows)`.
- **Malformed-input behavior:** the same typed environment, row, cell, and resource-limit failures as matrices.
- **Unsupported forms:** package-specific cases variants not listed as parser environments are rejected.
- **Primary contract test:** `src/layout/internal_tests/amsmath_grid.rs::cases_use_arraystretch_quad_gap_and_axis_center`.
- **Governing authority:** amsmath `cases`, with TeX82 math semantics inside cells.

## Aligned and display environments

- **Accepted syntax:** `aligned`, `align`, `gather`, `multline`, `equation`, and `split`; `\intertext{...}` and `\hline` are recognized row forms where the engine supports them.
- **Semantic representation:** the corresponding `MatrixStyle` plus `Vec<EnvRow>`; equation-number metadata is carried in `EqNumber` and row labels.
- **Malformed-input behavior:** malformed environment structure is `MalformedMatrix`; mismatched closing names are `MismatchedEnvironment`; resource budgets apply globally across environments.
- **Nesting:** `equation`, `gather`, and `multline` are outer displays; `align` is outer or directly inside `gather`; `split` requires a direct `equation`, `align`, or `gather` parent. `aligned` and matrix/array environments may appear in math cells. Invalid parent/child combinations fail as `MalformedMatrix`.
- **Unsupported forms:** starred environment spellings and other amsmath environments are outside the explicit parser list unless separately documented.
- **Primary contract test:** `src/layout/internal_tests/amsmath_grid.rs`, `src/layout/internal_tests/env_golds.rs::env_golds`, and `src/layout/internal_tests/amsmath_substack.rs`.
- **Governing authority:** LaTeX/amsmath environment contracts; TeX82 governs math content inside cells.

## Spacing

- **Accepted syntax:** `\,`, `\:`, `\>`, `\;`, `\!`, `\quad`, `\qquad`, control-space `\ `, and `\hspace{length}`. Parsed lengths accept `em`, `mu`, `pt`, and `bp`; omitted unit in this parser path means `em` for compatibility.
- **Semantic representation:** `MathNode::Space(SpaceKind)` with `Length::{Em, Mu, TexPt, BigPt}` retained until layout. Math-space commands are style-scaled through `mu`; control-space `\ ` is the supported one-third root-em interword-space approximation and is not math-style-scaled.
- **Malformed-input behavior:** invalid numeric text or unsupported units return `MalformedDimension` at the dimension's source span.
- **Unsupported forms:** other TeX/LaTeX glue syntax, stretch/shrink components, and additional physical units are outside the current contract.
- **Primary contract test:** `src/layout/internal_tests/length_units.rs`, `src/layout/internal_tests/semantic_spacing.rs`, and spacing golds.
- **Governing authority:** TeX82 math units/spacing and LaTeX physical-unit conventions, narrowed to the four documented units.

## Colors

- **Accepted syntax:** `\color`, `\textcolor`, `\colorbox`, `\fcolorbox`, and `\definecolor`; color specifications accept `named`, `rgb`, `RGB`, `HTML`, `cmyk`, and `gray`, with built-in names plus earlier `\definecolor` declarations.
- **Semantic representation:** `MathNode::Color`, `TextColor`, `ColorBox`, or `FColorBox` carrying a typed `Color`; definitions update the parse-local `ColorTable`.
- **Malformed-input behavior:** malformed color components return `MalformedArgument`; unsupported models or unknown named colors return `UnsupportedCommand` with the relevant specification span when available.
- **Unsupported forms:** forward references to later color definitions and color models outside `parse_color_spec` are rejected.
- **Primary contract test:** color records in `src/parser/tests/parse_golds.rs`, `src/layout/internal_tests/layout_golds.rs::layout_golds`, and `src/layout/internal_tests/color_boxes_cancel.rs`.
- **Governing authority:** explicit TeXpose math-color subset using LaTeX command vocabulary. Foreground wrappers preserve geometry/class; color boxes use the standard physical `\fboxsep`/`\fboxrule` defaults while keeping their body in TeXpose math mode.

## Numbering

- **Accepted syntax:** `\tag{...}`, `\tag*{...}`, `\nonumber`, and `\notag` on rows owned by the supported numbered display environments; `align`/`gather` number rows by default, while `equation`/`multline` own one environment number according to `MatrixStyle` policy.
- **Semantic representation:** `MathNode::Tag`/`NoNumber` are peeled into `EnvRow::Cells { number: EqNumber, ... }`. `NumberingState` retains only durable counter/label state; each layout call prepares an ephemeral assignment plan and commits that plan only after layout succeeds.
- **Counter contract:** `\tag`/`\tag*` and `\nonumber`/`\notag` do not consume the automatic counter. Automatic values use the configured `start`; counter exhaustion returns `Error::InvalidOption` instead of saturating and repeating the terminal value.
- **Failure contract:** a failed layout does not consume equation numbers or publish labels. Missing tag arguments and malformed environment structure return typed failures rather than inventing a number.
- **Unsupported forms:** arbitrary counter manipulation, unsupported starred environments, and use of numbering controls outside the documented numbered-display subset are not contractual. `NumberFormat` changes the displayed equation number but not the payload returned by `\ref`.
- **Primary contract test:** `src/layout/internal_tests/numbering_labels.rs`, plus numbering records in `src/layout/internal_tests/env_golds.rs`.
- **Governing authority:** LaTeX/amsmath numbering forms plus explicit TeXpose in-memory and transactional-state policy.

## Labels

- **Accepted syntax:** `\label{key}` on a supported numbered environment row.
- **Semantic representation:** `MathNode::Label(String)` is peeled into row metadata. A successful numbering pass stores both the formatted display number and the unwrapped reference payload; `NumberingState::label` exposes the formatted display form for inspection.
- **Failure contract:** labels are published only when their number/tag exists and the complete layout succeeds. A label on a suppressed number remains unbound.
- **Malformed-input behavior:** an unclosed/missing key group returns the corresponding typed group/argument error.
- **Unsupported forms:** auxiliary-file persistence, general cross-document label machinery, and labels outside the documented numbered-display subset are outside the portable core.
- **Primary contract test:** `src/layout/internal_tests/numbering_labels.rs` and label records in `src/layout/internal_tests/env_golds.rs`.
- **Governing authority:** LaTeX label vocabulary plus TeXpose in-memory numbering policy.

## References

- **Accepted syntax:** `\ref{key}`.
- **Semantic representation:** `MathNode::Ref(String)` resolves through `NumberingState` during layout. The rendered payload is the unwrapped reference value (`1`, `iv`, `A`, ...); display wrappers such as `(1)` or `[iv]` belong to the equation number itself, not to `\ref`.
- **Forward-reference contract:** references to labels later in the same parsed layout tree resolve because numbering preparation precedes geometry. A reference in an earlier independent layout call cannot see a label introduced only by a later call.
- **Malformed-input behavior:** malformed key groups return typed group/argument errors; an unresolved reference fails explicitly during layout.
- **Unsupported forms:** `\eqref`, `\pageref`, hyperlink/package extensions, external auxiliary-file lookup, and cross-call forward-reference precollection are outside the core.
- **Primary contract test:** `src/layout/internal_tests/numbering_labels.rs` and reference/numbering records in `src/layout/internal_tests/env_golds.rs`.
- **Governing authority:** LaTeX reference vocabulary plus TeXpose in-memory numbering policy.

## Phantoms

- **Accepted syntax:** `\phantom{...}`, `\vphantom{...}`, and `\hphantom{...}`.
- **Semantic representation:** `MathNode::Phantom(PhantomKind::{Full, Vertical, Horizontal}, body)`.
- **Malformed-input behavior:** missing/unclosed body arguments return typed parser errors.
- **Unsupported forms:** package-specific phantom variants are not inferred.
- **Primary contract test:** `src/layout/internal_tests/phantom_overunder.rs` plus phantom records in `src/parser/tests/parse_golds.rs`, `src/layout/internal_tests/layout_golds.rs`, and the math comparison corpus.
- **Governing authority:** TeX/LaTeX phantom semantics as narrowed by `PhantomKind`.

## Boxes and rules

- **Accepted syntax:** `\boxed{...}`, `\fbox{...}`, `\colorbox{color}{body}`, `\fcolorbox{border}{fill}{body}`, `\rule{width}{height}`, and `\strut`.
- **Semantic representation:** boxed forms use `MathNode::Accent(..., AccentKind::Boxed)` or color-box nodes; `\rule` is `MathNode::Rule(Length, Length)`; `\strut` is `MathNode::Strut(Length, Length)`.
- **Malformed-input behavior:** missing bodies/colors or malformed rule dimensions return typed parser errors; length units are never erased during parsing.
- **Unsupported forms:** optional LaTeX `\rule` raise arguments and other box-model commands are outside the current subset.
- **Primary contract test:** `src/layout/internal_tests/length_units.rs::parsed_rule_preserves_both_length_units`, box/color golds, accent golds, and `src/layout/internal_tests/color_boxes_cancel.rs`.
- **Governing authority:** amsmath `\boxed` plus LaTeX `\fboxsep=3pt` / `\fboxrule=0.4pt`, narrowed to TeXpose's math-mode box subset; TeX units follow the documented length contract.

## Literal text runs

- **Accepted syntax:** `\text{...}` and `\mbox{...}`; `\intertext{...}` is recognized inside supported environments.
- **Semantic representation:** literal text is `MathNode::LiteralText(String)`; `EnvRow::Intertext` contains the same literal-text node. Math alphabets and literal text are distinct AST constructs.
- **Layout contract:** one selected face; a Unicode scalar sequence; one direct cmap/glyph lookup per scalar; U+0020 uses that face's U+0020 advance; glyphs are placed left-to-right without shaping, kerning, bidi reordering, ligatures, or text-font fallback. Math-style scaling still applies to the completed literal run.
- **Literal escapes:** the parser accepts direct scalars plus the unambiguous control-symbol escapes `\%`, `\$`, `\#`, `\&`, `\_`, `\{`, `\}`, and control-space. Other control sequences are not reinterpreted as literal bytes.
- **Malformed-input behavior:** missing or unclosed text groups return typed parser errors with original-source byte spans. Multi-letter control sequences, nested text groups, and other text-mode syntax requiring semantics outside the literal-run contract return `UnsupportedCommand`.
- **Unsupported forms:** text font selection (`\textrm`, `\textbf`, ...), full TeX paragraph/text-mode parsing, nested text macros/groups, shaping, bidi, font fallback, and general macro expansion are outside this math parser. Missing glyphs fail through the typed font error instead of fabricating replacement output.
- **Primary contract test:** `src/layout/internal_tests/literal_text.rs`, text records in parser golds, and environment golds for `\intertext`.
- **Governing authority:** LaTeX/amsmath text-in-math constructs narrowed to TeXpose's explicit literal glyph-run contract.

## Diagnostic and resource contract

`SourceSpan` offsets are byte offsets into the exact string passed to `parse` or `parse_with_options`; they are half-open `[start, end)` ranges and therefore remain correct for UTF-8 source. TeXpose has no semantic preprocessing stage: the exact caller source is tokenized directly, so diagnostics never need to reverse-map rewritten text. Spans are parser-produced read-only values inspected through `start()`, `end()`, `len()`, and `is_empty()`.

The required parser failure categories are `TrailingBackslash`, `UnknownCommand`, `UnsupportedCommand`, `UnclosedGroup`, `UnexpectedGroupEnd`, `UnmatchedDelimiter`, `MismatchedEnvironment`, `MalformedArgument`, `MalformedDimension`, `MalformedMatrix`, and `ResourceLimit`. `tests/parser_diagnostics.rs` owns their typed reachability and span contracts.

`ParseOptions` controls five independent budgets through `with_*` builders and read-only getters. The defaults are 32 nesting levels, 65,536 returned AST nodes, 4,096 total environment rows, 16,384 total environment cells, and 131,072 lexical tokens. Token, nesting, row, and cell limits are checked before accepting the next unit of that resource. The AST-node limit is an exact checked count of the completed returned tree; pre-tree parsing work remains independently bounded by the token, nesting, row, and cell budgets. Counter arithmetic is checked throughout. A would-exceed condition returns `ResourceLimit` with the resource and configured limit in `ParseErrorDetail`. `src/layout/internal_tests/depth_limit.rs::parser_resource_budgets_cover_hostile_shapes` owns hostile-input coverage.

These five parser budgets do not impose a source-byte or individual control-word-length limit. Callers that accept untrusted inputs and require a hard byte-volume ceiling must enforce that separate boundary before calling the parser.
