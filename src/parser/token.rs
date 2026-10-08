//! LaTeX math tokenizer.

use core::fmt;
use core::iter::Peekable;
use core::str::CharIndices;

use crate::error::{ParseError, ParseResource, SourceSpan};

/// A single TeX-style math token.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Token {
    /// Ordinary character (letter, digit, or other).
    Char(char),
    /// Control sequence without the leading backslash (`frac`, `[`, `,`).
    Command(String),
    /// `{`
    BeginGroup,
    /// `}`
    EndGroup,
    /// `^`
    Superscript,
    /// `_`
    Subscript,
    /// `&`
    AlignmentTab,
    /// Single `$`
    MathShift,
    /// `$$`
    DisplayShift,
    /// A space character (kept for `\text`; skipped in math lists).
    Space,
}

/// A token paired with its byte range in the original source.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct SpannedToken {
    pub(crate) token: Token,
    pub(crate) span: SourceSpan,
}

impl fmt::Display for Token {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Char(c) => write!(f, "char:{c}"),
            Self::Command(name) => write!(f, "cmd:{name}"),
            Self::BeginGroup => f.write_str("{"),
            Self::EndGroup => f.write_str("}"),
            Self::Superscript => f.write_str("^"),
            Self::Subscript => f.write_str("_"),
            Self::AlignmentTab => f.write_str("&"),
            Self::MathShift => f.write_str("$"),
            Self::DisplayShift => f.write_str("$$"),
            Self::Space => f.write_str("space"),
        }
    }
}

#[cfg(test)]
pub(crate) fn format_tokens(tokens: &[Token]) -> String {
    let mut out = String::new();
    for (i, token) in tokens.iter().enumerate() {
        if i > 0 {
            out.push(' ');
        }
        out.push_str(&token.to_string());
    }
    out
}

#[cfg(test)]
pub(crate) fn tokenize(input: &str) -> Result<Vec<Token>, ParseError> {
    Ok(tokenize_spanned(input)?
        .into_iter()
        .map(|item| item.token)
        .collect())
}

/// Tokenize while retaining exact original-source byte ranges.
#[cfg(test)]
pub(crate) fn tokenize_spanned(input: &str) -> Result<Vec<SpannedToken>, ParseError> {
    tokenize_spanned_with_limit(input, usize::MAX)
}

pub(crate) fn tokenize_spanned_with_limit(
    input: &str,
    max_tokens: usize,
) -> Result<Vec<SpannedToken>, ParseError> {
    let mut chars = input.char_indices().peekable();
    let mut out = Vec::new();
    while let Some((start, c)) = chars.next() {
        let item = match c {
            '%' => {
                skip_line(&mut chars);
                continue;
            }
            ' ' => SpannedToken {
                token: Token::Space,
                span: SourceSpan::new(start, start + 1),
            },
            '\t' | '\n' | '\r' => continue,
            '{' => simple(Token::BeginGroup, start, c),
            '}' => simple(Token::EndGroup, start, c),
            '^' => simple(Token::Superscript, start, c),
            '_' => simple(Token::Subscript, start, c),
            '&' => simple(Token::AlignmentTab, start, c),
            '$' => {
                if chars.peek().map(|(_, next)| *next) == Some('$') {
                    let (second, second_char) = chars.next().expect("peeked dollar");
                    SpannedToken {
                        token: Token::DisplayShift,
                        span: SourceSpan::new(start, second + second_char.len_utf8()),
                    }
                } else {
                    simple(Token::MathShift, start, c)
                }
            }
            '\\' => command(input.len(), start, &mut chars)?,
            other => simple(Token::Char(other), start, other),
        };
        push_token(&mut out, item, max_tokens)?;
    }
    Ok(out)
}

fn simple(token: Token, start: usize, c: char) -> SpannedToken {
    SpannedToken {
        token,
        span: SourceSpan::new(start, start + c.len_utf8()),
    }
}

fn push_token(
    out: &mut Vec<SpannedToken>,
    token: SpannedToken,
    max_tokens: usize,
) -> Result<(), ParseError> {
    let next = out
        .len()
        .checked_add(1)
        .ok_or_else(|| ParseError::resource_limit(token.span, ParseResource::Tokens, max_tokens))?;
    if next > max_tokens {
        return Err(ParseError::resource_limit(
            token.span,
            ParseResource::Tokens,
            max_tokens,
        ));
    }
    out.push(token);
    Ok(())
}

fn skip_line(chars: &mut Peekable<CharIndices<'_>>) {
    for (_, c) in chars.by_ref() {
        if c == '\n' {
            break;
        }
    }
}

fn command(
    input_len: usize,
    slash_start: usize,
    chars: &mut Peekable<CharIndices<'_>>,
) -> Result<SpannedToken, ParseError> {
    let Some(&(first_start, first)) = chars.peek() else {
        return Err(ParseError::trailing_backslash(SourceSpan::new(
            slash_start,
            input_len,
        )));
    };
    if first.is_ascii_alphabetic() {
        let mut name = String::new();
        let mut end = first_start;
        while let Some(&(offset, c)) = chars.peek() {
            if c.is_ascii_alphabetic() {
                name.push(c);
                chars.next();
                end = offset + c.len_utf8();
            } else {
                break;
            }
        }
        let span = SourceSpan::new(slash_start, end);
        while matches!(
            chars.peek().map(|(_, c)| *c),
            Some(' ' | '\t' | '\n' | '\r')
        ) {
            chars.next();
        }
        Ok(SpannedToken {
            token: Token::Command(name),
            span,
        })
    } else {
        chars.next();
        Ok(SpannedToken {
            token: Token::Command(first.to_string()),
            span: SourceSpan::new(slash_start, first_start + first.len_utf8()),
        })
    }
}
