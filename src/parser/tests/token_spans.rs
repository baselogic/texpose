//! Lexer provenance stays an internal parser invariant rather than public API.

use super::super::token::{tokenize_spanned, Token};

fn span_of(source: &str, needle: &str) -> (usize, usize) {
    let start = source.find(needle).expect("needle");
    (start, start + needle.len())
}

#[test]
fn tokenizer_spans_are_original_utf8_byte_ranges() {
    let source = "α % skipped\n\\frac  {β}{2}";
    let tokens = tokenize_spanned(source).expect("tokenize");

    assert_eq!(tokens[0].token, Token::Char('α'));
    assert_eq!((tokens[0].span.start(), tokens[0].span.end()), (0, 2));

    let command = tokens
        .iter()
        .find(|item| matches!(&item.token, Token::Command(name) if *name == "frac"))
        .expect("frac token");
    assert_eq!(
        (command.span.start(), command.span.end()),
        span_of(source, r"\frac")
    );

    let beta = tokens
        .iter()
        .find(|item| item.token == Token::Char('β'))
        .expect("beta token");
    assert_eq!((beta.span.start(), beta.span.end()), span_of(source, "β"));
}
