mod common;

use texpose::{layout, parse, BoxContent, MathFont, MathStyle};

fn glyph_id(source: &str, style: MathStyle, font: &MathFont) -> u16 {
    let ast = parse(source).expect("parse glyph case");
    let bx = layout(&ast, font, style).expect("layout glyph case");
    let BoxContent::Glyph { glyph_id, .. } = bx.content else {
        panic!("single glyph case must stay a glyph box");
    };
    glyph_id
}

#[test]
fn script_styles_use_open_type_ssty_alternates() {
    let font = common::stix_two_math().expect("STIX Two Math");
    let base = font.glyph('2').expect("digit 2").glyph_id;
    let text = glyph_id("2", MathStyle::Text, &font);
    let script = glyph_id("2", MathStyle::Script, &font);
    let scriptscript = glyph_id("2", MathStyle::ScriptScript, &font);

    assert_eq!(text, base);
    assert_ne!(script, base);
    assert_ne!(scriptscript, base);
}
