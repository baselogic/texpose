//! Gold runner: `golds/layout.toml` is the layout-dimension contract.

use super::{common, gold_support};

use crate::test_support::{layout, parse, BoxContent, Color, MathBox, MathFont, MathStyle};

struct Rec {
    name: String,
    kind: String,
    style: String,
    input: String,
    expect: String,
}

fn load_golds() -> Vec<Rec> {
    gold_support::load("golds/layout.toml")
        .into_iter()
        .map(|gold| {
            match gold.kind() {
                "dims" | "color" | "back_color" | "frame_stroke" | "color_nested" | "err" => {
                    gold.expect_fields(&["name", "kind", "input", "expect"], &["style"]);
                }
                "style_frac" => {
                    gold.expect_fields(&["name", "kind", "style", "expect"], &[]);
                }
                other => panic!("{}: unknown kind {other}", gold.name()),
            }
            Rec {
                name: gold.name().to_string(),
                kind: gold.kind().to_string(),
                style: gold.optional("style").unwrap_or("").to_string(),
                input: gold.optional("input").unwrap_or("").to_string(),
                expect: gold.required("expect").to_string(),
            }
        })
        .collect()
}

fn parse_style(s: &str) -> MathStyle {
    match s {
        "display" => MathStyle::Display,
        "display-cramped" => MathStyle::DisplayCramped,
        "text" => MathStyle::Text,
        "text-cramped" => MathStyle::TextCramped,
        "script" => MathStyle::Script,
        "script-cramped" => MathStyle::ScriptCramped,
        "scriptscript" => MathStyle::ScriptScript,
        "scriptscript-cramped" => MathStyle::ScriptScriptCramped,
        other => panic!("unknown style {other}"),
    }
}

fn first_color(b: &MathBox) -> Option<Color> {
    match &b.content {
        BoxContent::Color(c, inner) => first_color(inner).or(Some(*c)),
        BoxContent::BackColor(_, inner)
        | BoxContent::Frame { inner, .. }
        | BoxContent::PaintCopies { inner, .. } => first_color(inner),
        BoxContent::HList(v) | BoxContent::VList(v) | BoxContent::Overlap(v) => {
            v.iter().find_map(first_color)
        }
        _ => None,
    }
}

fn first_back_color(b: &MathBox) -> Option<Color> {
    match &b.content {
        BoxContent::BackColor(c, _) => Some(*c),
        BoxContent::Color(_, inner)
        | BoxContent::Frame { inner, .. }
        | BoxContent::PaintCopies { inner, .. } => first_back_color(inner),
        BoxContent::HList(v) | BoxContent::VList(v) | BoxContent::Overlap(v) => {
            v.iter().find_map(first_back_color)
        }
        _ => None,
    }
}

fn first_frame_stroke(b: &MathBox) -> Option<Color> {
    match &b.content {
        BoxContent::Frame { stroke, inner, .. } => stroke.or_else(|| first_frame_stroke(inner)),
        BoxContent::Color(_, inner)
        | BoxContent::BackColor(_, inner)
        | BoxContent::PaintCopies { inner, .. } => first_frame_stroke(inner),
        BoxContent::HList(v) | BoxContent::VList(v) | BoxContent::Overlap(v) => {
            v.iter().find_map(first_frame_stroke)
        }
        _ => None,
    }
}

fn has_nested_color(b: &MathBox, outer: Color, inner: Color) -> bool {
    match &b.content {
        BoxContent::Color(c, body) if *c == outer => first_color(body) == Some(inner),
        BoxContent::Color(_, body) => has_nested_color(body, outer, inner),
        BoxContent::BackColor(_, body)
        | BoxContent::Frame { inner: body, .. }
        | BoxContent::PaintCopies { inner: body, .. } => has_nested_color(body, outer, inner),
        BoxContent::HList(v) | BoxContent::VList(v) | BoxContent::Overlap(v) => {
            v.iter().any(|k| has_nested_color(k, outer, inner))
        }
        _ => false,
    }
}

fn parse_expected_hex_color(value: &str, name: &str) -> Color {
    let hex = value
        .strip_prefix('#')
        .filter(|hex| hex.len() == 6)
        .unwrap_or_else(|| panic!("{name}: expected #rrggbb color, got {value:?}"));
    let rgb = u32::from_str_radix(hex, 16)
        .unwrap_or_else(|_| panic!("{name}: invalid #rrggbb color {value:?}"));
    Color::rgb((rgb >> 16) as u8, (rgb >> 8) as u8, rgb as u8)
}

fn expected_nested_colors(expect: &str, name: &str) -> (Color, Color) {
    let (outer, inner) = expect.split_once("-then-").unwrap_or_else(|| {
        panic!("{name}: expected color pair #rrggbb-then-#rrggbb, got {expect:?}")
    });
    (
        parse_expected_hex_color(outer, name),
        parse_expected_hex_color(inner, name),
    )
}

fn lay(font: &MathFont, rec: &Rec) -> MathBox {
    let ast = parse(&rec.input).unwrap_or_else(|e| panic!("{}: parse {e}", rec.name));
    let style = if rec.style.is_empty() {
        MathStyle::Text
    } else {
        parse_style(&rec.style)
    };
    layout(&ast, font, style).unwrap_or_else(|e| panic!("{}: layout {e}", rec.name))
}

#[test]
fn layout_golds() {
    let recs = load_golds();
    assert!(!recs.is_empty(), "no layout golds loaded");
    let font = common::stix_two_math().expect("STIX Two Math");
    for rec in recs {
        match rec.kind.as_str() {
            "dims" => {
                let bx = lay(&font, &rec);
                assert_eq!(bx.dim_gold(), rec.expect, "{}", rec.name);
            }
            "color" => {
                let bx = lay(&font, &rec);
                let c = first_color(&bx).unwrap_or_else(|| panic!("{}: no color box", rec.name));
                assert_eq!(c.css_hex(), rec.expect, "{}", rec.name);
            }
            "back_color" => {
                let bx = lay(&font, &rec);
                let c = first_back_color(&bx)
                    .unwrap_or_else(|| panic!("{}: no background color", rec.name));
                assert_eq!(c.css_hex(), rec.expect, "{}", rec.name);
            }
            "frame_stroke" => {
                let bx = lay(&font, &rec);
                let c = first_frame_stroke(&bx)
                    .unwrap_or_else(|| panic!("{}: no frame stroke", rec.name));
                assert_eq!(c.css_hex(), rec.expect, "{}", rec.name);
            }
            "color_nested" => {
                let bx = lay(&font, &rec);
                let (outer, inner) = expected_nested_colors(&rec.expect, &rec.name);
                assert!(
                    has_nested_color(&bx, outer, inner),
                    "{}: nested color {} not found",
                    rec.name,
                    rec.expect
                );
            }
            "err" => {
                let ast = parse(&rec.input).unwrap_or_else(|e| panic!("{}: parse {e}", rec.name));
                let style = parse_style(if rec.style.is_empty() {
                    "text"
                } else {
                    &rec.style
                });
                let err = layout(&ast, &font, style).expect_err(&rec.name);
                assert!(err.to_string().contains(&rec.expect), "{}: {err}", rec.name);
            }
            "style_frac" => {
                let outer = parse_style(&rec.style);
                let parts: Vec<&str> = rec.expect.split(' ').collect();
                assert_eq!(parts.len(), 2, "{}", rec.name);
                assert_eq!(outer.numerator().gold(), parts[0], "{}", rec.name);
                assert_eq!(outer.denominator().gold(), parts[1], "{}", rec.name);
            }
            other => panic!("{}: unknown kind {other}", rec.name),
        }
    }
}
