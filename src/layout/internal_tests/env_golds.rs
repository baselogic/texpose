//! Gold runner: `golds/envs.toml` is the Milestone 7 environment contract.

use super::{common, gold_support};

use crate::test_support::{
    layout, layout_with_numbering, parse, BoxContent, MathBox, MathStyle, NumberingState,
    ParseError,
};

struct Rec {
    name: String,
    kind: String,
    style: String,
    input: String,
    expect: String,
    error: String,
    key: String,
}

fn load_golds() -> Vec<Rec> {
    gold_support::load("golds/envs.toml")
        .into_iter()
        .map(|gold| {
            match gold.kind() {
                "ast" => gold.expect_fields(&["name", "kind", "input", "expect"], &[]),
                "dims" | "eq_x" => {
                    gold.expect_fields(&["name", "kind", "input", "expect"], &["style"]);
                }
                "err" => {
                    gold.expect_fields(&["name", "kind", "input", "expect", "error"], &[]);
                }
                "label" => {
                    gold.expect_fields(&["name", "kind", "input", "expect", "key"], &["style"]);
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
                key: gold.optional("key").unwrap_or("").to_string(),
            }
        })
        .collect()
}

fn parse_style(s: &str) -> MathStyle {
    match s {
        "" | "display" => MathStyle::Display,
        "text" => MathStyle::Text,
        other => panic!("unknown style {other}"),
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

fn glyph_xs(b: &MathBox, x: texpose::Dim, ch: char, out: &mut Vec<texpose::Dim>) {
    match &b.content {
        BoxContent::Glyph { ch: c, .. } if *c == ch => out.push(x),
        BoxContent::HList(v) => {
            let mut cx = x;
            for k in v {
                glyph_xs(k, cx.clone(), ch, out);
                cx = cx.checked_add(&k.width).expect("glyph x overflow");
            }
        }
        BoxContent::VList(v) | BoxContent::Overlap(v) => {
            for k in v {
                glyph_xs(k, x.clone(), ch, out);
            }
        }
        BoxContent::Color(_, inner)
        | BoxContent::BackColor(_, inner)
        | BoxContent::Frame { inner, .. } => glyph_xs(inner, x, ch, out),
        _ => {}
    }
}

#[test]
fn env_golds() {
    let recs = load_golds();
    assert!(!recs.is_empty(), "no env golds loaded");
    let font = common::stix_two_math().expect("STIX Two Math");
    for rec in recs {
        match rec.kind.as_str() {
            "ast" => {
                let got = parse(&rec.input).unwrap_or_else(|e| panic!("{}: {e}", rec.name));
                assert_eq!(got.gold(), rec.expect, "{}", rec.name);
            }
            "dims" => {
                let ast = parse(&rec.input).unwrap_or_else(|e| panic!("{}: parse {e}", rec.name));
                let bx = layout(&ast, &font, parse_style(&rec.style))
                    .unwrap_or_else(|e| panic!("{}: layout {e}", rec.name));
                assert_eq!(bx.dim_gold(), rec.expect, "{}", rec.name);
            }
            "err" => {
                let err = parse(&rec.input).expect_err(&rec.name);
                assert_eq!(variant_name(&err), rec.error, "{}: {err}", rec.name);
                assert!(err.to_string().contains(&rec.expect), "{}: {err}", rec.name);
            }
            "label" => {
                let ast = parse(&rec.input).unwrap_or_else(|e| panic!("{}: parse {e}", rec.name));
                let mut st = NumberingState::default();
                layout_with_numbering(&ast, &font, parse_style(&rec.style), &mut st)
                    .unwrap_or_else(|e| panic!("{}: layout {e}", rec.name));
                let got = st
                    .label(&rec.key)
                    .unwrap_or_else(|| panic!("{}: no label", rec.name));
                assert_eq!(got, rec.expect, "{}", rec.name);
            }
            "eq_x" => {
                let ast = parse(&rec.input).unwrap_or_else(|e| panic!("{}: parse {e}", rec.name));
                let bx = layout(&ast, &font, parse_style(&rec.style))
                    .unwrap_or_else(|e| panic!("{}: layout {e}", rec.name));
                let mut expected_chars = rec.expect.chars();
                let ch = expected_chars.next().expect("eq_x char");
                assert!(
                    expected_chars.next().is_none(),
                    "{}: eq_x expect must be exactly one character, got {:?}",
                    rec.name,
                    rec.expect
                );
                let mut xs = Vec::new();
                glyph_xs(&bx, texpose::Dim::zero(), ch, &mut xs);
                assert!(
                    xs.len() >= 2,
                    "{}: need two {ch:?} glyphs, got {}",
                    rec.name,
                    xs.len()
                );
                let first = xs[0].clone();
                for x in &xs[1..] {
                    assert!(
                        first.eq_dim(x),
                        "{}: {ch:?} x {} vs {}",
                        rec.name,
                        first.to_dec_string(),
                        x.to_dec_string()
                    );
                }
            }
            other => panic!("{}: unknown kind {other}", rec.name),
        }
    }
}
