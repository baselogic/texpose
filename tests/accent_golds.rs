//! Gold runner: `golds/accents.toml` is the accent/decoration contract.

mod common;

use texpose::{layout, parse, BoxContent, MathBox, MathFont, MathStyle, ParseError};

#[derive(Default)]
struct Rec {
    name: String,
    kind: String,
    style: String,
    input: String,
    expect: String,
    lines: String,
    error: String,
}

fn unescape(s: &str) -> String {
    let mut out = String::new();
    let mut chars = s.chars();
    while let Some(c) = chars.next() {
        if c == '\\' {
            match chars.next() {
                Some('\\') => out.push('\\'),
                Some('n') => out.push('\n'),
                Some('t') => out.push('\t'),
                Some('"') => out.push('"'),
                Some(other) => out.push(other),
                None => break,
            }
        } else {
            out.push(c);
        }
    }
    out
}

fn parse_value(raw: &str) -> String {
    let t = raw.trim();
    if let Some(s) = t.strip_prefix('"').and_then(|s| s.strip_suffix('"')) {
        unescape(s)
    } else {
        t.to_string()
    }
}

fn load_golds() -> Vec<Rec> {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/golds/accents.toml");
    let text = std::fs::read_to_string(path).expect("accents.toml");
    let mut recs = Vec::new();
    let mut rec = Rec::default();
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if line == "[[gold]]" {
            if !rec.name.is_empty() {
                recs.push(rec);
            }
            rec = Rec::default();
            continue;
        }
        let Some((k, v)) = line.split_once('=') else {
            continue;
        };
        let k = k.trim();
        let v = parse_value(v);
        match k {
            "name" => rec.name = v,
            "kind" => rec.kind = v,
            "style" => rec.style = v,
            "input" => rec.input = v,
            "expect" => rec.expect = v,
            "lines" => rec.lines = v,
            "error" => rec.error = v,
            _ => panic!("unknown gold field {k}"),
        }
    }
    if !rec.name.is_empty() {
        recs.push(rec);
    }
    recs
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
