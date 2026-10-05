//! LaTeX math string → [`MathNode`].
//!
//! Binding of `_` / `^` / primes is Pratt-style (tight postfix on the nucleus).
//! The math list itself is a TeX-style row of atoms, not an arithmetic tree.

use super::ast::{
    AccentKind, AtomKind, ColSpec, DelimSize, Delimiter, EnvRow, EqNumber, FractionAlignment,
    FractionRule, FractionSpec, FractionStyle, IntegralKind, Length, LimitMode, MathNode,
    MathStyleDeclaration, MatrixStyle, PhantomKind, SpaceKind, TextStyle,
};
use super::preproc::preprocess;
use super::token::{tokenize_spanned_with_limit, SpannedToken, Token};
use crate::color::{parse_color_spec, Color, ColorTable};
use crate::dim::Dim;
use crate::error::{Error, ParseError, ParseErrorKind, ParseResource, SourceSpan};
use crate::symbols::{lookup, SymbolKind as CatalogKind};

/// Default deepest nesting [`parse`] and [`layout`](crate::layout()) accept.
///
/// Both recurse over the structure of the input, so their stack use grows with
/// how deeply it nests. Past the limit they return an error rather than
/// consume unbounded stack: input this deep is pathological, and rejecting it
/// is preferable to risking process-stack exhaustion.
///
/// The count is of parser recursion levels rather than of LaTeX constructs, and
/// a braced argument costs two of them (one for the argument and one for the
/// group), so the default admits `\frac{1}{…}` nested about 15 deep. Real
/// mathematics rarely nests beyond five levels.
///
/// A caller that deliberately accepts deeper input can raise the limit with
/// [`ParseOptions::with_max_depth`] and
/// [`layout_with_max_depth`](crate::layout_with_max_depth).
pub const DEFAULT_MAX_NESTING_DEPTH: usize = 32;
/// Default maximum number of AST nodes returned by one parse.
pub const DEFAULT_MAX_AST_NODES: usize = 65_536;
/// Default maximum total number of rows across parsed environments.
pub const DEFAULT_MAX_ENVIRONMENT_ROWS: usize = 4_096;
/// Default maximum total number of cells across parsed environments.
pub const DEFAULT_MAX_ENVIRONMENT_CELLS: usize = 16_384;
/// Default maximum number of lexical tokens produced from one input.
pub const DEFAULT_MAX_TOKENS: usize = 131_072;

/// Options for [`parse_with_options`].
///
/// # Examples
///
/// ```
/// use texpose::{parse_with_options, ParseOptions};
///
/// let deep = "{".repeat(40) + "x" + &"}".repeat(40);
/// assert!(parse_with_options(&deep, &ParseOptions::new()).is_err());
/// let roomy = ParseOptions::new().with_max_depth(128);
/// assert!(parse_with_options(&deep, &roomy).is_ok());
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub struct ParseOptions {
    /// Deepest parser recursion accepted. Defaults to [`DEFAULT_MAX_NESTING_DEPTH`].
    pub max_depth: usize,
    /// Maximum nodes in the returned AST. Defaults to [`DEFAULT_MAX_AST_NODES`].
    pub max_ast_nodes: usize,
    /// Maximum total environment rows. Defaults to [`DEFAULT_MAX_ENVIRONMENT_ROWS`].
    pub max_environment_rows: usize,
    /// Maximum total environment cells. Defaults to [`DEFAULT_MAX_ENVIRONMENT_CELLS`].
    pub max_environment_cells: usize,
    /// Maximum lexical tokens produced from the source. Defaults to [`DEFAULT_MAX_TOKENS`].
    pub max_tokens: usize,
}

impl Default for ParseOptions {
    fn default() -> Self {
        Self {
            max_depth: DEFAULT_MAX_NESTING_DEPTH,
            max_ast_nodes: DEFAULT_MAX_AST_NODES,
            max_environment_rows: DEFAULT_MAX_ENVIRONMENT_ROWS,
            max_environment_cells: DEFAULT_MAX_ENVIRONMENT_CELLS,
            max_tokens: DEFAULT_MAX_TOKENS,
        }
    }
}

impl ParseOptions {
    /// Options with every field at its default.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the deepest parser recursion accepted.
    ///
    /// Raising it past the default is safe only if the calling thread has the
    /// stack for it; see [`DEFAULT_MAX_NESTING_DEPTH`].
    #[must_use]
    pub fn with_max_depth(mut self, max_depth: usize) -> Self {
        self.max_depth = max_depth;
        self
    }

    /// Set the maximum number of nodes in the returned AST.
    #[must_use]
    pub fn with_max_ast_nodes(mut self, max_ast_nodes: usize) -> Self {
        self.max_ast_nodes = max_ast_nodes;
        self
    }

    /// Set the maximum total number of rows across parsed environments.
    #[must_use]
    pub fn with_max_environment_rows(mut self, max_environment_rows: usize) -> Self {
        self.max_environment_rows = max_environment_rows;
        self
    }

    /// Set the maximum total number of cells across parsed environments.
    #[must_use]
    pub fn with_max_environment_cells(mut self, max_environment_cells: usize) -> Self {
        self.max_environment_cells = max_environment_cells;
        self
    }

    /// Set the maximum number of lexical tokens produced from the source.
    #[must_use]
    pub fn with_max_tokens(mut self, max_tokens: usize) -> Self {
        self.max_tokens = max_tokens;
        self
    }
}

/// Parse a LaTeX math string into a [`MathNode`] using a fresh color table.
///
/// Accepts raw math, `$...$`, `$$...$$`, `\(...\)`, or `\[...\]`. Binding of
/// `_` / `^` / primes is Pratt-style (tight postfix on the nucleus).
///
/// # Arguments
///
/// * `input` — math source, with or without delimiter fences.
///
/// # Returns
///
/// The typed AST, or a [`ParseError`] naming the problem.
///
/// # Errors
///
/// Errors expose a [`ParseErrorKind`](crate::ParseErrorKind) and an original-source
/// [`SourceSpan`](crate::SourceSpan). Unknown/unsupported commands, malformed
/// arguments, unmatched delimiters, resource limits, and other structural failures
/// remain distinguishable without parsing diagnostic text.
///
/// # Examples
///
/// ```
/// use texpose::parse;
///
/// let ast = parse(r"\frac{1}{2}").unwrap();
/// assert_eq!(ast.gold(), r#"(frac (atom Ord "1") (atom Ord "2"))"#);
/// ```
pub fn parse(input: &str) -> Result<MathNode, ParseError> {
    parse_with_colors(input).map(|(n, _)| n)
}

/// Parse a math string, returning the AST and the color table after `\definecolor`.
///
/// # Arguments
///
/// * `input` — math source, with or without delimiter fences.
///
/// # Returns
///
/// The AST and the color table including any `\definecolor` names from `input`.
///
/// # Errors
///
/// Same as [`parse`].
///
/// # Examples
///
/// ```
/// use texpose::parse_with_colors;
///
/// let (ast, table) = parse_with_colors(r"\definecolor{ok}{named}{red}x").unwrap();
/// assert!(table.get("ok").is_ok());
/// let _ = ast;
/// ```
pub fn parse_with_colors(input: &str) -> Result<(MathNode, ColorTable), ParseError> {
    parse_with_options(input, &ParseOptions::default())
}

/// Parse a math string with explicit [`ParseOptions`], returning the AST and
/// the color table after `\definecolor`.
///
/// # Errors
///
/// Same as [`parse`]. Any configured resource budget violation returns
/// [`ParseErrorKind::ResourceLimit`](crate::ParseErrorKind::ResourceLimit).
///
/// # Examples
///
/// ```
/// use texpose::{parse_with_options, ParseOptions};
///
/// let tight = ParseOptions::new().with_max_depth(4);
/// assert!(parse_with_options(r"\frac{1}{\frac{1}{\frac{1}{2}}}", &tight).is_err());
/// ```
pub fn parse_with_options(
    input: &str,
    options: &ParseOptions,
) -> Result<(MathNode, ColorTable), ParseError> {
    let sanitized = preprocess(input);
    let tokens = tokenize_spanned_with_limit(&sanitized, options.max_tokens)?;
    let tokens = strip_fences(&tokens)?;
    let mut p = Parser {
        tokens,
        pos: 0,
        depth: 0,
        max_depth: options.max_depth,
        rows: 0,
        max_rows: options.max_environment_rows,
        cells: 0,
        max_cells: options.max_environment_cells,
        source_len: sanitized.len(),
        colors: ColorTable::new(),
    };
    let node = p.parse_list(Stop::eof())?;
    p.skip_ws();
    if p.pos < p.tokens.len() {
        let span = p.current_span();
        let token = p.tokens[p.pos].token.to_string();
        return Err(ParseError::malformed(
            ParseErrorKind::MalformedArgument,
            span,
            format!("unexpected leftover token {token}"),
        ));
    }
    enforce_ast_node_limit(&node, options.max_ast_nodes, sanitized.len())?;
    Ok((node, p.colors))
}

#[derive(Clone, Copy)]
struct Stop {
    end_group: bool,
    amp: bool,
    cr: bool,
    right: bool,
    end_env: bool,
    rbracket: bool,
}

impl Stop {
    fn eof() -> Self {
        Self {
            end_group: false,
            amp: false,
            cr: false,
            right: false,
            end_env: false,
            rbracket: false,
        }
    }

    fn group() -> Self {
        Self {
            end_group: true,
            ..Self::eof()
        }
    }

    fn cell() -> Self {
        Self {
            amp: true,
            cr: true,
            end_env: true,
            ..Self::eof()
        }
    }

    fn delim() -> Self {
        Self {
            right: true,
            ..Self::eof()
        }
    }

    fn index() -> Self {
        Self {
            rbracket: true,
            ..Self::eof()
        }
    }

    fn substack_line() -> Self {
        Self {
            end_group: true,
            cr: true,
            ..Self::eof()
        }
    }
}

#[derive(Clone, Copy)]
enum InfixFractionKind {
    Over,
    Choose,
}

enum ParsedList {
    Plain(MathNode),
    Infix {
        kind: InfixFractionKind,
        numerator: MathNode,
        denominator: MathNode,
    },
}

impl ParsedList {
    fn into_node(self) -> MathNode {
        match self {
            Self::Plain(node) => node,
            Self::Infix {
                kind,
                numerator,
                denominator,
            } => {
                let spec = match kind {
                    InfixFractionKind::Over => FractionSpec::ordinary(numerator, denominator),
                    InfixFractionKind::Choose => FractionSpec {
                        style: FractionStyle::Inherit,
                        rule: FractionRule::None,
                        left_delimiter: Delimiter::Char('('),
                        right_delimiter: Delimiter::Char(')'),
                        numerator_alignment: FractionAlignment::Default,
                        numerator: Box::new(numerator),
                        denominator: Box::new(denominator),
                    },
                };
                MathNode::Fraction(spec)
            }
        }
    }

    fn prepend(self, mut items: Vec<MathNode>) -> Self {
        match self {
            Self::Plain(node) => {
                items.push(node);
                Self::Plain(wrap_row(items))
            }
            Self::Infix {
                kind,
                numerator,
                denominator,
            } => {
                items.push(numerator);
                Self::Infix {
                    kind,
                    numerator: wrap_row(items),
                    denominator,
                }
            }
        }
    }

    fn with_color(self, color: Color) -> Self {
        match self {
            Self::Plain(node) => Self::Plain(MathNode::Color(color, Box::new(node))),
            Self::Infix {
                kind,
                numerator,
                denominator,
            } => Self::Infix {
                kind,
                numerator: MathNode::Color(color, Box::new(numerator)),
                denominator: MathNode::Color(color, Box::new(denominator)),
            },
        }
    }

    fn with_math_alphabet(self, style: TextStyle) -> Self {
        match self {
            Self::Plain(node) => Self::Plain(collapse_runs(apply_math_alphabet(node, style))),
            Self::Infix {
                kind,
                numerator,
                denominator,
            } => Self::Infix {
                kind,
                numerator: collapse_runs(apply_math_alphabet(numerator, style)),
                denominator: collapse_runs(apply_math_alphabet(denominator, style)),
            },
        }
    }
}

struct SpannedText {
    text: String,
    span: SourceSpan,
}

struct Parser {
    tokens: Vec<SpannedToken>,
    pos: usize,
    colors: ColorTable,
    /// Current nesting depth, bounded by `max_depth`.
    depth: usize,
    /// Deepest nesting accepted, from [`ParseOptions::max_depth`].
    max_depth: usize,
    rows: usize,
    max_rows: usize,
    cells: usize,
    max_cells: usize,
    source_len: usize,
}

impl Parser {
    fn skip_ws(&mut self) {
        while matches!(
            self.tokens.get(self.pos).map(|item| &item.token),
            Some(Token::Space)
        ) {
            self.pos += 1;
        }
    }

    fn peek(&self) -> Option<&Token> {
        self.tokens.get(self.pos).map(|item| &item.token)
    }

    fn current_span(&self) -> SourceSpan {
        self.tokens
            .get(self.pos)
            .map_or(SourceSpan::point(self.source_len), |item| item.span)
    }

    fn peek_ws(&mut self) -> Option<&Token> {
        self.skip_ws();
        self.peek()
    }

    fn bump(&mut self) -> Option<Token> {
        self.skip_ws();
        self.bump_raw()
    }

    fn bump_raw(&mut self) -> Option<Token> {
        let t = self.tokens.get(self.pos)?.token.clone();
        self.pos += 1;
        Some(t)
    }

    fn is_stop(&self, tok: &Token, stop: Stop) -> bool {
        match tok {
            Token::EndGroup if stop.end_group => true,
            Token::AlignmentTab if stop.amp => true,
            Token::Command(s) if s == "\\" && stop.cr => true,
            Token::Command(s) if s == "cr" && stop.cr => true,
            Token::Command(s) if s == "right" && stop.right => true,
            Token::Command(s) if s == "end" && stop.end_env => true,
            Token::Char(']') if stop.rbracket => true,
            _ => false,
        }
    }

    /// Run `f` one level deeper, refusing to descend past `max_depth`.
    fn nested<T>(
        &mut self,
        f: impl FnOnce(&mut Self) -> Result<T, ParseError>,
    ) -> Result<T, ParseError> {
        let span = self.current_span();
        let next = self.depth.checked_add(1).ok_or_else(|| {
            ParseError::resource_limit(span, ParseResource::NestingDepth, self.max_depth)
        })?;
        if next > self.max_depth {
            return Err(ParseError::resource_limit(
                span,
                ParseResource::NestingDepth,
                self.max_depth,
            ));
        }
        self.depth = next;
        let out = f(self);
        self.depth -= 1;
        out
    }

    fn consume_environment_row(&mut self, span: SourceSpan) -> Result<(), ParseError> {
        let next = self.rows.checked_add(1).ok_or_else(|| {
            ParseError::resource_limit(span, ParseResource::EnvironmentRows, self.max_rows)
        })?;
        if next > self.max_rows {
            return Err(ParseError::resource_limit(
                span,
                ParseResource::EnvironmentRows,
                self.max_rows,
            ));
        }
        self.rows = next;
        Ok(())
    }

    fn consume_environment_cell(&mut self, span: SourceSpan) -> Result<(), ParseError> {
        let next = self.cells.checked_add(1).ok_or_else(|| {
            ParseError::resource_limit(span, ParseResource::EnvironmentCells, self.max_cells)
        })?;
        if next > self.max_cells {
            return Err(ParseError::resource_limit(
                span,
                ParseResource::EnvironmentCells,
                self.max_cells,
            ));
        }
        self.cells = next;
        Ok(())
    }

    fn malformed_here(&self, kind: ParseErrorKind, what: impl Into<String>) -> ParseError {
        ParseError::malformed(kind, self.current_span(), what.into())
    }

    fn malformed_at(
        &self,
        kind: ParseErrorKind,
        span: SourceSpan,
        what: impl Into<String>,
    ) -> ParseError {
        ParseError::malformed(kind, span, what.into())
    }

    fn parse_list(&mut self, stop: Stop) -> Result<MathNode, ParseError> {
        self.nested(|p| p.parse_list_inner(stop, true).map(ParsedList::into_node))
    }

    fn parse_list_inner(
        &mut self,
        stop: Stop,
        allow_infix_fraction: bool,
    ) -> Result<ParsedList, ParseError> {
        let mut items = Vec::new();
        loop {
            self.skip_ws();
            let Some(tok) = self.peek().cloned() else {
                break;
            };
            if self.is_stop(&tok, stop) {
                break;
            }
            if let Token::Command(name) = &tok {
                if let Some(style) = plain_tex_math_alphabet(name) {
                    self.bump();
                    let rest = self
                        .parse_list_inner(stop, allow_infix_fraction)?
                        .with_math_alphabet(style)
                        .prepend(items);
                    return Ok(rest);
                }
            }
            match &tok {
                Token::MathShift | Token::DisplayShift => {
                    return Err(self.malformed_here(
                        ParseErrorKind::MalformedArgument,
                        "unexpected math shift",
                    ));
                }
                Token::Command(n) if n == "over" || n == "choose" => {
                    if !stop.end_group {
                        return Err(self.malformed_here(
                            ParseErrorKind::MalformedArgument,
                            format!("\\{n} outside a group"),
                        ));
                    }
                    if !allow_infix_fraction {
                        return Err(self.malformed_here(
                            ParseErrorKind::MalformedArgument,
                            format!("multiple generalized fraction operators in one group: \\{n}"),
                        ));
                    }
                    self.bump();
                    let denominator = self.parse_list_inner(stop, false)?.into_node();
                    return Ok(ParsedList::Infix {
                        kind: if n == "choose" {
                            InfixFractionKind::Choose
                        } else {
                            InfixFractionKind::Over
                        },
                        numerator: wrap_row(items),
                        denominator,
                    });
                }
                Token::Command(n) if n == "color" => {
                    self.bump();
                    let color = self.parse_color_from_cmd()?;
                    let rest = self
                        .parse_list_inner(stop, allow_infix_fraction)?
                        .with_color(color)
                        .prepend(items);
                    return Ok(rest);
                }
                Token::Command(n) if n == "definecolor" => {
                    self.bump();
                    self.parse_definecolor()?;
                    continue;
                }
                _ => {}
            }
            items.push(self.parse_atom()?);
        }
        Ok(ParsedList::Plain(wrap_row(items)))
    }

    fn parse_atom(&mut self) -> Result<MathNode, ParseError> {
        let mut nucleus = self.parse_nucleus()?;
        let mut limit_mode = None;
        loop {
            match self.peek_ws() {
                Some(Token::Command(n)) if n == "limits" => {
                    let span = self.current_span();
                    self.bump();
                    limit_mode = Some((LimitMode::Limits, span));
                }
                Some(Token::Command(n)) if n == "nolimits" => {
                    let span = self.current_span();
                    self.bump();
                    limit_mode = Some((LimitMode::NoLimits, span));
                }
                _ => break,
            }
        }
        if let Some((mode, span)) = limit_mode {
            if !accepts_limit_control(&nucleus) {
                return Err(self.malformed_at(
                    ParseErrorKind::MalformedArgument,
                    span,
                    "\\limits/\\nolimits requires an operator nucleus",
                ));
            }
            nucleus = MathNode::Limits(Box::new(nucleus), mode);
        }
        self.bind_scripts(nucleus)
    }

    fn parse_nucleus(&mut self) -> Result<MathNode, ParseError> {
        self.skip_ws();
        match self.peek().cloned() {
            None => {
                Err(self
                    .malformed_here(ParseErrorKind::MalformedArgument, "unexpected end of input"))
            }
            Some(Token::Superscript | Token::Subscript | Token::Char('\'')) => {
                Ok(MathNode::Row(Vec::new()))
            }
            Some(Token::BeginGroup) => self.parse_group(),
            Some(Token::EndGroup) => {
                Err(self.malformed_here(ParseErrorKind::UnexpectedGroupEnd, "unexpected '}'"))
            }
            Some(Token::Char(c)) => {
                self.bump();
                Ok(MathNode::Atom(c, atom_kind(c)))
            }
            Some(Token::Command(name)) => {
                let command_span = self.current_span();
                self.bump();
                self.parse_command(&name, command_span)
            }
            Some(other) => Err(self.malformed_here(
                ParseErrorKind::MalformedArgument,
                format!("unexpected token {other}"),
            )),
        }
    }

    fn parse_group(&mut self) -> Result<MathNode, ParseError> {
        self.skip_ws();
        let open_span = self.current_span();
        match self.bump() {
            Some(Token::BeginGroup) => {}
            _ => {
                return Err(self.malformed_at(
                    ParseErrorKind::MalformedArgument,
                    open_span,
                    "expected '{'",
                ))
            }
        }
        let inner = self.parse_list(Stop::group())?;
        match self.bump() {
            Some(Token::EndGroup) => Ok(inner),
            _ => Err(self.malformed_at(ParseErrorKind::UnclosedGroup, open_span, "unmatched '{'")),
        }
    }

    fn parse_arg(&mut self) -> Result<MathNode, ParseError> {
        self.nested(Self::parse_arg_inner)
    }

    fn parse_arg_inner(&mut self) -> Result<MathNode, ParseError> {
        self.skip_ws();
        match self.peek() {
            Some(Token::BeginGroup) => self.parse_group(),
            Some(_) => self.parse_nucleus(),
            None => Err(self.malformed_here(ParseErrorKind::MalformedArgument, "missing argument")),
        }
    }

    fn parse_script(&mut self) -> Result<MathNode, ParseError> {
        self.parse_arg()
    }

    fn bind_scripts(&mut self, nucleus: MathNode) -> Result<MathNode, ParseError> {
        let mut sub: Option<MathNode> = None;
        let mut sup: Option<MathNode> = None;
        let mut sup_from_prime = false;
        loop {
            match self.peek_ws() {
                Some(Token::Subscript) => {
                    let operator_span = self.current_span();
                    self.bump();
                    if sub.is_some() {
                        return Err(self.malformed_at(
                            ParseErrorKind::MalformedArgument,
                            operator_span,
                            "double subscript",
                        ));
                    }
                    sub = Some(self.parse_script()?);
                }
                Some(Token::Superscript) => {
                    let operator_span = self.current_span();
                    self.bump();
                    let s = self.parse_script()?;
                    if let Some(prev) = sup.take() {
                        if !sup_from_prime {
                            return Err(self.malformed_at(
                                ParseErrorKind::MalformedArgument,
                                operator_span,
                                "double superscript",
                            ));
                        }
                        sup = Some(wrap_row(vec![prev, s]));
                        sup_from_prime = false;
                    } else {
                        sup = Some(s);
                    }
                }
                Some(Token::Char('\'')) => {
                    self.bump();
                    let prime = MathNode::Atom('′', AtomKind::Ord);
                    sup = Some(match sup.take() {
                        None => prime,
                        Some(prev) => wrap_row(vec![prev, prime]),
                    });
                    sup_from_prime = true;
                }
                _ => break,
            }
        }
        Ok(apply_scripts(nucleus, sub, sup))
    }

    fn parse_command(
        &mut self,
        name: &str,
        command_span: SourceSpan,
    ) -> Result<MathNode, ParseError> {
        match name {
            "frac" | "dfrac" | "tfrac" => {
                let numerator = self.parse_arg()?;
                let denominator = self.parse_arg()?;
                let style = match name {
                    "dfrac" => FractionStyle::Display,
                    "tfrac" => FractionStyle::Text,
                    _ => FractionStyle::Inherit,
                };
                Ok(MathNode::Fraction(FractionSpec {
                    style,
                    rule: FractionRule::Default,
                    left_delimiter: Delimiter::Empty,
                    right_delimiter: Delimiter::Empty,
                    numerator_alignment: FractionAlignment::Default,
                    numerator: Box::new(numerator),
                    denominator: Box::new(denominator),
                }))
            }
            "cfrac" => self.parse_cfrac(),
            "binom" | "dbinom" | "tbinom" => {
                let numerator = self.parse_arg()?;
                let denominator = self.parse_arg()?;
                let style = match name {
                    "dbinom" => FractionStyle::Display,
                    "tbinom" => FractionStyle::Text,
                    _ => FractionStyle::Inherit,
                };
                Ok(MathNode::Fraction(FractionSpec {
                    style,
                    rule: FractionRule::None,
                    left_delimiter: Delimiter::Char('('),
                    right_delimiter: Delimiter::Char(')'),
                    numerator_alignment: FractionAlignment::Default,
                    numerator: Box::new(numerator),
                    denominator: Box::new(denominator),
                }))
            }
            "genfrac" => self.parse_genfrac(),
            "sqrt" => {
                let index = if matches!(self.peek_ws(), Some(Token::Char('['))) {
                    self.bump();
                    let idx = self.parse_list(Stop::index())?;
                    let close_span = self.current_span();
                    match self.bump() {
                        Some(Token::Char(']')) => {}
                        _ => {
                            return Err(self.malformed_at(
                                ParseErrorKind::MalformedArgument,
                                close_span,
                                "expected ']' after \\sqrt index",
                            ))
                        }
                    }
                    Some(Box::new(idx))
                } else {
                    None
                };
                let rad = self.parse_arg()?;
                Ok(MathNode::Radical(index, Box::new(rad)))
            }
            "left" => self.parse_delimited(),
            "right" => Err(ParseError::unmatched_delimiter(command_span)),
            "begin" => self.parse_begin(),
            "end" => Err(self.malformed_at(
                ParseErrorKind::MalformedMatrix,
                command_span,
                "unexpected \\end",
            )),
            "over" => Err(self.malformed_at(
                ParseErrorKind::MalformedArgument,
                command_span,
                "\\over outside a group",
            )),
            "choose" => Err(self.malformed_at(
                ParseErrorKind::MalformedArgument,
                command_span,
                "\\choose outside a group",
            )),
            "hat" => self.accent(AccentKind::Hat),
            "check" => self.accent(AccentKind::Check),
            "breve" => self.accent(AccentKind::Breve),
            "acute" => self.accent(AccentKind::Acute),
            "grave" => self.accent(AccentKind::Grave),
            "tilde" => self.accent(AccentKind::Tilde),
            "bar" => self.accent(AccentKind::Bar),
            "vec" => self.accent(AccentKind::Vec),
            "dot" => self.accent(AccentKind::Dot),
            "ddot" => self.accent(AccentKind::Ddot),
            "dddot" => self.accent(AccentKind::Dddot),
            "ddddot" => self.accent(AccentKind::Ddddot),
            "widehat" => self.accent(AccentKind::WideHat),
            "widetilde" => self.accent(AccentKind::WideTilde),
            "overline" => self.accent(AccentKind::Overline),
            "underline" => self.accent(AccentKind::Underline),
            "overbrace" => self.accent(AccentKind::Overbrace),
            "underbrace" => self.accent(AccentKind::Underbrace),
            "overleftarrow" => self.accent(AccentKind::Overleftarrow),
            "overrightarrow" => self.accent(AccentKind::Overrightarrow),
            "overleftrightarrow" => self.accent(AccentKind::Overleftrightarrow),
            "underleftarrow" => self.accent(AccentKind::Underleftarrow),
            "underrightarrow" => self.accent(AccentKind::Underrightarrow),
            "underleftrightarrow" => self.accent(AccentKind::Underleftrightarrow),
            "cancel" => self.accent(AccentKind::Cancel),
            "bcancel" => self.accent(AccentKind::BCancel),
            "xcancel" => self.accent(AccentKind::XCancel),
            "boxed" | "fbox" => self.accent(AccentKind::Boxed),
            "mathring" => self.accent(AccentKind::Ring),
            "cancelto" => {
                let value = self.parse_arg()?;
                let expr = self.parse_arg()?;
                if is_empty_node(&expr) {
                    return Err(
                        self.malformed_here(ParseErrorKind::MalformedArgument, "empty accent base")
                    );
                }
                Ok(MathNode::CancelTo(Box::new(value), Box::new(expr)))
            }
            "not" => {
                let body = self.parse_nucleus()?;
                Ok(MathNode::Accent(Box::new(body), AccentKind::Not))
            }
            "overset" => {
                let over = self.parse_arg()?;
                let base = self.parse_arg()?;
                Ok(MathNode::OverUnder(
                    Box::new(base),
                    Some(Box::new(over)),
                    None,
                ))
            }
            "underset" => {
                let under = self.parse_arg()?;
                let base = self.parse_arg()?;
                Ok(MathNode::OverUnder(
                    Box::new(base),
                    None,
                    Some(Box::new(under)),
                ))
            }
            "stackrel" => {
                let over = self.parse_arg()?;
                let base = self.parse_arg()?;
                Ok(MathNode::StackRel(Box::new(base), Box::new(over)))
            }
            "mathrm" => self.math_alphabet(TextStyle::Rm),
            "mathbf" => self.math_alphabet(TextStyle::Bf),
            "mathit" => self.math_alphabet(TextStyle::It),
            "mathsf" => self.math_alphabet(TextStyle::Sf),
            "mathtt" => self.math_alphabet(TextStyle::Tt),
            "mathbb" => self.math_alphabet(TextStyle::Bb),
            "mathcal" => self.math_alphabet(TextStyle::Cal),
            "mathfrak" => self.math_alphabet(TextStyle::Frak),
            "mathscr" => self.math_alphabet(TextStyle::Scr),
            "boldsymbol" => self.math_alphabet(TextStyle::Boldsymbol),
            "pmb" => self.math_alphabet(TextStyle::Pmb),
            "textrm" | "textbf" | "textit" | "textsf" | "texttt" => {
                Err(ParseError::unsupported(command_span, format!("\\{name}")))
            }
            "xrightarrow" => self.parse_xarrow("longrightarrow"),
            "xleftarrow" => self.parse_xarrow("longleftarrow"),
            "text" | "mbox" => self.parse_literal_text(),
            "operatorname" => {
                let name = self.collect_group_text()?;
                Ok(MathNode::Operator(name, false))
            }
            "," => Ok(MathNode::Space(SpaceKind::Thin)),
            ":" | ">" => Ok(MathNode::Space(SpaceKind::Medium)),
            ";" => Ok(MathNode::Space(SpaceKind::Thick)),
            "!" => Ok(MathNode::Space(SpaceKind::NegThin)),
            "quad" => Ok(MathNode::Space(SpaceKind::Quad)),
            "qquad" => Ok(MathNode::Space(SpaceKind::Qquad)),
            " " => Ok(MathNode::Space(SpaceKind::ControlSpace)),
            "hspace" => {
                let spec = self.collect_group_text_spanned()?;
                let d = parse_tex_dim(&spec.text, spec.span)?;
                Ok(MathNode::Space(SpaceKind::Hspace(d)))
            }
            "phantom" => {
                let b = self.parse_arg()?;
                Ok(MathNode::Phantom(PhantomKind::Full, Box::new(b)))
            }
            "vphantom" => {
                let b = self.parse_arg()?;
                Ok(MathNode::Phantom(PhantomKind::Vertical, Box::new(b)))
            }
            "hphantom" => {
                let b = self.parse_arg()?;
                Ok(MathNode::Phantom(PhantomKind::Horizontal, Box::new(b)))
            }
            "strut" => Ok(MathNode::Strut(
                Length::Em(Dim::ratio(7, 10).map_err(|e| numeric_parse_err(e, command_span))?),
                Length::Em(Dim::ratio(3, 10).map_err(|e| numeric_parse_err(e, command_span))?),
            )),
            "rule" => {
                let width = self.collect_group_text_spanned()?;
                let height = self.collect_group_text_spanned()?;
                Ok(MathNode::Rule(
                    parse_tex_dim(&width.text, width.span)?,
                    parse_tex_dim(&height.text, height.span)?,
                ))
            }
            "textcolor" => {
                let c = self.parse_color_from_cmd()?;
                let body = self.parse_arg()?;
                Ok(MathNode::TextColor(c, Box::new(body)))
            }
            "colorbox" => {
                let c = self.parse_color_from_cmd()?;
                let body = self.parse_arg()?;
                Ok(MathNode::ColorBox(c, Box::new(body)))
            }
            "fcolorbox" => {
                let border = self.parse_color_from_cmd()?;
                let fill = self.parse_color_from_cmd()?;
                let body = self.parse_arg()?;
                Ok(MathNode::FColorBox(border, fill, Box::new(body)))
            }
            "sum" => Ok(MathNode::Sum(None, None)),
            "prod" => Ok(MathNode::Product(None, None)),
            "int" => Ok(MathNode::Integral(IntegralKind::Int, None, None)),
            "iint" => Ok(MathNode::Integral(IntegralKind::Iint, None, None)),
            "iiint" => Ok(MathNode::Integral(IntegralKind::Iiint, None, None)),
            "oint" => Ok(MathNode::Integral(IntegralKind::Oint, None, None)),
            "oiint" => Ok(MathNode::Integral(IntegralKind::Oiint, None, None)),
            "lim" => Ok(MathNode::Limit(None)),
            "sin" | "cos" | "tan" | "cot" | "sec" | "csc" | "arcsin" | "arccos" | "arctan"
            | "sinh" | "cosh" | "tanh" | "coth" | "log" | "ln" | "lg" | "exp" | "limsup"
            | "liminf" | "sup" | "inf" | "max" | "min" | "det" | "dim" | "ker" | "deg" | "gcd"
            | "lcm" | "Pr" | "arg" => Ok(MathNode::Operator(name.to_string(), false)),
            "coprod" | "bigcup" | "bigcap" | "bigsqcup" | "bigvee" | "bigwedge" | "bigoplus"
            | "bigotimes" | "biguplus" => Ok(MathNode::Operator(name.to_string(), true)),
            "big" | "Big" | "bigg" | "Bigg" | "bigl" | "bigr" | "Bigl" | "Bigr" | "biggl"
            | "biggr" | "Biggl" | "Biggr" | "bigm" | "Bigm" | "biggm" | "Biggm" => {
                self.parse_sized_delim(name)
            }
            "tag" => {
                let star = matches!(self.peek_ws(), Some(Token::Char('*')));
                if star {
                    self.bump();
                }
                let body = self.parse_arg()?;
                Ok(MathNode::Tag {
                    star,
                    body: Box::new(body),
                })
            }
            "label" => {
                let key = self.collect_group_text()?;
                Ok(MathNode::Label(key))
            }
            "ref" => {
                let key = self.collect_group_text()?;
                Ok(MathNode::Ref(key))
            }
            "nonumber" | "notag" => Ok(MathNode::NoNumber),
            "hline" => Ok(MathNode::Hline),
            "intertext" => {
                let s = self.collect_literal_text()?;
                Ok(MathNode::Intertext(Box::new(MathNode::LiteralText(s))))
            }
            "substack" => self.parse_substack(),
            "displaystyle" => Ok(MathNode::Style(MathStyleDeclaration::Display)),
            "textstyle" => Ok(MathNode::Style(MathStyleDeclaration::Text)),
            "scriptstyle" => Ok(MathNode::Style(MathStyleDeclaration::Script)),
            "scriptscriptstyle" => Ok(MathNode::Style(MathStyleDeclaration::ScriptScript)),
            "limits" | "nolimits" => Err(self.malformed_at(
                ParseErrorKind::MalformedArgument,
                command_span,
                "limit control without a preceding operator",
            )),
            "{" | "}" => {
                let c = name.chars().next().unwrap_or('{');
                Ok(MathNode::Atom(
                    c,
                    if name == "{" {
                        AtomKind::Open
                    } else {
                        AtomKind::Close
                    },
                ))
            }
            "|" => Ok(MathNode::Symbol("Vert".into())),
            "backslash" => Ok(MathNode::Symbol("backslash".into())),
            _ => {
                if name.starts_with("math")
                    && name.len() > 4
                    && name.chars().all(|c| c.is_ascii_alphabetic())
                {
                    return Err(ParseError::unsupported(
                        command_span,
                        format!("font style {name}"),
                    ));
                }
                if name.starts_with("wide") {
                    return Err(ParseError::unsupported(
                        command_span,
                        format!("accent {name}"),
                    ));
                }
                self.parse_symbol_or_unknown(name, command_span)
            }
        }
    }

    fn accent(&mut self, kind: AccentKind) -> Result<MathNode, ParseError> {
        let body = self.parse_arg()?;
        if is_empty_node(&body) {
            return Err(self.malformed_here(ParseErrorKind::MalformedArgument, "empty accent base"));
        }
        Ok(MathNode::Accent(Box::new(body), kind))
    }

    fn parse_xarrow(&mut self, arrow: &str) -> Result<MathNode, ParseError> {
        let under = if matches!(self.peek_ws(), Some(Token::Char('['))) {
            self.bump();
            let u = self.parse_list(Stop::index())?;
            let close_span = self.current_span();
            match self.bump() {
                Some(Token::Char(']')) => Some(Box::new(u)),
                _ => {
                    return Err(self.malformed_at(
                        ParseErrorKind::MalformedArgument,
                        close_span,
                        "expected ']' after x-arrow optional argument",
                    ))
                }
            }
        } else {
            None
        };
        let over = self.parse_arg()?;
        Ok(MathNode::OverUnder(
            Box::new(MathNode::Symbol(arrow.to_string())),
            Some(Box::new(over)),
            under,
        ))
    }

    fn math_alphabet(&mut self, style: TextStyle) -> Result<MathNode, ParseError> {
        let inner = self.parse_arg()?;
        Ok(collapse_runs(apply_math_alphabet(inner, style)))
    }

    fn parse_literal_text(&mut self) -> Result<MathNode, ParseError> {
        Ok(MathNode::LiteralText(self.collect_literal_text()?))
    }

    fn parse_delimited(&mut self) -> Result<MathNode, ParseError> {
        let open = self.parse_delimiter()?;
        let body = self.parse_list(Stop::delim())?;
        match self.bump() {
            Some(Token::Command(n)) if n == "right" => {}
            _ => return Err(ParseError::unmatched_delimiter(self.current_span())),
        }
        let close = self.parse_delimiter()?;
        Ok(MathNode::Delimited(open, Box::new(body), close))
    }

    fn parse_sized_delim(&mut self, name: &str) -> Result<MathNode, ParseError> {
        let size = DelimSize::from_command(name).ok_or_else(|| {
            self.malformed_here(
                ParseErrorKind::MalformedArgument,
                format!("unknown delimiter size \\{name}"),
            )
        })?;
        let d = self.parse_delimiter()?;
        let class = DelimSize::class_from_command(name).unwrap_or_else(|| match &d {
            Delimiter::Char(c) => atom_kind(*c),
            Delimiter::Named(n) if n == "{" => AtomKind::Open,
            Delimiter::Named(n) if n == "}" => AtomKind::Close,
            _ => AtomKind::Open,
        });
        Ok(MathNode::SizedDelim(d, size, class))
    }

    fn parse_delimiter(&mut self) -> Result<Delimiter, ParseError> {
        self.skip_ws();
        let delimiter_span = self.current_span();
        match self.bump() {
            Some(Token::Char('.')) => Ok(Delimiter::Empty),
            Some(Token::Char(c)) if matches!(c, '(' | ')' | '[' | ']' | '|' | '/' | '<' | '>') => {
                Ok(Delimiter::Char(c))
            }
            Some(Token::Command(n)) => match n.as_str() {
                "." => Ok(Delimiter::Empty),
                "{" | "}" | "|" => Ok(Delimiter::Named(n)),
                "langle" | "rangle" | "lfloor" | "rfloor" | "lceil" | "rceil" | "lvert"
                | "rvert" | "lVert" | "rVert" | "vert" | "Vert" | "uparrow" | "downarrow"
                | "Uparrow" | "Downarrow" | "updownarrow" | "Updownarrow" | "backslash"
                | "lgroup" | "rgroup" | "lmoustache" | "rmoustache" => Ok(Delimiter::Named(n)),
                other => Err(self.malformed_at(
                    ParseErrorKind::MalformedArgument,
                    delimiter_span,
                    format!("unknown delimiter \\{other}"),
                )),
            },
            Some(other) => Err(self.malformed_at(
                ParseErrorKind::MalformedArgument,
                delimiter_span,
                format!("expected delimiter, found {other}"),
            )),
            None => Err(self.malformed_at(
                ParseErrorKind::MalformedArgument,
                delimiter_span,
                "expected delimiter",
            )),
        }
    }

    fn parse_begin(&mut self) -> Result<MathNode, ParseError> {
        let name = self.collect_group_text_spanned()?;
        let colspec = if name.text == "array" {
            let preamble = self.collect_group_text_spanned()?;
            parse_colspec(&preamble.text, preamble.span)?
        } else {
            Vec::new()
        };
        let style = match name.text.as_str() {
            "matrix" => MatrixStyle::Matrix,
            "pmatrix" => MatrixStyle::Pmatrix,
            "bmatrix" => MatrixStyle::Bmatrix,
            "vmatrix" => MatrixStyle::Vmatrix,
            "Vmatrix" => MatrixStyle::VVmatrix,
            "Bmatrix" => MatrixStyle::BBmatrix,
            "cases" => MatrixStyle::Cases,
            "array" => MatrixStyle::Array,
            "aligned" => MatrixStyle::Aligned,
            "align" => MatrixStyle::Align,
            "gather" => MatrixStyle::Gather,
            "multline" => MatrixStyle::Multline,
            "equation" => MatrixStyle::Equation,
            "split" => MatrixStyle::Split,
            other => {
                return Err(ParseError::unsupported(
                    name.span,
                    format!("environment {other}"),
                ));
            }
        };
        let rows = self.parse_rows()?;
        self.expect_end(&name.text)?;
        Ok(MathNode::Matrix(style, colspec, rows))
    }

    fn parse_substack(&mut self) -> Result<MathNode, ParseError> {
        self.skip_ws();
        let open_span = self.current_span();
        match self.bump() {
            Some(Token::BeginGroup) => {}
            _ => {
                return Err(self.malformed_at(
                    ParseErrorKind::MalformedArgument,
                    open_span,
                    "expected '{' after \\substack",
                ))
            }
        }
        let mut lines = Vec::new();
        loop {
            self.skip_ws();
            if matches!(self.peek(), Some(Token::EndGroup)) {
                self.bump();
                break;
            }
            let line = self.parse_list(Stop::substack_line())?;
            lines.push(line);
            self.skip_ws();
            match self.peek() {
                Some(Token::Command(n)) if n == "\\" || n == "cr" => {
                    self.bump();
                }
                Some(Token::EndGroup) => {
                    self.bump();
                    break;
                }
                None => {
                    return Err(self.malformed_here(
                        ParseErrorKind::MalformedArgument,
                        "unmatched '{' in \\substack",
                    ))
                }
                Some(other) => {
                    return Err(self.malformed_here(
                        ParseErrorKind::MalformedArgument,
                        format!("unexpected token {other} in \\substack"),
                    ))
                }
            }
        }
        if lines.is_empty() {
            return Err(self.malformed_here(ParseErrorKind::MalformedArgument, "empty \\substack"));
        }
        Ok(MathNode::Substack(lines))
    }

    fn parse_rows(&mut self) -> Result<Vec<EnvRow>, ParseError> {
        self.skip_ws();
        if matches!(self.peek(), Some(Token::Command(n)) if n == "end") {
            return Ok(Vec::new());
        }
        let mut rows = Vec::new();
        loop {
            self.skip_ws();
            if matches!(self.peek(), Some(Token::Command(n)) if n == "end") {
                return Ok(rows);
            }
            let row_span = self.current_span();
            self.consume_environment_row(row_span)?;
            if matches!(self.peek(), Some(Token::Command(n)) if n == "hline") {
                self.bump();
                rows.push(EnvRow::Hline);
                continue;
            }
            if matches!(self.peek(), Some(Token::Command(n)) if n == "intertext") {
                self.bump();
                let text = self.collect_literal_text()?;
                rows.push(EnvRow::Intertext(Box::new(MathNode::LiteralText(text))));
                continue;
            }
            let mut cells = Vec::new();
            let mut number = EqNumber::Default;
            let mut labels = Vec::new();
            loop {
                self.skip_ws();
                let cell_span = self.current_span();
                self.consume_environment_cell(cell_span)?;
                let cell = self.parse_list(Stop::cell())?;
                let cell = peel_row_meta(cell, &mut number, &mut labels);
                cells.push(cell);
                self.skip_ws();
                match self.peek() {
                    Some(Token::AlignmentTab) => {
                        self.bump();
                    }
                    Some(Token::Command(n)) if n == "\\" || n == "cr" => {
                        self.bump();
                        rows.push(finish_env_row(cells, number, labels));
                        self.skip_ws();
                        if matches!(self.peek(), Some(Token::Command(e)) if e == "end") {
                            return Ok(rows);
                        }
                        break;
                    }
                    Some(Token::Command(n)) if n == "end" => {
                        rows.push(finish_env_row(cells, number, labels));
                        return Ok(rows);
                    }
                    None => {
                        return Err(self.malformed_at(
                            ParseErrorKind::MalformedMatrix,
                            row_span,
                            "unmatched \\begin (missing \\end)",
                        ));
                    }
                    Some(other) => {
                        return Err(self.malformed_here(
                            ParseErrorKind::MalformedMatrix,
                            format!("unexpected token {other} in environment body"),
                        ));
                    }
                }
            }
        }
    }

    fn expect_end(&mut self, name: &str) -> Result<(), ParseError> {
        self.skip_ws();
        let end_span = self.current_span();
        match self.bump() {
            Some(Token::Command(n)) if n == "end" => {}
            _ => {
                return Err(self.malformed_at(
                    ParseErrorKind::MalformedMatrix,
                    end_span,
                    format!("expected \\end{{{name}}}"),
                ))
            }
        }
        let got = self.collect_group_text_spanned()?;
        if got.text != name {
            return Err(ParseError::mismatched_environment(
                got.span,
                name.to_string(),
                got.text,
            ));
        }
        Ok(())
    }

    fn parse_cfrac(&mut self) -> Result<MathNode, ParseError> {
        self.skip_ws();
        let numerator_alignment = if matches!(self.peek(), Some(Token::Char('['))) {
            self.bump();
            let alignment = self.collect_until_char(']')?;
            let close_span = self.current_span();
            match self.bump() {
                Some(Token::Char(']')) => {}
                _ => {
                    return Err(self.malformed_at(
                        ParseErrorKind::MalformedArgument,
                        close_span,
                        "expected ']' after \\cfrac alignment",
                    ))
                }
            }
            match alignment.trim() {
                "l" => FractionAlignment::Left,
                "r" => FractionAlignment::Right,
                "" => FractionAlignment::Center,
                other => {
                    return Err(self.malformed_here(
                        ParseErrorKind::MalformedArgument,
                        format!("invalid \\cfrac numerator alignment `{other}`"),
                    ))
                }
            }
        } else {
            FractionAlignment::Center
        };
        let numerator = self.parse_arg()?;
        let denominator = self.parse_arg()?;
        Ok(MathNode::Fraction(FractionSpec {
            style: FractionStyle::Display,
            rule: FractionRule::Default,
            left_delimiter: Delimiter::Empty,
            right_delimiter: Delimiter::Empty,
            numerator_alignment,
            numerator: Box::new(numerator),
            denominator: Box::new(denominator),
        }))
    }

    fn parse_genfrac(&mut self) -> Result<MathNode, ParseError> {
        let left_text = self.collect_group_text_spanned()?;
        let right_text = self.collect_group_text_spanned()?;
        let thickness_text = self.collect_group_text_spanned()?;
        let style_text = self.collect_group_text_spanned()?;
        let left_delimiter = delim_from_text(&left_text.text)?;
        let right_delimiter = delim_from_text(&right_text.text)?;
        let numerator = self.parse_arg()?;
        let denominator = self.parse_arg()?;

        let rule = if thickness_text.text.trim().is_empty() {
            FractionRule::Default
        } else {
            let thickness = parse_tex_dim(&thickness_text.text, thickness_text.span)?;
            let value = length_value(&thickness);
            if value < &Dim::zero() {
                return Err(self.malformed_at(
                    ParseErrorKind::MalformedArgument,
                    thickness_text.span,
                    "negative \\genfrac rule thickness",
                ));
            }
            if value.is_zero() {
                FractionRule::None
            } else {
                FractionRule::Exact(thickness)
            }
        };
        let style = match style_text.text.trim() {
            "" => FractionStyle::Inherit,
            "0" => FractionStyle::Display,
            "1" => FractionStyle::Text,
            "2" => FractionStyle::Script,
            "3" => FractionStyle::ScriptScript,
            other => {
                return Err(self.malformed_at(
                    ParseErrorKind::MalformedArgument,
                    style_text.span,
                    format!("invalid \\genfrac style `{other}`"),
                ))
            }
        };
        Ok(MathNode::Fraction(FractionSpec {
            style,
            rule,
            left_delimiter,
            right_delimiter,
            numerator_alignment: FractionAlignment::Default,
            numerator: Box::new(numerator),
            denominator: Box::new(denominator),
        }))
    }

    fn parse_color_from_cmd(&mut self) -> Result<Color, ParseError> {
        self.skip_ws();
        let model = if matches!(self.peek(), Some(Token::Char('['))) {
            self.bump();
            let m = self.collect_until_char(']')?;
            let close_span = self.current_span();
            match self.bump() {
                Some(Token::Char(']')) => {}
                _ => {
                    return Err(self.malformed_at(
                        ParseErrorKind::MalformedArgument,
                        close_span,
                        "expected ']' after color model",
                    ))
                }
            }
            m
        } else {
            "named".into()
        };
        let spec = self.collect_group_text_spanned()?;
        parse_color_spec(&model, &spec.text, Some(&self.colors))
            .map_err(|error| color_err(error, spec.span))
    }

    fn parse_definecolor(&mut self) -> Result<(), ParseError> {
        let name = self.collect_group_text()?;
        let model = self.collect_group_text()?;
        let spec = self.collect_group_text_spanned()?;
        self.colors
            .define(&name, &model, &spec.text)
            .map_err(|error| color_err(error, spec.span))?;
        Ok(())
    }

    fn collect_literal_text(&mut self) -> Result<String, ParseError> {
        self.skip_ws();
        let open_span = self.current_span();
        match self.bump_raw() {
            Some(Token::BeginGroup) => {}
            _ => {
                return Err(self.malformed_at(
                    ParseErrorKind::MalformedArgument,
                    open_span,
                    "expected '{'",
                ))
            }
        }

        let depth = self.depth.checked_add(1).ok_or_else(|| {
            ParseError::resource_limit(open_span, ParseResource::NestingDepth, self.max_depth)
        })?;
        if depth > self.max_depth {
            return Err(ParseError::resource_limit(
                open_span,
                ParseResource::NestingDepth,
                self.max_depth,
            ));
        }

        let mut text = String::new();
        loop {
            let token_span = self.current_span();
            match self.bump_raw() {
                None => {
                    return Err(self.malformed_at(
                        ParseErrorKind::UnclosedGroup,
                        open_span,
                        "unmatched '{'",
                    ))
                }
                Some(Token::EndGroup) => return Ok(text),
                Some(Token::Space) => text.push(' '),
                Some(Token::Char(c)) => text.push(c),
                Some(Token::Command(name)) => match name.as_str() {
                    "{" => text.push('{'),
                    "}" => text.push('}'),
                    "%" => text.push('%'),
                    "$" => text.push('$'),
                    "#" => text.push('#'),
                    "&" => text.push('&'),
                    "_" => text.push('_'),
                    " " => text.push(' '),
                    _ => {
                        return Err(ParseError::unsupported(
                            token_span,
                            format!("literal text command \\{name}"),
                        ))
                    }
                },
                Some(Token::BeginGroup) => {
                    return Err(ParseError::unsupported(
                        token_span,
                        "nested literal-text groups".into(),
                    ))
                }
                Some(other) => {
                    return Err(ParseError::unsupported(
                        token_span,
                        format!("literal text token {other}"),
                    ))
                }
            }
        }
    }

    fn collect_group_text(&mut self) -> Result<String, ParseError> {
        Ok(self.collect_group_text_spanned()?.text)
    }

    fn collect_group_text_spanned(&mut self) -> Result<SpannedText, ParseError> {
        self.skip_ws();
        let open_span = self.current_span();
        match self.bump_raw() {
            Some(Token::BeginGroup) => {}
            _ => {
                return Err(self.malformed_at(
                    ParseErrorKind::MalformedArgument,
                    open_span,
                    "expected '{'",
                ))
            }
        }
        let content_start = open_span.end;
        let initial_depth = self.depth.checked_add(1).ok_or_else(|| {
            ParseError::resource_limit(open_span, ParseResource::NestingDepth, self.max_depth)
        })?;
        if initial_depth > self.max_depth {
            return Err(ParseError::resource_limit(
                open_span,
                ParseResource::NestingDepth,
                self.max_depth,
            ));
        }
        let mut text = String::new();
        let mut depth = 1usize;
        loop {
            let token_span = self.current_span();
            match self.bump_raw() {
                None => {
                    return Err(self.malformed_at(
                        ParseErrorKind::UnclosedGroup,
                        open_span,
                        "unmatched '{'",
                    ))
                }
                Some(Token::BeginGroup) => {
                    let next = depth.checked_add(1).ok_or_else(|| {
                        ParseError::resource_limit(
                            token_span,
                            ParseResource::NestingDepth,
                            self.max_depth,
                        )
                    })?;
                    let total = self.depth.checked_add(next).ok_or_else(|| {
                        ParseError::resource_limit(
                            token_span,
                            ParseResource::NestingDepth,
                            self.max_depth,
                        )
                    })?;
                    if total > self.max_depth {
                        return Err(ParseError::resource_limit(
                            token_span,
                            ParseResource::NestingDepth,
                            self.max_depth,
                        ));
                    }
                    depth = next;
                    text.push('{');
                }
                Some(Token::EndGroup) => {
                    depth -= 1;
                    if depth == 0 {
                        return Ok(SpannedText {
                            text,
                            span: SourceSpan::new(content_start, token_span.start),
                        });
                    }
                    text.push('}');
                }
                Some(Token::Space) => text.push(' '),
                Some(Token::Char(c)) => text.push(c),
                Some(Token::Command(n)) => {
                    if n.len() == 1 {
                        text.push(n.chars().next().unwrap_or('\\'));
                    } else {
                        text.push('\\');
                        text.push_str(&n);
                    }
                }
                Some(other) => {
                    return Err(self.malformed_at(
                        ParseErrorKind::MalformedArgument,
                        token_span,
                        format!("unexpected token {other} in group text"),
                    ))
                }
            }
        }
    }

    fn collect_until_char(&mut self, end: char) -> Result<String, ParseError> {
        let mut s = String::new();
        loop {
            match self.peek() {
                None => {
                    return Err(self.malformed_here(
                        ParseErrorKind::MalformedArgument,
                        format!("expected '{end}'"),
                    ))
                }
                Some(Token::Char(c)) if *c == end => break,
                Some(Token::Char(c)) => {
                    s.push(*c);
                    self.bump_raw();
                }
                Some(Token::Space) => {
                    s.push(' ');
                    self.bump_raw();
                }
                Some(other) => {
                    return Err(self.malformed_here(
                        ParseErrorKind::MalformedArgument,
                        format!("unexpected token {other} in optional argument"),
                    ))
                }
            }
        }
        Ok(s)
    }

    fn parse_symbol_or_unknown(
        &self,
        name: &str,
        command_span: SourceSpan,
    ) -> Result<MathNode, ParseError> {
        let canon = alias(name);
        if let Some(e) = lookup(canon).or_else(|| lookup(name)) {
            match e.kind {
                CatalogKind::Container | CatalogKind::Modifier => {
                    return Err(ParseError::unsupported(command_span, format!("\\{name}")));
                }
                CatalogKind::Symbol | CatalogKind::Operator => {
                    return Ok(MathNode::Symbol(canon.to_string()));
                }
            }
        }
        if is_extra_symbol(canon) {
            return Ok(MathNode::Symbol(canon.to_string()));
        }
        Err(ParseError::unknown(command_span, format!("\\{name}")))
    }
}

fn strip_fences(tokens: &[SpannedToken]) -> Result<Vec<SpannedToken>, ParseError> {
    let t = trim_spaces(tokens);
    if t.len() >= 2 {
        let first = &t[0].token;
        let last = &t[t.len() - 1].token;
        let inner = match (first, last) {
            (Token::MathShift, Token::MathShift) => Some(&t[1..t.len() - 1]),
            (Token::DisplayShift, Token::DisplayShift) => Some(&t[1..t.len() - 1]),
            (Token::Command(a), Token::Command(b)) if a == "[" && b == "]" => {
                Some(&t[1..t.len() - 1])
            }
            (Token::Command(a), Token::Command(b)) if a == "(" && b == ")" => {
                Some(&t[1..t.len() - 1])
            }
            _ => None,
        };
        if let Some(inner) = inner {
            return Ok(trim_spaces(inner).to_vec());
        }
        if matches!(first, Token::MathShift | Token::DisplayShift)
            || matches!(first, Token::Command(s) if s == "[" || s == "(")
        {
            return Err(ParseError::unmatched_delimiter(t[0].span));
        }
    } else if let Some(first) = t.first() {
        if matches!(&first.token, Token::MathShift | Token::DisplayShift)
            || matches!(&first.token, Token::Command(s) if s == "[" || s == "(")
        {
            return Err(ParseError::unmatched_delimiter(first.span));
        }
    }
    Ok(t.to_vec())
}

fn trim_spaces(tokens: &[SpannedToken]) -> &[SpannedToken] {
    let mut a = 0;
    let mut b = tokens.len();
    while a < b && matches!(&tokens[a].token, Token::Space) {
        a += 1;
    }
    while b > a && matches!(&tokens[b - 1].token, Token::Space) {
        b -= 1;
    }
    &tokens[a..b]
}

fn enforce_ast_node_limit(
    root: &MathNode,
    max_nodes: usize,
    source_len: usize,
) -> Result<(), ParseError> {
    let span = SourceSpan::new(0, source_len);
    if max_nodes == 0 {
        return Err(ParseError::resource_limit(
            span,
            ParseResource::AstNodes,
            max_nodes,
        ));
    }

    let mut count = 0usize;
    let mut stack = Vec::with_capacity(max_nodes.min(64));
    stack.push(root);
    while let Some(node) = stack.pop() {
        count = count
            .checked_add(1)
            .ok_or_else(|| ParseError::resource_limit(span, ParseResource::AstNodes, max_nodes))?;
        if count > max_nodes {
            return Err(ParseError::resource_limit(
                span,
                ParseResource::AstNodes,
                max_nodes,
            ));
        }

        match node {
            MathNode::Atom(_, _)
            | MathNode::SizedDelim(_, _, _)
            | MathNode::Ref(_)
            | MathNode::Label(_)
            | MathNode::NoNumber
            | MathNode::Hline
            | MathNode::MathAlphabet(_, _)
            | MathNode::LiteralText(_)
            | MathNode::Space(_)
            | MathNode::Style(_)
            | MathNode::Operator(_, _)
            | MathNode::Symbol(_)
            | MathNode::Strut(_, _)
            | MathNode::Rule(_, _) => {}
            MathNode::Fraction(spec) => {
                push_ast_child(
                    &mut stack,
                    spec.denominator.as_ref(),
                    count,
                    max_nodes,
                    span,
                )?;
                push_ast_child(&mut stack, spec.numerator.as_ref(), count, max_nodes, span)?;
            }
            MathNode::Radical(index, body) => {
                push_ast_child(&mut stack, body.as_ref(), count, max_nodes, span)?;
                if let Some(index) = index {
                    push_ast_child(&mut stack, index.as_ref(), count, max_nodes, span)?;
                }
            }
            MathNode::Limits(body, _) => {
                push_ast_child(&mut stack, body.as_ref(), count, max_nodes, span)?;
            }
            MathNode::Superscript(base, script) | MathNode::Subscript(base, script) => {
                push_ast_child(&mut stack, script.as_ref(), count, max_nodes, span)?;
                push_ast_child(&mut stack, base.as_ref(), count, max_nodes, span)?;
            }
            MathNode::SubSup(base, sub, sup) => {
                push_ast_child(&mut stack, sup.as_ref(), count, max_nodes, span)?;
                push_ast_child(&mut stack, sub.as_ref(), count, max_nodes, span)?;
                push_ast_child(&mut stack, base.as_ref(), count, max_nodes, span)?;
            }
            MathNode::Delimited(_, body, _) => {
                push_ast_child(&mut stack, body.as_ref(), count, max_nodes, span)?;
            }
            MathNode::Row(items) | MathNode::Substack(items) => {
                for item in items.iter().rev() {
                    push_ast_child(&mut stack, item, count, max_nodes, span)?;
                }
            }
            MathNode::Matrix(_, _, rows) => {
                for row in rows.iter().rev() {
                    match row {
                        EnvRow::Cells { cells, number, .. } => {
                            if let EqNumber::Tag { body, .. } = number {
                                push_ast_child(&mut stack, body.as_ref(), count, max_nodes, span)?;
                            }
                            for cell in cells.iter().rev() {
                                push_ast_child(&mut stack, cell, count, max_nodes, span)?;
                            }
                        }
                        EnvRow::Intertext(body) => {
                            push_ast_child(&mut stack, body.as_ref(), count, max_nodes, span)?;
                        }
                        EnvRow::Hline => {}
                    }
                }
            }
            MathNode::Tag { body, .. }
            | MathNode::Intertext(body)
            | MathNode::Accent(body, _)
            | MathNode::Color(_, body)
            | MathNode::TextColor(_, body)
            | MathNode::ColorBox(_, body)
            | MathNode::FColorBox(_, _, body)
            | MathNode::Phantom(_, body) => {
                push_ast_child(&mut stack, body.as_ref(), count, max_nodes, span)?;
            }
            MathNode::Sum(lower, upper)
            | MathNode::Product(lower, upper)
            | MathNode::Integral(_, lower, upper) => {
                if let Some(upper) = upper {
                    push_ast_child(&mut stack, upper.as_ref(), count, max_nodes, span)?;
                }
                if let Some(lower) = lower {
                    push_ast_child(&mut stack, lower.as_ref(), count, max_nodes, span)?;
                }
            }
            MathNode::Limit(lower) => {
                if let Some(lower) = lower {
                    push_ast_child(&mut stack, lower.as_ref(), count, max_nodes, span)?;
                }
            }
            MathNode::OverUnder(base, over, under) => {
                if let Some(under) = under {
                    push_ast_child(&mut stack, under.as_ref(), count, max_nodes, span)?;
                }
                if let Some(over) = over {
                    push_ast_child(&mut stack, over.as_ref(), count, max_nodes, span)?;
                }
                push_ast_child(&mut stack, base.as_ref(), count, max_nodes, span)?;
            }
            MathNode::StackRel(base, over) => {
                push_ast_child(&mut stack, over.as_ref(), count, max_nodes, span)?;
                push_ast_child(&mut stack, base.as_ref(), count, max_nodes, span)?;
            }
            MathNode::CancelTo(value, expression) => {
                push_ast_child(&mut stack, expression.as_ref(), count, max_nodes, span)?;
                push_ast_child(&mut stack, value.as_ref(), count, max_nodes, span)?;
            }
        }
    }
    Ok(())
}

fn push_ast_child<'a>(
    stack: &mut Vec<&'a MathNode>,
    child: &'a MathNode,
    processed: usize,
    max_nodes: usize,
    span: SourceSpan,
) -> Result<(), ParseError> {
    let discovered = processed
        .checked_add(stack.len())
        .and_then(|n| n.checked_add(1))
        .ok_or_else(|| ParseError::resource_limit(span, ParseResource::AstNodes, max_nodes))?;
    if discovered > max_nodes {
        return Err(ParseError::resource_limit(
            span,
            ParseResource::AstNodes,
            max_nodes,
        ));
    }
    stack.push(child);
    Ok(())
}

fn wrap_row(mut items: Vec<MathNode>) -> MathNode {
    if items.len() == 1 {
        items.remove(0)
    } else {
        MathNode::Row(items)
    }
}

fn parse_colspec(s: &str, span: SourceSpan) -> Result<Vec<ColSpec>, ParseError> {
    let mut out = Vec::new();
    for c in s.chars() {
        match c {
            'l' => out.push(ColSpec::Left),
            'c' => out.push(ColSpec::Center),
            'r' => out.push(ColSpec::Right),
            '|' => out.push(ColSpec::VRule),
            ' ' | '\t' => {}
            '@' | '!' | '>' | '<' | 'p' | 'm' | 'b' | '*' => {
                return Err(ParseError::unsupported(
                    span,
                    format!("array preamble `{c}`"),
                ))
            }
            other => {
                return Err(ParseError::malformed(
                    ParseErrorKind::MalformedMatrix,
                    span,
                    format!("array preamble `{other}`"),
                ))
            }
        }
    }
    if out.is_empty() {
        return Err(ParseError::malformed(
            ParseErrorKind::MalformedMatrix,
            span,
            "empty array preamble".into(),
        ));
    }
    Ok(out)
}

fn peel_row_meta(node: MathNode, number: &mut EqNumber, labels: &mut Vec<String>) -> MathNode {
    match node {
        MathNode::NoNumber => {
            *number = EqNumber::Suppress;
            MathNode::Row(Vec::new())
        }
        MathNode::Tag { star, body } => {
            *number = EqNumber::Tag { star, body };
            MathNode::Row(Vec::new())
        }
        MathNode::Label(k) => {
            labels.push(k);
            MathNode::Row(Vec::new())
        }
        MathNode::Hline => MathNode::Hline,
        MathNode::Intertext(n) => MathNode::Intertext(n),
        MathNode::Row(items) => {
            let mut kept = Vec::new();
            for it in items {
                let p = peel_row_meta(it, number, labels);
                if !is_empty_node(&p) {
                    kept.push(p);
                }
            }
            wrap_row(kept)
        }
        other => other,
    }
}

fn finish_env_row(cells: Vec<MathNode>, number: EqNumber, labels: Vec<String>) -> EnvRow {
    if cells.len() == 1 && matches!(cells[0], MathNode::Hline) {
        return EnvRow::Hline;
    }
    if cells.len() == 1 {
        if let MathNode::Intertext(n) = &cells[0] {
            return EnvRow::Intertext(n.clone());
        }
    }
    EnvRow::Cells {
        cells,
        number,
        labels,
    }
}

fn is_empty_node(n: &MathNode) -> bool {
    match n {
        MathNode::Row(v) => v.is_empty() || v.iter().all(is_empty_node),
        MathNode::Space(_) | MathNode::NoNumber | MathNode::Label(_) => true,
        _ => false,
    }
}

fn accepts_limit_control(node: &MathNode) -> bool {
    matches!(
        node,
        MathNode::Operator(_, _)
            | MathNode::Sum(_, _)
            | MathNode::Product(_, _)
            | MathNode::Integral(_, _, _)
            | MathNode::Limit(_)
    )
}

fn apply_scripts(nucleus: MathNode, sub: Option<MathNode>, sup: Option<MathNode>) -> MathNode {
    match nucleus {
        MathNode::Sum(None, None) => MathNode::Sum(sub.map(Box::new), sup.map(Box::new)),
        MathNode::Product(None, None) => MathNode::Product(sub.map(Box::new), sup.map(Box::new)),
        MathNode::Integral(k, None, None) => {
            MathNode::Integral(k, sub.map(Box::new), sup.map(Box::new))
        }
        MathNode::Limit(None) => {
            let lim = MathNode::Limit(sub.map(Box::new));
            match sup {
                Some(s) => MathNode::Superscript(Box::new(lim), Box::new(s)),
                None => lim,
            }
        }
        MathNode::Accent(b, k @ (AccentKind::Overbrace | AccentKind::Underbrace)) => {
            match (sub, sup) {
                (None, None) => MathNode::Accent(b, k),
                (s, e) => MathNode::OverUnder(
                    Box::new(MathNode::Accent(b, k)),
                    e.map(Box::new),
                    s.map(Box::new),
                ),
            }
        }
        other => match (sub, sup) {
            (None, None) => other,
            (Some(s), None) => MathNode::Subscript(Box::new(other), Box::new(s)),
            (None, Some(e)) => MathNode::Superscript(Box::new(other), Box::new(e)),
            (Some(s), Some(e)) => MathNode::SubSup(Box::new(other), Box::new(s), Box::new(e)),
        },
    }
}

/// Apply a math-alphabet style to stylable mathematical characters.
///
/// A math-alphabet command such as `\mathrm` descends through mathematical
/// structure, but literal text is a separate semantic construct and is never
/// restyled by the surrounding math alphabet. Delimiters are `Delimiter` values
/// rather than nodes and are left unchanged.
fn apply_math_alphabet(node: MathNode, style: TextStyle) -> MathNode {
    match node {
        MathNode::Atom(c, _) if crate::style_map::is_stylable(c) => {
            MathNode::MathAlphabet(c.to_string(), style)
        }
        MathNode::MathAlphabet(s, _) => MathNode::MathAlphabet(s, style),
        MathNode::LiteralText(s) => MathNode::LiteralText(s),
        MathNode::Symbol(name) => {
            if let Some(ch) = crate::symbols::glyph_char(&name) {
                if crate::style_map::is_stylable(ch) {
                    MathNode::MathAlphabet(ch.to_string(), style)
                } else {
                    MathNode::Symbol(name)
                }
            } else {
                MathNode::Symbol(name)
            }
        }
        MathNode::Row(v) => collapse_runs(MathNode::Row(
            v.into_iter()
                .map(|n| apply_math_alphabet(n, style))
                .collect(),
        )),
        MathNode::Substack(v) => MathNode::Substack(
            v.into_iter()
                .map(|n| apply_math_alphabet(n, style))
                .collect(),
        ),
        MathNode::Superscript(b, sup) => MathNode::Superscript(
            Box::new(apply_math_alphabet(*b, style)),
            Box::new(apply_math_alphabet(*sup, style)),
        ),
        MathNode::Subscript(b, sub) => MathNode::Subscript(
            Box::new(apply_math_alphabet(*b, style)),
            Box::new(apply_math_alphabet(*sub, style)),
        ),
        MathNode::SubSup(b, sub, sup) => MathNode::SubSup(
            Box::new(apply_math_alphabet(*b, style)),
            Box::new(apply_math_alphabet(*sub, style)),
            Box::new(apply_math_alphabet(*sup, style)),
        ),
        MathNode::Fraction(mut spec) => {
            spec.numerator = Box::new(apply_math_alphabet(*spec.numerator, style));
            spec.denominator = Box::new(apply_math_alphabet(*spec.denominator, style));
            MathNode::Fraction(spec)
        }
        MathNode::Radical(index, body) => MathNode::Radical(
            index.map(|i| Box::new(apply_math_alphabet(*i, style))),
            Box::new(apply_math_alphabet(*body, style)),
        ),
        MathNode::Limits(body, mode) => {
            MathNode::Limits(Box::new(apply_math_alphabet(*body, style)), mode)
        }
        MathNode::Accent(body, kind) => {
            MathNode::Accent(Box::new(apply_math_alphabet(*body, style)), kind)
        }
        MathNode::Delimited(open, body, close) => {
            MathNode::Delimited(open, Box::new(apply_math_alphabet(*body, style)), close)
        }
        other => other,
    }
}

fn collapse_runs(node: MathNode) -> MathNode {
    let MathNode::Row(v) = node else {
        return node;
    };
    let mut out: Vec<MathNode> = Vec::new();
    for n in v {
        match (out.last_mut(), &n) {
            (Some(MathNode::MathAlphabet(a, sa)), MathNode::MathAlphabet(b, sb)) if sa == sb => {
                a.push_str(b);
            }
            (Some(MathNode::LiteralText(a)), MathNode::LiteralText(b)) => a.push_str(b),
            _ => out.push(n),
        }
    }
    wrap_row(out)
}

fn plain_tex_math_alphabet(name: &str) -> Option<TextStyle> {
    match name {
        "rm" => Some(TextStyle::Rm),
        "bf" => Some(TextStyle::Bf),
        "cal" => Some(TextStyle::Cal),
        "it" => Some(TextStyle::It),
        "sf" => Some(TextStyle::Sf),
        "tt" => Some(TextStyle::Tt),
        _ => None,
    }
}

fn length_value(length: &Length) -> &Dim {
    match length {
        Length::Em(value) | Length::Mu(value) | Length::TexPt(value) | Length::BigPt(value) => {
            value
        }
    }
}

fn atom_kind(c: char) -> AtomKind {
    match c {
        '+' | '-' | '*' | '±' | '∓' | '·' | '×' | '÷' => AtomKind::Bin,
        '=' | '<' | '>' | '≠' | '≤' | '≥' | '≈' | '≡' => AtomKind::Rel,
        '(' | '[' | '{' => AtomKind::Open,
        ')' | ']' | '}' => AtomKind::Close,
        ',' | ';' | '!' | '?' | ':' => AtomKind::Punct,
        _ => AtomKind::Ord,
    }
}

fn delim_from_text(s: &str) -> Result<Delimiter, ParseError> {
    let s = s.trim();
    if s.is_empty() || s == "." {
        return Ok(Delimiter::Empty);
    }
    if s.chars().count() == 1 {
        let c = s.chars().next().unwrap();
        return Ok(Delimiter::Char(c));
    }
    let name = s.strip_prefix('\\').unwrap_or(s);
    Ok(Delimiter::Named(name.to_string()))
}

fn parse_tex_dim(s: &str, span: SourceSpan) -> Result<Length, ParseError> {
    let s = s.trim();
    if s.is_empty() {
        return Err(ParseError::malformed(
            ParseErrorKind::MalformedDimension,
            span,
            "empty dimension".into(),
        ));
    }
    let mut i = 0;
    let b = s.as_bytes();
    if i < b.len() && (b[i] == b'+' || b[i] == b'-') {
        i += 1;
    }
    while i < b.len() && (b[i].is_ascii_digit() || b[i] == b'.') {
        i += 1;
    }
    if i == 0 || (i == 1 && (b[0] == b'+' || b[0] == b'-')) {
        return Err(ParseError::malformed(
            ParseErrorKind::MalformedDimension,
            span,
            format!("invalid dimension `{s}`"),
        ));
    }
    let num = Dim::parse(&s[..i]).map_err(|error| numeric_parse_err(error, span))?;
    let unit = s[i..].trim();
    match unit {
        "" | "em" => Ok(Length::Em(num)),
        "mu" => Ok(Length::Mu(num)),
        "pt" => Ok(Length::TexPt(num)),
        "bp" => Ok(Length::BigPt(num)),
        other => Err(ParseError::malformed(
            ParseErrorKind::MalformedDimension,
            span,
            format!("unsupported dimension unit {other}"),
        )),
    }
}

fn numeric_parse_err(error: crate::NumericError, span: SourceSpan) -> ParseError {
    ParseError::malformed(ParseErrorKind::MalformedDimension, span, error.to_string())
}

fn color_err(error: Error, span: SourceSpan) -> ParseError {
    match error {
        Error::Unsupported { what } => ParseError::unsupported(span, what),
        Error::Parse(parse) => parse,
        Error::Malformed { what } | Error::InvalidOption { what } => {
            ParseError::malformed(ParseErrorKind::MalformedArgument, span, what)
        }
        other => ParseError::malformed(ParseErrorKind::MalformedArgument, span, other.to_string()),
    }
}

fn alias(name: &str) -> &str {
    match name {
        "le" => "leq",
        "ge" => "geq",
        "ne" => "neq",
        "dots" => "ldots",
        "lnot" => "neg",
        "dag" => "dagger",
        "ddag" => "ddagger",
        "owns" => "ni",
        _ => name,
    }
}

fn is_extra_symbol(name: &str) -> bool {
    matches!(
        name,
        "Gamma"
            | "Delta"
            | "Theta"
            | "Lambda"
            | "Xi"
            | "Pi"
            | "Sigma"
            | "Upsilon"
            | "Phi"
            | "Psi"
            | "Omega"
            | "varepsilon"
            | "vartheta"
            | "varpi"
            | "varrho"
            | "varsigma"
            | "varphi"
            | "ldots"
            | "cdots"
            | "vdots"
            | "ddots"
            | "colon"
            | "mid"
            | "lvert"
            | "rvert"
            | "lVert"
            | "rVert"
            | "vert"
            | "Vert"
            | "implies"
            | "iff"
            | "to"
            | "gets"
            | "neq"
            | "leq"
            | "geq"
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn frac_gold() {
        let n = parse(r"\frac{1}{2}").unwrap();
        assert_eq!(n.gold(), r#"(frac (atom Ord "1") (atom Ord "2"))"#);
    }

    #[test]
    fn oversized_dimension_is_rejected_instead_of_becoming_invalid_dim() {
        let err = parse(r"\hspace{170141183460469231731687303715884105728em}")
            .expect_err("2^127 em must exceed the Dim contract");
        assert_eq!(err.kind(), ParseErrorKind::MalformedDimension);
    }

    #[test]
    fn style_declarations_and_limit_controls_survive_parsing() {
        assert_eq!(
            parse(r"\scriptstyle x").unwrap().gold(),
            r#"(row (style script) (atom Ord "x"))"#
        );
        assert_eq!(
            parse(r"\sum\limits_{i}^{n}").unwrap().gold(),
            r#"(subsup (limits (sum _ _)) (atom Ord "i") (atom Ord "n"))"#
        );
        assert_eq!(
            parse(r"\int\nolimits_0^1").unwrap().gold(),
            r#"(subsup (nolimits (int _ _)) (atom Ord "0") (atom Ord "1"))"#
        );
    }

    #[test]
    fn limit_control_requires_an_operator_nucleus() {
        let err = parse(r"x\limits_1").expect_err("limits on an ordinary atom must fail");
        assert_eq!(err.kind(), ParseErrorKind::MalformedArgument);
    }
}
