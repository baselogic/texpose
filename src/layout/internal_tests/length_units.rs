use super::common;

use crate::test_support::{
    layout_with_em_size_pt, parse, Dim, Length, MathFont, MathNode, MathParams, MathStyle,
    SpaceKind,
};

fn hspace_length(source: &str) -> Length {
    match parse(source).expect("parsed length") {
        MathNode::Space(SpaceKind::Hspace(length)) => length,
        other => panic!("expected hspace node, got {}", other.gold()),
    }
}

fn hspace_width(source: &str, font: &MathFont, style: MathStyle, root_em_size_pt: &Dim) -> Dim {
    let ast = parse(source).expect("hspace expression");
    layout_with_em_size_pt(&ast, font, style, root_em_size_pt)
        .expect("hspace layout")
        .width
}

#[test]
fn parsed_lengths_preserve_supported_units() {
    assert_eq!(hspace_length(r"\hspace{2em}"), Length::Em(Dim::from_i64(2)));
    assert_eq!(
        hspace_length(r"\hspace{18mu}"),
        Length::Mu(Dim::from_i64(18))
    );
    assert_eq!(hspace_length(r"\hspace{1pt}"), Length::TexPt(Dim::one()));
    assert_eq!(hspace_length(r"\hspace{1bp}"), Length::BigPt(Dim::one()));
}

#[test]
fn parsed_rule_preserves_both_length_units() {
    let rule = parse(r"\rule{1bp}{2pt}").expect("rule lengths");
    assert_eq!(
        rule,
        MathNode::Rule(Length::BigPt(Dim::one()), Length::TexPt(Dim::from_i64(2)))
    );
}

#[test]
fn physical_lengths_remain_constant_across_root_em_sizes() {
    let font = common::stix_two_math().expect("STIX Two Math");
    let one_bp_in_tex_pt = Dim::ratio(7227, 7200).unwrap();

    for points in [6, 10, 20, 40] {
        let root_em_size_pt = Dim::from_i64(points);
        for style in [MathStyle::Text, MathStyle::Script, MathStyle::ScriptScript] {
            let pt = hspace_width(r"\hspace{1pt}", &font, style, &root_em_size_pt);
            let bp = hspace_width(r"\hspace{1bp}", &font, style, &root_em_size_pt);

            assert_eq!(pt.checked_mul(&root_em_size_pt).unwrap(), Dim::one());
            assert_eq!(bp.checked_mul(&root_em_size_pt).unwrap(), one_bp_in_tex_pt);
        }
    }
}

#[test]
fn em_and_mu_resolve_against_current_math_style() {
    let font = common::stix_two_math().expect("STIX Two Math");
    let params = MathParams::from_font(&font).expect("MATH constants");

    for points in [6, 10, 20, 40] {
        let root_em_size_pt = Dim::from_i64(points);
        for style in [MathStyle::Text, MathStyle::Script, MathStyle::ScriptScript] {
            let em = hspace_width(r"\hspace{1em}", &font, style, &root_em_size_pt);
            let mu = hspace_width(r"\hspace{18mu}", &font, style, &root_em_size_pt);
            let current_quad = params.quad.checked_mul(&params.scale(style)).unwrap();

            assert_eq!(em, params.em(style).unwrap());
            assert_eq!(mu, current_quad);
        }
    }
}

#[test]
fn parsed_rule_height_uses_the_same_physical_length_resolution() {
    let font = common::stix_two_math().expect("STIX Two Math");

    for points in [6, 10, 20, 40] {
        let root_em_size_pt = Dim::from_i64(points);
        let ast = parse(r"\rule{0pt}{1pt}").expect("physical rule");
        let bx = layout_with_em_size_pt(&ast, &font, MathStyle::Script, &root_em_size_pt)
            .expect("physical rule layout");

        assert_eq!(bx.height.checked_mul(&root_em_size_pt).unwrap(), Dim::one());
    }
}

#[test]
fn layout_boundary_rejects_nonpositive_root_em_sizes() {
    let font = common::stix_two_math().expect("STIX Two Math");
    let ast = parse(r"\hspace{1pt}").expect("physical length");

    assert!(layout_with_em_size_pt(&ast, &font, MathStyle::Text, &Dim::zero()).is_err());
    assert!(layout_with_em_size_pt(&ast, &font, MathStyle::Text, &Dim::from_i64(-1)).is_err());
    assert!(layout_with_em_size_pt(
        &ast,
        &font,
        MathStyle::Text,
        &Dim::ratio(1, i64::MAX).unwrap(),
    )
    .is_ok());
}
