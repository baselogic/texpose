//! LaTeX math → [`MathNode`] AST.
//!
//! The exact source is tokenized directly and parsed into typed syntax. Unknown or
//! unsupported input is [`ParseError`](crate::ParseError), never a silent partial tree.

mod ast;
mod parse;
mod token;

pub use ast::{
    AccentKind, AtomKind, ColSpec, DelimSize, Delimiter, EnvRow, EqNumber, FractionAlignment,
    FractionRule, FractionSpec, FractionStyle, IntegralKind, Length, LimitMode, MathNode,
    MathStyleDeclaration, MatrixStyle, PhantomKind, SpaceKind, TextStyle,
};
pub use parse::{
    parse, parse_with_options, ParseOptions, DEFAULT_MAX_AST_NODES, DEFAULT_MAX_ENVIRONMENT_CELLS,
    DEFAULT_MAX_ENVIRONMENT_ROWS, DEFAULT_MAX_NESTING_DEPTH, DEFAULT_MAX_TOKENS,
};
#[cfg(test)]
pub(crate) use token::{format_tokens, tokenize};

#[cfg(test)]
mod tests;
