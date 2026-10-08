//! Gold runner: `golds/milestone1.toml` is the contract.

use super::{common, gold_support};

use crate::test_support::{
    category_count, format_tokens, lookup, named_color, parse_color_spec, symbols, tokenize,
    ColorTable, Dim, Error, MathBox, MathFont, ParseErrorKind,
};

struct Rec {
    name: String,
    kind: String,
    op: String,
    model: String,
    input: String,
    lhs: String,
    rhs: String,
    widths: String,
    heights: String,
    depths: String,
    expect: String,
    expect_width: String,
    expect_height: String,
    expect_depth: String,
}

fn load_golds() -> Vec<Rec> {
    gold_support::load("golds/milestone1.toml")
        .into_iter()
        .map(|gold| {
            match gold.kind() {
                "tokenize" | "tokenize_err" => {
                    gold.expect_fields(&["name", "kind", "input", "expect"], &[]);
                }
                "dim" => match gold.required("op") {
                    "add" | "mul" | "div" | "font_units" => {
                        gold.expect_fields(&["name", "kind", "op", "lhs", "rhs", "expect"], &[])
                    }
                    "from_mu" | "as_ratio" => {
                        gold.expect_fields(&["name", "kind", "op", "lhs", "expect"], &[]);
                    }
                    other => panic!("{}: unknown dim op {other}", gold.name()),
                },
                "box" => match gold.required("op") {
                    "hpack" => {
                        gold.expect_fields(&["name", "kind", "op", "widths", "expect_width"], &[])
                    }
                    "vpack" => gold.expect_fields(
                        &[
                            "name",
                            "kind",
                            "op",
                            "heights",
                            "depths",
                            "expect_height",
                            "expect_depth",
                        ],
                        &[],
                    ),
                    other => panic!("{}: unknown box op {other}", gold.name()),
                },
                "font" => match gold.required("op") {
                    "advance" | "missing" => {
                        gold.expect_fields(&["name", "kind", "op", "input", "expect"], &[])
                    }
                    "units_per_em" | "hhea" | "sha256" => {
                        gold.expect_fields(&["name", "kind", "op", "expect"], &[]);
                    }
                    other => panic!("{}: unknown font op {other}", gold.name()),
                },
                "symbol" => match gold.required("op") {
                    "lookup" | "category_count" => {
                        gold.expect_fields(&["name", "kind", "op", "input", "expect"], &[])
                    }
                    "count" => gold.expect_fields(&["name", "kind", "op", "expect"], &[]),
                    other => panic!("{}: unknown symbol op {other}", gold.name()),
                },
                "color" => match gold.required("op") {
                    "named" | "error_named" => {
                        gold.expect_fields(&["name", "kind", "op", "input", "expect"], &[])
                    }
                    "named_count" => {
                        gold.expect_fields(&["name", "kind", "op", "expect"], &[]);
                    }
                    "model" | "error" | "error_malformed" => {
                        gold.expect_fields(&["name", "kind", "op", "model", "input", "expect"], &[])
                    }
                    "define" => gold.expect_fields(
                        &["name", "kind", "op", "lhs", "model", "input", "expect"],
                        &[],
                    ),
                    other => panic!("{}: unknown color op {other}", gold.name()),
                },
                other => panic!("{}: unknown kind {other}", gold.name()),
            }

            Rec {
                name: gold.name().to_string(),
                kind: gold.kind().to_string(),
                op: gold.optional("op").unwrap_or("").to_string(),
                model: gold.optional("model").unwrap_or("").to_string(),
                input: gold.optional("input").unwrap_or("").to_string(),
                lhs: gold.optional("lhs").unwrap_or("").to_string(),
                rhs: gold.optional("rhs").unwrap_or("").to_string(),
                widths: gold.optional("widths").unwrap_or("").to_string(),
                heights: gold.optional("heights").unwrap_or("").to_string(),
                depths: gold.optional("depths").unwrap_or("").to_string(),
                expect: gold.optional("expect").unwrap_or("").to_string(),
                expect_width: gold.optional("expect_width").unwrap_or("").to_string(),
                expect_height: gold.optional("expect_height").unwrap_or("").to_string(),
                expect_depth: gold.optional("expect_depth").unwrap_or("").to_string(),
            }
        })
        .collect()
}

fn csv_dims(s: &str) -> Vec<Dim> {
    s.split(',')
        .map(|p| Dim::parse(p.trim()).expect("gold dimension"))
        .collect()
}

#[test]
fn milestone1_golds() {
    let recs = load_golds();
    assert!(!recs.is_empty(), "no golds loaded");
    let font = common::stix_two_math().expect("STIX Two Math");
    for rec in recs {
        match rec.kind.as_str() {
            "tokenize" => {
                let got = format_tokens(&tokenize(&rec.input).expect(&rec.name));
                assert_eq!(got, rec.expect, "{}", rec.name);
            }
            "tokenize_err" => {
                let err = tokenize(&rec.input).expect_err(&rec.name);
                assert_eq!(
                    err.kind(),
                    ParseErrorKind::TrailingBackslash,
                    "{}",
                    rec.name
                );
                assert!(err.to_string().contains(&rec.expect), "{}", rec.name);
            }
            "dim" => match rec.op.as_str() {
                "add" => {
                    let lhs = Dim::parse(&rec.lhs).expect(&rec.name);
                    let rhs = Dim::parse(&rec.rhs).expect(&rec.name);
                    let g = lhs.checked_add(&rhs).expect(&rec.name);
                    assert!(
                        g.eq_dim(&Dim::parse(&rec.expect).expect(&rec.name)),
                        "{}",
                        rec.name
                    );
                }
                "mul" => {
                    let lhs = Dim::parse(&rec.lhs).expect(&rec.name);
                    let rhs = Dim::parse(&rec.rhs).expect(&rec.name);
                    let g = lhs.checked_mul(&rhs).expect(&rec.name);
                    assert!(
                        g.eq_dim(&Dim::parse(&rec.expect).expect(&rec.name)),
                        "{}",
                        rec.name
                    );
                }
                "div" => {
                    let lhs = Dim::parse(&rec.lhs).expect(&rec.name);
                    let rhs = Dim::parse(&rec.rhs).expect(&rec.name);
                    let g = lhs.checked_div(&rhs).expect(&rec.name);
                    assert!(
                        g.eq_dim(&Dim::parse(&rec.expect).expect(&rec.name)),
                        "{}",
                        rec.name
                    );
                }
                "from_mu" => {
                    let lhs = Dim::parse(&rec.lhs).expect(&rec.name);
                    let g = Dim::from_mu(&lhs).expect(&rec.name);
                    assert!(
                        g.eq_dim(&Dim::parse(&rec.expect).expect(&rec.name)),
                        "{}",
                        rec.name
                    );
                }
                "as_ratio" => {
                    let (n, d) = Dim::parse(&rec.lhs).expect(&rec.name).as_ratio();
                    assert_eq!(format!("{n}/{d}"), rec.expect, "{}", rec.name);
                }
                "font_units" => {
                    let units: i64 = rec.lhs.parse().expect("units");
                    let upem: u16 = rec.rhs.parse().expect("upem");
                    let g = Dim::from_font_units(units, upem).expect(&rec.name);
                    assert!(
                        g.eq_dim(&Dim::parse(&rec.expect).expect(&rec.name)),
                        "{}",
                        rec.name
                    );
                }
                other => panic!("{}: unknown dim op {other}", rec.name),
            },
            "box" => match rec.op.as_str() {
                "hpack" => {
                    let kids: Vec<MathBox> = csv_dims(&rec.widths)
                        .into_iter()
                        .map(|w| MathBox::rule(w, Dim::zero(), Dim::zero()))
                        .collect();
                    let b = MathBox::hpack(kids).expect(&rec.name);
                    assert!(
                        b.width
                            .eq_dim(&Dim::parse(&rec.expect_width).expect(&rec.name)),
                        "{}: {}",
                        rec.name,
                        b.width
                    );
                }
                "vpack" => {
                    let hs = csv_dims(&rec.heights);
                    let ds = csv_dims(&rec.depths);
                    assert_eq!(hs.len(), ds.len(), "{}", rec.name);
                    let kids: Vec<MathBox> = hs
                        .into_iter()
                        .zip(ds)
                        .map(|(h, d)| MathBox::rule(Dim::zero(), h, d))
                        .collect();
                    let b = MathBox::vpack(kids).expect(&rec.name);
                    assert!(
                        b.height
                            .eq_dim(&Dim::parse(&rec.expect_height).expect(&rec.name)),
                        "{}",
                        rec.name
                    );
                    assert!(
                        b.depth
                            .eq_dim(&Dim::parse(&rec.expect_depth).expect(&rec.name)),
                        "{}: {}",
                        rec.name,
                        b.depth
                    );
                }
                other => panic!("{}: unknown box op {other}", rec.name),
            },
            "font" => match rec.op.as_str() {
                "units_per_em" => {
                    assert_eq!(font.units_per_em().to_string(), rec.expect, "{}", rec.name);
                }
                "hhea" => {
                    let got = format!("{},{}", font.ascender_fu(), font.descender_fu());
                    assert_eq!(got, rec.expect, "{}", rec.name);
                }
                "advance" => {
                    let ch = rec.input.chars().next().expect("char");
                    let g = font.glyph(ch).expect(&rec.name);
                    assert_eq!(g.advance_fu.to_string(), rec.expect, "{}", rec.name);
                }
                "sha256" => {
                    assert_eq!(
                        MathFont::sha256_hex(common::STIX_TWO_MATH_OTF),
                        rec.expect,
                        "{}",
                        rec.name
                    );
                }
                "missing" => {
                    let ch = rec.input.chars().next().expect("char");
                    let err = font.glyph(ch).expect_err(&rec.name);
                    match err {
                        Error::Font(_) => {}
                        other => panic!("{}: {other}", rec.name),
                    }
                    assert!(err.to_string().contains(&rec.expect), "{}", rec.name);
                }
                other => panic!("{}: unknown font op {other}", rec.name),
            },
            "symbol" => match rec.op.as_str() {
                "count" => {
                    assert_eq!(symbols().len().to_string(), rec.expect, "{}", rec.name);
                }
                "lookup" => {
                    let e = lookup(&rec.input).expect(&rec.name);
                    if rec.expect.chars().count() == 1 {
                        assert_eq!(e.glyph, rec.expect, "{}", rec.name);
                    } else {
                        assert_eq!(e.latex, rec.expect, "{}", rec.name);
                    }
                }
                "category_count" => {
                    assert_eq!(
                        category_count(&rec.input).to_string(),
                        rec.expect,
                        "{}",
                        rec.name
                    );
                }
                other => panic!("{}: unknown symbol op {other}", rec.name),
            },
            "color" => match rec.op.as_str() {
                "named" => {
                    let c = named_color(&rec.input).expect(&rec.name);
                    assert_eq!(c.css_hex(), rec.expect, "{}", rec.name);
                }
                "named_count" => {
                    assert_eq!(
                        ColorTable::new().len().to_string(),
                        rec.expect,
                        "{}",
                        rec.name
                    );
                }
                "model" => {
                    let c = parse_color_spec(&rec.model, &rec.input, None).expect(&rec.name);
                    assert_eq!(c.css_hex(), rec.expect, "{}", rec.name);
                }
                "define" => {
                    let mut t = ColorTable::new();
                    let c = t.define(&rec.lhs, &rec.model, &rec.input).expect(&rec.name);
                    assert_eq!(c.css_hex(), rec.expect, "{}", rec.name);
                    assert_eq!(
                        t.get(&rec.lhs).expect("lookup").css_hex(),
                        rec.expect,
                        "{}",
                        rec.name
                    );
                }
                "error" => {
                    let err = parse_color_spec(&rec.model, &rec.input, None).expect_err(&rec.name);
                    assert!(
                        matches!(err, Error::Unsupported { .. }),
                        "{}: {err}",
                        rec.name
                    );
                    assert!(
                        err.to_string().to_ascii_lowercase().contains(&rec.expect),
                        "{}: {err}",
                        rec.name
                    );
                }
                "error_named" => {
                    let err = named_color(&rec.input).expect_err(&rec.name);
                    assert!(
                        matches!(err, Error::Unsupported { .. }),
                        "{}: {err}",
                        rec.name
                    );
                    assert!(
                        err.to_string().to_ascii_lowercase().contains(&rec.expect),
                        "{}: {err}",
                        rec.name
                    );
                }
                "error_malformed" => {
                    let err = parse_color_spec(&rec.model, &rec.input, None).expect_err(&rec.name);
                    assert!(
                        matches!(err, Error::Malformed { .. }),
                        "{}: {err}",
                        rec.name
                    );
                    assert!(
                        err.to_string().to_ascii_lowercase().contains(&rec.expect),
                        "{}: {err}",
                        rec.name
                    );
                }
                other => panic!("{}: unknown color op {other}", rec.name),
            },
            other => panic!("{}: unknown kind {other}", rec.name),
        }
    }
}
