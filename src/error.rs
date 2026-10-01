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

/// Tokenizer / parser failure.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ParseError {
    /// Input ended with a stray `\` and no command character.
    TrailingBackslash,
    /// Command is known but this crate does not support its semantics.
    Unsupported(String),
    /// Command is not in the catalog and is not a known math structure.
    Unknown(String),
    /// Syntactically invalid input. Names the construct or position.
    Malformed(String),
    /// `\left` without `\right`, or `\right` without `\left`.
    UnmatchedDelimiter,
}

/// Font loader or metric lookup failure.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FontError {
    /// Embedded or supplied bytes are not a usable OpenType face.
    InvalidFace,
    /// Character has no glyph in this face.
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
        match self {
            Self::TrailingBackslash => f.write_str("trailing backslash"),
            Self::Unsupported(s) => write!(f, "unsupported: {s}"),
            Self::Unknown(s) => write!(f, "unknown command: {s}"),
            Self::Malformed(s) => write!(f, "malformed: {s}"),
            Self::UnmatchedDelimiter => f.write_str("unmatched delimiter"),
        }
    }
}

impl fmt::Display for FontError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidFace => f.write_str("invalid OpenType face"),
            Self::MissingGlyph { ch } => write!(f, "missing glyph for {ch:?}"),
        }
    }
}

impl std::error::Error for NumericError {}

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
