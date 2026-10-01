//! Script and scriptscript glyphs carry the OpenType MATH style scale in layout.

mod common;

use texpose::{layout, parse, BoxContent, Dim, MathBox, MathStyle};

fn glyphs(b: &MathBox, out: &mut Vec<(char, Dim)>) {
    match &b.content {
        BoxContent::Glyph { ch, scale, .. } => out.push((*ch, scale.clone())),
        BoxContent::HList(v) | BoxContent::VList(v) | BoxContent::Overlap(v) => {
            v.iter().for_each(|k| glyphs(k, out));
        }
        BoxContent::Color(_, k) | BoxContent::BackColor(_, k) => glyphs(k, out),
        _ => {}
    }
}

fn scales(latex: &str) -> Vec<(char, Dim)> {
    let font = common::stix_two_math().expect("STIX Two Math");
    let bx = layout(&parse(latex).expect("parse"), &font, MathStyle::Text).expect("layout");
    let mut out = Vec::new();
    glyphs(&bx, &mut out);
    out
}

#[test]
fn layout_records_script_and_scriptscript_scale() {
    let s = scales("x^{2^3}");
    assert_eq!(s.len(), 3);
    assert_eq!(s[0].1, Dim::one(), "base at text size");
    assert_eq!(
        s[1].1,
        Dim::ratio(7, 10).expect("script scale"),
        "STIX ScriptPercentScaleDown = 70"
    );
    assert_eq!(
        s[2].1,
        Dim::ratio(11, 20).expect("scriptscript scale"),
        "STIX ScriptScriptPercentScaleDown = 55"
    );
}
