//! Parser diagnostics carry typed kinds and byte spans in the original source.

use texpose::{parse, ParseErrorDetail, ParseErrorKind, SourceSpan};

fn span_of(source: &str, needle: &str) -> (usize, usize) {
    let start = source.find(needle).expect("needle");
    (start, start + needle.len())
}

fn assert_span(actual: SourceSpan, expected: (usize, usize)) {
    assert_eq!((actual.start(), actual.end()), expected);
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
        ("{α", ParseErrorKind::UnclosedGroup, (0, 1)),
        ("x}", ParseErrorKind::UnexpectedGroupEnd, (1, 2)),
        (
            r"\hspace{12zz}",
            ParseErrorKind::MalformedDimension,
            span_of(r"\hspace{12zz}", "12zz"),
        ),
        ("x__y", ParseErrorKind::MalformedArgument, (2, 3)),
    ];

    for (source, kind, span) in cases {
        let error = parse(source).expect_err(source);
        assert_eq!(error.kind(), kind, "{source}: {error}");
        assert_eq!(
            (error.span().start(), error.span().end()),
            span,
            "{source}: {error}"
        );
    }
}

#[test]
fn environment_mismatch_carries_both_names_and_closing_name_span() {
    let source = r"\begin{matrix}x\end{aligned}";
    let error = parse(source).expect_err("mismatch");
    assert_eq!(error.kind(), ParseErrorKind::MismatchedEnvironment);
    assert_span(error.span(), span_of(source, "aligned"));
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
