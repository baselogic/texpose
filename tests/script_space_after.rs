use latex_rust::{layout, parse, MathFont, MathParams, MathStyle};

fn layout_source(source: &str, style: MathStyle, font: &MathFont) -> latex_rust::MathBox {
    let ast = parse(source).expect("parse math case");
    layout(&ast, font, style).expect("layout math case")
}

#[test]
fn space_after_script_uses_parent_style_scale() {
    let font = MathFont::stix_two_math().expect("embedded STIX Two Math");
    let params = MathParams::from_font(&font).expect("OpenType MATH constants");

    let base = layout_source("i", MathStyle::Text, &font);
    let superscript = layout_source("2", MathStyle::ScriptCramped, &font);
    let scripted = layout_source("i^2", MathStyle::Text, &font);

    let expected_after = &params.space_after_script * &params.scale(MathStyle::Text);
    let expected_width = &(&(&base.width + &base.italic) + &superscript.width) + &expected_after;

    assert!(
        scripted.width.eq_dim(&expected_width),
        "scripted width was {}, expected {} with parent-style SpaceAfterScript",
        scripted.width.to_dec_string(),
        expected_width.to_dec_string()
    );
}
