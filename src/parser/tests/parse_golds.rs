//! Gold runner: `golds/parse.toml` is the parser contract.

use crate::gold_support;
use crate::{symbols, ParseError, ParseErrorKind, SymbolKind};

use super::super::parse;

struct Rec {
    name: String,
    kind: String,
    error: String,
    input: String,
    expect: String,
}

fn load_golds() -> Vec<Rec> {
    gold_support::load("golds/parse.toml")
        .into_iter()
        .map(|gold| {
            match gold.kind() {
                "ast" => gold.expect_fields(&["name", "kind", "input", "expect"], &[]),
                "err" => {
                    gold.expect_fields(&["name", "kind", "error", "input", "expect"], &[]);
                }
                other => panic!("{}: unknown kind {other}", gold.name()),
            }
            Rec {
                name: gold.name().to_string(),
                kind: gold.kind().to_string(),
                error: gold.optional("error").unwrap_or("").to_string(),
                input: gold.required("input").to_string(),
                expect: gold.required("expect").to_string(),
            }
        })
        .collect()
}

fn variant_name(err: &ParseError) -> &'static str {
    match err.kind() {
        ParseErrorKind::TrailingBackslash => "TrailingBackslash",
        ParseErrorKind::UnknownCommand => "UnknownCommand",
        ParseErrorKind::UnsupportedCommand => "UnsupportedCommand",
        ParseErrorKind::UnclosedGroup => "UnclosedGroup",
        ParseErrorKind::UnexpectedGroupEnd => "UnexpectedGroupEnd",
        ParseErrorKind::UnmatchedDelimiter => "UnmatchedDelimiter",
        ParseErrorKind::MismatchedEnvironment => "MismatchedEnvironment",
        ParseErrorKind::MalformedArgument => "MalformedArgument",
        ParseErrorKind::MalformedDimension => "MalformedDimension",
        ParseErrorKind::MalformedMatrix => "MalformedMatrix",
        ParseErrorKind::ResourceLimit => "ResourceLimit",
    }
}

#[test]
fn parse_golds() {
    let recs = load_golds();
    assert!(!recs.is_empty(), "no parse golds loaded");
    for rec in recs {
        match rec.kind.as_str() {
            "ast" => {
                let got = parse(&rec.input).unwrap_or_else(|e| panic!("{}: {e}", rec.name));
                assert_eq!(got.gold(), rec.expect, "{}", rec.name);
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

fn catalog_sample(latex: &str, name: &str, kind: SymbolKind) -> String {
    if !latex.starts_with('\\') {
        return latex.to_string();
    }
    match name {
        "frac" => r"\frac{1}{2}".into(),
        "sqrt" if latex.contains('[') => r"\sqrt[n]{x}".into(),
        "sqrt" => r"\sqrt{x}".into(),
        "begin" if latex.contains("pmatrix") => r"\begin{pmatrix}a&b\\c&d\end{pmatrix}".into(),
        "begin" if latex.contains("vmatrix") => r"\begin{vmatrix}a&b\\c&d\end{vmatrix}".into(),
        "begin" => r"\begin{matrix}a&b\\c&d\end{matrix}".into(),
        "left" => r"\left(x\right)".into(),
        "sum" | "int" | "lim" => format!("\\{name}"),
        "bar"
        | "hat"
        | "tilde"
        | "vec"
        | "dot"
        | "ddot"
        | "dddot"
        | "ddddot"
        | "check"
        | "breve"
        | "acute"
        | "grave"
        | "widehat"
        | "widetilde"
        | "underbrace"
        | "overbrace"
        | "mathring"
        | "overline"
        | "underline"
        | "overleftarrow"
        | "overrightarrow"
        | "overleftrightarrow"
        | "underleftarrow"
        | "underrightarrow"
        | "underleftrightarrow"
        | "cancel"
        | "bcancel"
        | "xcancel"
        | "boxed"
        | "fbox" => {
            format!("\\{name}{{x}}")
        }
        "cancelto" => r"\cancelto{0}{x}".into(),
        "text" => r"\text{a}".into(),
        "mathcal" | "mathbb" => format!("\\{name}{{X}}"),
        "not" => r"\not\equiv".into(),
        _ if kind == SymbolKind::Modifier => format!("\\{name}{{x}}"),
        _ if kind == SymbolKind::Container && latex.contains("{}") => latex.replace("{}", "{x}"),
        _ if latex.contains(' ') => latex.to_string(),
        _ => format!("\\{name}"),
    }
}

#[test]
fn catalog_commands_parse() {
    let mut failed = Vec::new();
    for e in symbols() {
        let sample = catalog_sample(e.latex, e.command_name(), e.kind);
        if let Err(err) = parse(&sample) {
            failed.push(format!("{} [{}]: {err}", e.latex, sample));
        }
    }
    assert!(
        failed.is_empty(),
        "catalog commands failed to parse:\n{}",
        failed.join("\n")
    );
}
