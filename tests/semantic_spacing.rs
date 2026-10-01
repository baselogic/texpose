use texpose::{layout, parse, Dim, MathFont, MathParams, MathStyle};

fn layout_width(source: &str, style: MathStyle, font: &MathFont) -> Dim {
    let ast = parse(source).expect("semantic-spacing fixture parses");
    layout(&ast, font, style)
        .expect("semantic-spacing fixture lays out")
        .width
}

#[test]
fn explicit_glue_does_not_hide_adjacent_noads_from_binary_spacing() {
    let font = MathFont::stix_two_math().expect("embedded STIX Two Math");
    let params = MathParams::from_font(&font).expect("OpenType MATH constants");

    for style in [
        MathStyle::Display,
        MathStyle::DisplayCramped,
        MathStyle::Text,
        MathStyle::TextCramped,
        MathStyle::Script,
        MathStyle::ScriptCramped,
        MathStyle::ScriptScript,
        MathStyle::ScriptScriptCramped,
    ] {
        let plain = layout_width("a+b", style, &font);
        let explicit = layout_width(r"a\, + b", style, &font);
        let expected_extra = params
            .mu(style)
            .unwrap()
            .checked_mul(&Dim::from_i64(3))
            .unwrap();
        let actual_extra = explicit.checked_sub(&plain).unwrap();

        assert!(
            actual_extra.eq_dim(&expected_extra),
            "{style:?}: explicit thin glue changed binary classification: got {}, expected {}",
            actual_extra.to_dec_string(),
            expected_extra.to_dec_string()
        );
    }
}
