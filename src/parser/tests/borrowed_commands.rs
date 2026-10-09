//! Command tokens borrow the exact original UTF-8 source bytes.

use super::super::token::{tokenize_spanned, Token};
use crate::{parse, ParseErrorKind};

#[test]
fn control_words_and_symbols_borrow_source_without_allocating_names() {
    let source = "α % ignored\n\\frac  {β}\\%\\ \\λ";
    let tokens = tokenize_spanned(source).expect("tokenize UTF-8 source");
    let mut names: Vec<&str> = Vec::new();
    for token in &tokens {
        if let Token::Command(name) = &token.token {
            let from = token.span.start() + 1;
            let through = token.span.end();
            assert_eq!(*name, &source[from..through]);
            assert!(std::ptr::eq(name.as_ptr(), source[from..].as_ptr()));
            names.push(&**name);
        }
    }
    assert_eq!(names, ["frac", "%", " ", "λ"]);
    // Whitespace after a control word is swallowed but excluded from its span.
    let frac = tokens
        .iter()
        .find(|t| matches!(&t.token, Token::Command(name) if *name == "frac"))
        .unwrap();
    assert_eq!(frac.span.end(), source.find("frac").unwrap() + 4);
}

#[test]
fn parser_keeps_public_ast_and_error_spans_with_borrowed_command_names() {
    let src = r"\alpha+\beta";
    assert!(parse(src).is_ok());
    let unknown = r"α+\notarealcommand";
    let err = parse(unknown).expect_err("unknown command");
    assert_eq!(err.kind(), ParseErrorKind::UnknownCommand);
    assert_eq!(
        err.span().start(),
        unknown.find(r"\notarealcommand").unwrap()
    );
    let trailing = parse("x\\").expect_err("trailing slash");
    assert_eq!(trailing.kind(), ParseErrorKind::TrailingBackslash);
}
