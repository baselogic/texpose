//! Public error types. Unsupported input is never coerced into success.

use core::fmt;

/// Exact-dimension construction or arithmetic failure.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NumericError {
    /// Decimal syntax is malformed.
    InvalidDecimal,
    /// A rational denominator or divisor is zero.
    ZeroDenominator,
    /// The canonical result of exact arithmetic is outside the `Dim` range.
    ArithmeticOverflow,
    /// Converting external numeric text or data overflowed its staging type.
    ConversionOverflow,
    /// A mathematically valid value is outside the supported `Dim` contract.
    OutOfRange,
}

/// Byte range in the original math source, half-open as `[start, end)`.
///
/// Spans are created by TeXpose and are read-only consumer values. Use the
/// accessors rather than depending on the representation.
///
/// ```compile_fail
/// let error = texpose::parse("{").unwrap_err();
/// let span = error.span();
/// let _ = span.start;
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SourceSpan {
    start: usize,
    end: usize,
}

impl SourceSpan {
    pub(crate) const fn new(start: usize, end: usize) -> Self {
        Self { start, end }
    }

    pub(crate) const fn point(offset: usize) -> Self {
        Self {
            start: offset,
            end: offset,
        }
    }

    /// Inclusive starting byte offset.
    #[must_use]
    pub const fn start(self) -> usize {
        self.start
    }

    /// Exclusive ending byte offset.
    #[must_use]
    pub const fn end(self) -> usize {
        self.end
    }

    /// Length of the covered byte range.
    #[must_use]
    pub const fn len(self) -> usize {
        self.end - self.start
    }

    /// True when the span is a point between bytes.
    #[must_use]
    pub const fn is_empty(self) -> bool {
        self.start == self.end
    }
}

/// Parser resource controlled by [`crate::ParseOptions`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum ParseResource {
    /// Recursive parser nesting.
    NestingDepth,
    /// Number of nodes in the returned syntax tree.
    AstNodes,
    /// Total rows across parsed environments.
    EnvironmentRows,
    /// Total cells across parsed environments.
    EnvironmentCells,
    /// Number of lexical tokens produced from the source.
    Tokens,
}

impl fmt::Display for ParseResource {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NestingDepth => f.write_str("nesting depth"),
            Self::AstNodes => f.write_str("AST node count"),
            Self::EnvironmentRows => f.write_str("environment row count"),
            Self::EnvironmentCells => f.write_str("environment cell count"),
            Self::Tokens => f.write_str("token count"),
        }
    }
}

/// Typed parser failure category.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum ParseErrorKind {
    /// Input ended with a stray `\` and no command character.
    TrailingBackslash,
    /// Command is not in the supported command catalog.
    UnknownCommand,
    /// Command or construct is known but unsupported.
    UnsupportedCommand,
    /// A `{` group was not closed.
    UnclosedGroup,
    /// A `}` occurred where no group end was expected.
    UnexpectedGroupEnd,
    /// `\left` / `\right` pairing is invalid.
    UnmatchedDelimiter,
    /// `\begin{...}` and `\end{...}` names do not match.
    MismatchedEnvironment,
    /// A construct argument is syntactically malformed or missing.
    MalformedArgument,
    /// A dimension is malformed or uses an unsupported unit.
    MalformedDimension,
    /// A matrix/environment body or preamble is malformed.
    MalformedMatrix,
    /// A configured parser resource budget was exceeded.
    ResourceLimit,
}

/// Structured parser-error details.
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum ParseErrorDetail {
    /// No additional detail is required.
    None,
    /// Command or construct name involved in the failure.
    Command(String),
    /// Human-readable construct-specific diagnostic context.
    ///
    /// This text is not a machine-stable discriminator; callers should branch on
    /// [`ParseErrorKind`] and typed detail variants instead.
    Message(String),
    /// Environment closing mismatch.
    Environment {
        /// Name from `\begin{...}`.
        expected: String,
        /// Name from `\end{...}`.
        found: String,
    },
    /// Configured resource budget that was exceeded.
    ResourceLimit {
        /// Resource whose counter would exceed the limit.
        resource: ParseResource,
        /// Configured maximum accepted value.
        limit: usize,
    },
}

/// Tokenizer / parser failure with an original-source byte range.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ParseError {
    kind: ParseErrorKind,
    span: SourceSpan,
    detail: ParseErrorDetail,
}

impl ParseError {
    /// Typed failure category.
    #[must_use]
    pub const fn kind(&self) -> ParseErrorKind {
        self.kind
    }

    /// Byte range in the original input associated with the failure.
    #[must_use]
    pub const fn span(&self) -> SourceSpan {
        self.span
    }

    /// Construct-specific typed information for diagnostics and callers.
    ///
    /// Free-form [`ParseErrorDetail::Message`] text is human-facing and is not a
    /// compatibility discriminator.
    #[must_use]
    pub const fn detail(&self) -> &ParseErrorDetail {
        &self.detail
    }

    pub(crate) const fn trailing_backslash(span: SourceSpan) -> Self {
        Self {
            kind: ParseErrorKind::TrailingBackslash,
            span,
            detail: ParseErrorDetail::None,
        }
    }

    pub(crate) fn unknown(span: SourceSpan, command: String) -> Self {
        Self {
            kind: ParseErrorKind::UnknownCommand,
            span,
            detail: ParseErrorDetail::Command(command),
        }
    }

    pub(crate) fn unsupported(span: SourceSpan, what: String) -> Self {
        Self {
            kind: ParseErrorKind::UnsupportedCommand,
            span,
            detail: ParseErrorDetail::Message(what),
        }
    }

    pub(crate) fn malformed(kind: ParseErrorKind, span: SourceSpan, what: String) -> Self {
        Self {
            kind,
            span,
            detail: ParseErrorDetail::Message(what),
        }
    }

    pub(crate) const fn unmatched_delimiter(span: SourceSpan) -> Self {
        Self {
            kind: ParseErrorKind::UnmatchedDelimiter,
            span,
            detail: ParseErrorDetail::None,
        }
    }

    pub(crate) fn mismatched_environment(
        span: SourceSpan,
        expected: String,
        found: String,
    ) -> Self {
        Self {
            kind: ParseErrorKind::MismatchedEnvironment,
            span,
            detail: ParseErrorDetail::Environment { expected, found },
        }
    }

    pub(crate) const fn resource_limit(
        span: SourceSpan,
        resource: ParseResource,
        limit: usize,
    ) -> Self {
        Self {
            kind: ParseErrorKind::ResourceLimit,
            span,
            detail: ParseErrorDetail::ResourceLimit { resource, limit },
        }
    }

    fn message(&self) -> Option<&str> {
        match &self.detail {
            ParseErrorDetail::Command(s) | ParseErrorDetail::Message(s) => Some(s),
            _ => None,
        }
    }
}

/// Crate-level error.
///
/// [`Error::Parse`] wraps tokenizer/parser failures; [`Error::Font`] wraps
/// face/glyph failures; [`Error::Numeric`] wraps exact-dimension failures;
/// [`Error::Unsupported`] is a construct or feature out of
/// scope; [`Error::Malformed`] is a bad value that parsed as the wrong shape;
/// [`Error::InvalidOption`] is a core option outside its accepted range.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Error {
    /// Tokenizer or parser rejected the input.
    Parse(ParseError),
    /// Font bytes or a requested glyph could not be used.
    Font(FontError),
    /// Exact-dimension construction or arithmetic failed.
    Numeric(NumericError),
    /// A feature listed as out of scope, or not yet implemented.
    ///
    /// Callers must treat this as failure. The core does not invent semantics.
    Unsupported {
        /// Human-readable name of the missing feature or construct.
        what: String,
    },
    /// Syntactically invalid value (for example a color spec with the wrong
    /// number of components). Distinct from [`Self::Unsupported`].
    Malformed {
        /// Human-readable description of what was malformed.
        what: String,
    },
    /// A core option is outside its accepted range.
    InvalidOption {
        /// Human-readable name of the invalid option.
        what: String,
    },
}

/// Font loader or metric lookup failure.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FontError {
    /// Supplied bytes are not a usable OpenType face.
    InvalidFace,
    /// A font collection was supplied without selecting an explicit face index.
    CollectionFaceIndexRequired,
    /// The requested face index does not exist in the supplied font or collection.
    FaceIndexOutOfBounds,
    /// The selected face contains functional OpenType variation axes.
    VariableFontUnsupported,
    /// The selected face has no physical OpenType MATH table.
    MissingMathTable,
    /// A physical MATH table exists but its header or table range is malformed.
    MalformedMathTable,
    /// The MATH header does not reference a MathConstants table.
    MissingMathConstants,
    /// The referenced MathConstants table is truncated or otherwise malformed.
    MalformedMathConstants,
    /// Character has no glyph in this face for a direct or strict glyph lookup.
    MissingGlyph {
        /// Requested character.
        ch: char,
    },
}

impl fmt::Display for NumericError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidDecimal => f.write_str("invalid decimal"),
            Self::ZeroDenominator => f.write_str("zero denominator"),
            Self::ArithmeticOverflow => f.write_str("dimension arithmetic overflow"),
            Self::ConversionOverflow => f.write_str("numeric conversion overflow"),
            Self::OutOfRange => f.write_str("dimension out of supported range"),
        }
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Parse(e) => write!(f, "{e}"),
            Self::Font(e) => write!(f, "{e}"),
            Self::Numeric(e) => write!(f, "{e}"),
            Self::Unsupported { what } => write!(f, "unsupported: {what}"),
            Self::Malformed { what } => write!(f, "malformed: {what}"),
            Self::InvalidOption { what } => write!(f, "invalid option: {what}"),
        }
    }
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.kind {
            ParseErrorKind::TrailingBackslash => f.write_str("trailing backslash"),
            ParseErrorKind::UnknownCommand => {
                write!(
                    f,
                    "unknown command: {}",
                    self.message().unwrap_or("<unknown>")
                )
            }
            ParseErrorKind::UnsupportedCommand => {
                write!(f, "unsupported: {}", self.message().unwrap_or("<unknown>"))
            }
            ParseErrorKind::UnclosedGroup
            | ParseErrorKind::UnexpectedGroupEnd
            | ParseErrorKind::MalformedArgument
            | ParseErrorKind::MalformedDimension
            | ParseErrorKind::MalformedMatrix => {
                write!(
                    f,
                    "malformed: {}",
                    self.message().unwrap_or("invalid input")
                )
            }
            ParseErrorKind::UnmatchedDelimiter => f.write_str("unmatched delimiter"),
            ParseErrorKind::MismatchedEnvironment => match &self.detail {
                ParseErrorDetail::Environment { expected, found } => {
                    write!(
                        f,
                        "malformed: \\begin{{{expected}}} closed by \\end{{{found}}}"
                    )
                }
                _ => f.write_str("malformed: mismatched environment"),
            },
            ParseErrorKind::ResourceLimit => match &self.detail {
                ParseErrorDetail::ResourceLimit { resource, limit } => {
                    write!(f, "resource limit: {resource} exceeds {limit}")
                }
                _ => f.write_str("resource limit exceeded"),
            },
        }
    }
}

impl fmt::Display for FontError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidFace => f.write_str("invalid OpenType face"),
            Self::CollectionFaceIndexRequired => {
                f.write_str("font collection requires an explicit face index")
            }
            Self::FaceIndexOutOfBounds => f.write_str("font face index is out of bounds"),
            Self::VariableFontUnsupported => f.write_str("variable fonts are unsupported"),
            Self::MissingMathTable => f.write_str("font has no OpenType MATH table"),
            Self::MalformedMathTable => f.write_str("OpenType MATH table is malformed"),
            Self::MissingMathConstants => f.write_str("MATH table has no MathConstants offset"),
            Self::MalformedMathConstants => f.write_str("MATH MathConstants table is malformed"),
            Self::MissingGlyph { ch } => write!(f, "missing glyph for {ch:?}"),
        }
    }
}

impl std::error::Error for NumericError {}

impl std::error::Error for ParseError {}

impl std::error::Error for FontError {}

impl std::error::Error for Error {}

impl From<ParseError> for Error {
    fn from(e: ParseError) -> Self {
        Self::Parse(e)
    }
}

impl From<FontError> for Error {
    fn from(e: FontError) -> Self {
        Self::Font(e)
    }
}

impl From<NumericError> for Error {
    fn from(e: NumericError) -> Self {
        Self::Numeric(e)
    }
}
