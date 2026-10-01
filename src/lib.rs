//! TeXpose: portable Rust infrastructure for TeX/LaTeX mathematical notation and layout.
//!
//! The current codebase is an independent hard fork of LaTeX-Rust 2.0.1 and is
//! being reduced and reworked around backend-neutral parsing, font metrics, and
//! layout. Public API stability is not yet promised.
//!
//! Unsupported constructs return [`Error`] rather than fabricating output.

#![forbid(unsafe_code)]
#![deny(dead_code)]
#![deny(missing_docs)]
#![deny(clippy::float_arithmetic)]
#![deny(clippy::undocumented_unsafe_blocks)]
#![cfg_attr(docsrs, feature(doc_cfg))]

mod atoms;
mod color;
mod dim;
mod error;
mod hash;
mod style_map;
mod symbols;

/// OpenType MATH metrics and the embedded STIX Two Math face.
pub mod font;
/// AST → TeX-faithful [`MathBox`](layout::MathBox).
pub mod layout;
/// LaTeX math → [`MathNode`](parser::MathNode) AST.
pub mod parser;
/// The OpenType parser this crate uses, re-exported so that consumers of
/// [`MathFont::face`] name the same version.
pub use ttf_parser;

pub use atoms::symbol_atom_kind;
pub use color::{named_color, parse_color_spec, Color, ColorTable};
pub use dim::{Dim, DIM_PREC};
pub use error::{Error, FontError, NumericError, ParseError};
pub use font::{
    GlyphMetrics, MathFont, STIX_TWO_MATH_NAME, STIX_TWO_MATH_OTF, STIX_TWO_MATH_SHA256,
};
pub use layout::{
    layout, layout_with_em_size_pt, layout_with_max_depth, layout_with_numbering,
    layout_with_numbering_and_em_size_pt, BoxContent, MathBox, MathParams, MathStyle, NumberFormat,
    NumberStyle, NumberingConfig, NumberingState,
};
pub use parser::{
    format_tokens, parse, parse_with_colors, parse_with_options, preprocess, tokenize, AccentKind,
    AtomKind, ColSpec, DelimSize, Delimiter, EnvRow, EqNumber, FractionAlignment, FractionRule,
    FractionSpec, FractionStyle, IntegralKind, Length, MathNode, MatrixStyle, ParseOptions,
    PhantomKind, SpaceKind, TextStyle, Token, DEFAULT_MAX_NESTING_DEPTH,
};
pub use style_map::styled_char;
pub use symbols::{category_count, glyph_char, lookup, symbols, SymbolEntry, SymbolKind};
