use super::{common, vertical_variants};

use crate::test_support::{layout, parse, BoxContent, MathParams, MathStyle};

fn radical_glyph_id(tree: &crate::layout::MathBox) -> u16 {
    let BoxContent::HList(children) = &tree.content else {
        panic!("expected radical HList");
    };
    children
        .iter()
        .find_map(|child| match &child.content {
            BoxContent::Glyph {
                ch: '√', glyph_id,
            ..
            } => Some(*glyph_id),
            _ => None,
        })
        .expect("radical glyph")
}

#[test]
fn radical_variant_selection_excludes_extra_ascender_from_minimum_span() {
    let font = common::stix_two_math().expect("STIX Two Math");
    let params = MathParams::from_font(&font).expect("MATH constants");
    let style = MathStyle::Display;
    let scale = params.scale(style);

    let radicand = parse(r"x^2+y^2").expect("radicand");
    let radicand_box = layout(&radicand, &font, style.cramp()).expect("radicand layout");
    let gap = params
        .radical_display_style_vertical_gap
        .checked_mul(&scale)
        .unwrap();
    let thickness = params.radical_rule_thickness.checked_mul(&scale).unwrap();
    let extra = params.radical_extra_ascender.checked_mul(&scale).unwrap();

    let needed = radicand_box
        .height
        .checked_add(&radicand_box.depth)
        .unwrap()
        .checked_add(&gap)
        .unwrap()
        .checked_add(&thickness)
        .unwrap();
    let inflated_needed = needed.checked_add(&extra).unwrap();
    let expected = vertical_variants::select_by_advance(&font, '√', &needed, &scale);
    let defective = vertical_variants::select_by_advance(&font, '√', &inflated_needed, &scale);
    assert_ne!(
        expected, defective,
        "fixture must cross a radical variant boundary only because of RadicalExtraAscender"
    );

    let ast = parse(r"\sqrt{x^2+y^2}").expect("radical");
    let tree = layout(&ast, &font, style).expect("radical layout");
    assert_eq!(radical_glyph_id(&tree), expected);
}
