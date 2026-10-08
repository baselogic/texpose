//! Gold runner: `golds/accents.toml` is the accent/decoration contract.

use super::{common, gold_support};

use crate::test_support::{layout, parse, BoxContent, MathBox, MathFont, MathStyle, ParseError};

struct Rec {
    name: String,
    kind: String,
    style: String,
    input: String,
    expect: String,
    error: String,
}

fn load_golds() -> Vec<Rec> {
    gold_support::load("golds/accents.toml")
        .into_iter()
        .map(|gold| {
            match gold.kind() {
                "ast" => gold.expect_fields(&["name", "kind", "input", "expect"], &[]),
                "dims" | "lines" => {
                    gold.expect_fields(&["name", "kind", "input", "expect"], &["style"]);
                }
                "err" => {
                    gold.expect_fields(&["name", "kind", "input", "expect", "error"], &[]);
                }
                other => panic!("{}: unknown kind {other}", gold.name()),
            }
            Rec {
                name: gold.name().to_string(),
                kind: gold.kind().to_string(),
                style: gold.optional("style").unwrap_or("").to_string(),
                input: gold.required("input").to_string(),
                expect: gold.required("expect").to_string(),
                error: gold.optional("error").unwrap_or("").to_string(),
            }
        })
        .collect()
}

fn parse_style(s: &str) -> MathStyle {
    match s {
        "" | "text" => MathStyle::Text,
        "display" => MathStyle::Display,
        "text-cramped" => MathStyle::TextCramped,
        other => panic!("unknown style {other}"),
    }
}

fn collect_lines(b: &MathBox, out: &mut Vec<String>) {
    match &b.content {
        BoxContent::Line {
            x1,
            y1,
            x2,
            y2,
            thickness,
        } => out.push(format!(
            "x1={} y1={} x2={} y2={} t={}",
            x1.to_dec_string(),
            y1.to_dec_string(),
            x2.to_dec_string(),
            y2.to_dec_string(),
            thickness.to_dec_string()
        )),
        BoxContent::HList(v) | BoxContent::VList(v) | BoxContent::Overlap(v) => {
            for k in v {
                collect_lines(k, out);
            }
        }
        BoxContent::Color(_, inner)
        | BoxContent::BackColor(_, inner)
        | BoxContent::Frame { inner, .. } => collect_lines(inner, out),
        _ => {}
    }
}

fn variant_name(err: &ParseError) -> &'static str {
    match err.kind() {
        texpose::ParseErrorKind::TrailingBackslash => "TrailingBackslash",
        texpose::ParseErrorKind::UnknownCommand => "UnknownCommand",
        texpose::ParseErrorKind::UnsupportedCommand => "UnsupportedCommand",
        texpose::ParseErrorKind::UnclosedGroup => "UnclosedGroup",
        texpose::ParseErrorKind::UnexpectedGroupEnd => "UnexpectedGroupEnd",
        texpose::ParseErrorKind::UnmatchedDelimiter => "UnmatchedDelimiter",
        texpose::ParseErrorKind::MismatchedEnvironment => "MismatchedEnvironment",
        texpose::ParseErrorKind::MalformedArgument => "MalformedArgument",
        texpose::ParseErrorKind::MalformedDimension => "MalformedDimension",
        texpose::ParseErrorKind::MalformedMatrix => "MalformedMatrix",
        texpose::ParseErrorKind::ResourceLimit => "ResourceLimit",
    }
}

fn lay(font: &MathFont, rec: &Rec) -> MathBox {
    let ast = parse(&rec.input).unwrap_or_else(|e| panic!("{}: parse {e}", rec.name));
    layout(&ast, font, parse_style(&rec.style))
        .unwrap_or_else(|e| panic!("{}: layout {e}", rec.name))
}

#[test]
fn accent_golds() {
    let recs = load_golds();
    assert!(!recs.is_empty(), "no accent golds loaded");
    let font = common::stix_two_math().expect("STIX Two Math");
    for rec in recs {
        match rec.kind.as_str() {
            "ast" => {
                let got = parse(&rec.input).unwrap_or_else(|e| panic!("{}: {e}", rec.name));
                assert_eq!(got.gold(), rec.expect, "{}", rec.name);
            }
            "dims" => {
                let bx = lay(&font, &rec);
                assert_eq!(bx.dim_gold(), rec.expect, "{}", rec.name);
            }
            "lines" => {
                let bx = lay(&font, &rec);
                let mut got = Vec::new();
                collect_lines(&bx, &mut got);
                assert_eq!(got.join(" | "), rec.expect, "{}", rec.name);
            }
            "err" => {
                let err = parse(&rec.input).expect_err(&rec.name);
                assert_eq!(variant_name(&err), rec.error, "{}: {err}", rec.name);
                assert_eq!(err.to_string(), rec.expect, "{}", rec.name);
            }
            other => panic!("{}: unknown kind {other}", rec.name),
        }
    }
}
