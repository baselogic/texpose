//! TeXpose: portable Rust infrastructure for TeX/LaTeX mathematical notation and layout.
//!
//! The current codebase is an independent hard fork of LaTeX-Rust 2.0.1 and is
//! being reduced and reworked around backend-neutral parsing, font metrics, and
//! layout. Parser, font, layout, and first-stable compatibility boundaries are
//! stabilized; release-candidate verification remains separate work.
//!
//! Unsupported constructs return [`Error`] rather than fabricating output. Missing cmap entries are recoverable layout diagnostics with deterministic degradation.
//!
//! Parser consumers use crate-root items rather than the private source module:
//!
//! ```compile_fail
//! use texpose::parser::parse;
//! ```
//!
//! Former lexer/preprocessor helpers are intentionally not consumer API:
//!
//! ```compile_fail
//! use texpose::tokenize_spanned;
//! ```
//!
//! ```compile_fail
//! use texpose::parse_with_colors;
//! ```
//!
//! ```compile_fail
//! use texpose::preprocess;
//! ```
//!
//! ```
//! let ast = texpose::parse(r"\frac{1}{2}").unwrap();
//! assert!(matches!(ast, texpose::MathNode::Fraction(_)));
//! ```
//!
//! Font consumers use the crate-root [`MathFont`] handle. Font internals and the
//! concrete OpenType parser are deliberately not part of the public API:
//!
//! ```compile_fail
//! use texpose::font::MathFont;
//! ```
//!
//! ```compile_fail
//! use texpose::ttf_parser;
//! ```
//!
//! ```compile_fail
//! use texpose::GlyphMetrics;
//! ```
//!
//! Raw parser and intermediate font-query views are not methods on the public
//! handle:
//!
//! ```compile_fail
//! fn raw_face(font: &texpose::MathFont) {
//!     let _ = font.face();
//! }
//! ```
//!
//! ```compile_fail
//! fn raw_metrics(font: &texpose::MathFont) {
//!     let _ = font.glyph('x');
//! }
//! ```
//!
//! ```compile_fail
//! fn raw_variants(font: &texpose::MathFont) {
//!     let _ = font.horizontal_variants(0);
//! }
//! ```
//!
//! A native renderer obtains the exact bytes, face index, and units-per-em from
//! `MathLayout::font()` and may parse those bytes with its own OpenType stack.
//!
//! Layout values are producer-owned. Consumers inspect display-list operations,
//! diagnostics, colors, and numbering configuration without constructing internal
//! invariant-bearing values directly:
//!
//! ```compile_fail
//! let _ = texpose::Color { r: 1, g: 2, b: 3 };
//! ```
//!
//! ```compile_fail
//! let _ = texpose::MathOp::Rule {
//!     x: 0.0,
//!     y: 0.0,
//!     width: 1.0,
//!     height: 1.0,
//!     color: texpose::Color::rgb(0, 0, 0),
//! };
//! ```
//!
//! ```compile_fail
//! let _ = texpose::LayoutDiagnostic::MissingGlyph { ch: 'x' };
//! ```
//!
//! ```compile_fail
//! let _ = texpose::NumberingConfig {
//!     style: texpose::NumberStyle::Arabic,
//!     start: 1,
//!     format: texpose::NumberFormat::Parenthesized,
//! };
//! ```
//!
//! TeX style transitions are layout-engine mechanics rather than public helpers:
//!
//! ```compile_fail
//! let _ = texpose::MathStyle::Display.cramp();
//! ```

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
#[cfg(test)]
#[path = "../tests/support/hash.rs"]
mod hash;
mod style_map;
mod symbols;

mod font;
/// AST → backend-neutral [`MathLayout`](layout::MathLayout).
pub mod layout;
mod parser;
pub use atoms::symbol_atom_kind;
pub use color::{named_color, parse_color_spec, Color, ColorTable};
pub use dim::{Dim, DIM_PREC};
pub use error::{
    Error, FontError, NumericError, ParseError, ParseErrorDetail, ParseErrorKind, ParseResource,
    SourceSpan,
};
pub use font::MathFont;
pub use layout::{
    layout, layout_with_em_size_pt, layout_with_max_depth, layout_with_numbering,
    layout_with_numbering_and_em_size_pt, LayoutDiagnostic, MathLayout, MathOp, MathParams,
    MathStyle, NumberFormat, NumberStyle, NumberingConfig, NumberingState,
};
pub use parser::{
    parse, parse_with_options, AccentKind, AtomKind, ColSpec, DelimSize, Delimiter, EnvRow,
    EqNumber, FractionAlignment, FractionRule, FractionSpec, FractionStyle, IntegralKind, Length,
    LimitMode, MathNode, MathStyleDeclaration, MatrixStyle, ParseOptions, PhantomKind, SpaceKind,
    TextStyle, DEFAULT_MAX_AST_NODES, DEFAULT_MAX_ENVIRONMENT_CELLS, DEFAULT_MAX_ENVIRONMENT_ROWS,
    DEFAULT_MAX_NESTING_DEPTH, DEFAULT_MAX_TOKENS,
};
pub use style_map::styled_char;
pub use symbols::{category_count, glyph_char, lookup, symbols, SymbolEntry, SymbolKind};

#[cfg(test)]
extern crate self as texpose;

#[cfg(test)]
#[path = "../tests/support/golds.rs"]
pub(crate) mod gold_support;

#[cfg(test)]
pub(crate) mod test_support {
    pub(crate) use crate::layout::engine::{
        layout, layout_with_diagnostics, layout_with_em_size_pt,
        layout_with_em_size_pt_and_diagnostics, layout_with_max_depth, layout_with_numbering,
    };
    pub(crate) use crate::layout::{BoxContent, LayoutOutput, MathBox};
    pub(crate) use crate::parser::{format_tokens, tokenize};
    pub(crate) use crate::{
        category_count, lookup, named_color, parse, parse_color_spec, parse_with_options,
        styled_char, symbol_atom_kind, symbols, AccentKind, AtomKind, Color, ColorTable, Dim,
        Error, FontError, FractionAlignment, FractionRule, FractionSpec, FractionStyle,
        LayoutDiagnostic, Length, MathFont, MathNode, MathParams, MathStyle, NumberFormat,
        NumberStyle, NumberingConfig, NumberingState, ParseError, ParseErrorKind, ParseOptions,
        SpaceKind, SymbolKind, TextStyle, DEFAULT_MAX_AST_NODES, DEFAULT_MAX_ENVIRONMENT_CELLS,
        DEFAULT_MAX_ENVIRONMENT_ROWS, DEFAULT_MAX_NESTING_DEPTH, DEFAULT_MAX_TOKENS,
    };
}
