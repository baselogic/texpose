//! Parser diagnostics carry typed kinds and byte spans in the original source.

use texpose::{parse, tokenize_spanned, ParseErrorDetail, ParseErrorKind, SourceSpan, Token};

fn span_of(source: &str, needle: &str) -> SourceSpan {
    let start = source.find(needle).expect("needle");
    SourceSpan {
        start,
        end: start + needle.len(),
    }
}

#[test]
fn tokenizer_spans_are_original_utf8_byte_ranges() {
    let source = "α % skipped\n\\frac  {β}{2}";
    let tokens = tokenize_spanned(source).expect("tokenize");

    assert_eq!(tokens[0].token, Token::Char('α'));
    assert_eq!(tokens[0].span, SourceSpan { start: 0, end: 2 });

    let command = tokens
        .iter()
        .find(|item| matches!(&item.token, Token::Command(name) if name == "frac"))
        .expect("frac token");
    assert_eq!(command.span, span_of(source, r"\frac"));

    let beta = tokens
        .iter()
        .find(|item| item.token == Token::Char('β'))
        .expect("beta token");
    assert_eq!(beta.span, span_of(source, "β"));
}

#[test]
fn parser_errors_keep_typed_kind_and_original_source_span() {
    let cases = [
        (
            "α\\",
            ParseErrorKind::TrailingBackslash,
            span_of("α\\", r"\"),
        ),
        (
            "α+\\doesnotexist",
            ParseErrorKind::UnknownCommand,
            span_of("α+\\doesnotexist", r"\doesnotexist"),
        ),
        (
            "{α",
            ParseErrorKind::UnclosedGroup,
            SourceSpan { start: 0, end: 1 },
        ),
        (
            "x}",
            ParseErrorKind::UnexpectedGroupEnd,
            SourceSpan { start: 1, end: 2 },
        ),
        (
            r"\hspace{12zz}",
            ParseErrorKind::MalformedDimension,
            span_of(r"\hspace{12zz}", "12zz"),
        ),
        (
            "x__y",
            ParseErrorKind::MalformedArgument,
            SourceSpan { start: 2, end: 3 },
        ),
    ];

    for (source, kind, span) in cases {
        let error = parse(source).expect_err(source);
        assert_eq!(error.kind(), kind, "{source}: {error}");
        assert_eq!(error.span(), span, "{source}: {error}");
    }
}

#[test]
fn environment_mismatch_carries_both_names_and_closing_name_span() {
    let source = r"\begin{matrix}x\end{aligned}";
    let error = parse(source).expect_err("mismatch");
    assert_eq!(error.kind(), ParseErrorKind::MismatchedEnvironment);
    assert_eq!(error.span(), span_of(source, "aligned"));
    assert_eq!(
        error.detail(),
        &ParseErrorDetail::Environment {
            expected: "matrix".into(),
            found: "aligned".into(),
        }
    );
}

#[test]
fn every_required_parser_error_kind_has_a_reachable_typed_path() {
    let cases = [
        ("\\", ParseErrorKind::TrailingBackslash),
        (r"\unknowncommand", ParseErrorKind::UnknownCommand),
        (r"\mathunsupported{x}", ParseErrorKind::UnsupportedCommand),
        ("{x", ParseErrorKind::UnclosedGroup),
        ("x}", ParseErrorKind::UnexpectedGroupEnd),
        (r"\right)", ParseErrorKind::UnmatchedDelimiter),
        (
            r"\begin{matrix}x\end{aligned}",
            ParseErrorKind::MismatchedEnvironment,
        ),
        ("x__y", ParseErrorKind::MalformedArgument),
        (r"\hspace{1zz}", ParseErrorKind::MalformedDimension),
        (r"\begin{matrix}x", ParseErrorKind::MalformedMatrix),
    ];

    for (source, expected) in cases {
        let error = parse(source).expect_err(source);
        assert_eq!(error.kind(), expected, "{source}: {error}");
    }

    let error = texpose::parse_with_options("xx", &texpose::ParseOptions::new().with_max_tokens(1))
        .expect_err("token budget");
    assert_eq!(error.kind(), ParseErrorKind::ResourceLimit);
}
