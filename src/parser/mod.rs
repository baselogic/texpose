//! LaTeX math → [`MathNode`] AST.
//!
//! Pipeline: [`preprocess`] → [`tokenize`] → typed parse. Unknown or unsupported
//! input is [`ParseError`](crate::error::ParseError), never a silent partial tree.

mod ast;
mod parse;
mod preproc;
mod token;

pub use ast::{
    AccentKind, AtomKind, ColSpec, DelimSize, Delimiter, EnvRow, EqNumber, FractionAlignment,
    FractionRule, FractionSpec, FractionStyle, IntegralKind, Length, LimitMode, MathNode,
    MathStyleDeclaration, MatrixStyle, PhantomKind, SpaceKind, TextStyle,
};
pub use parse::{
    parse, parse_with_colors, parse_with_options, ParseOptions, DEFAULT_MAX_AST_NODES,
    DEFAULT_MAX_ENVIRONMENT_CELLS, DEFAULT_MAX_ENVIRONMENT_ROWS, DEFAULT_MAX_NESTING_DEPTH,
    DEFAULT_MAX_TOKENS,
};
pub use preproc::preprocess;
pub use token::{format_tokens, tokenize, tokenize_spanned, SpannedToken, Token};
