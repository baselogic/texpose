mod common;

use texpose::{
    layout, layout_with_em_size_pt_and_diagnostics, parse, BoxContent, Dim, MathBox, MathParams,
    MathStyle,
};

fn add(a: &Dim, b: &Dim) -> Dim {
    a.checked_add(b).unwrap()
}
fn sub(a: &Dim, b: &Dim) -> Dim {
    a.checked_sub(b).unwrap()
}
fn mul(a: &Dim, b: &Dim) -> Dim {
    a.checked_mul(b).unwrap()
}
fn div(a: &Dim, b: &Dim) -> Dim {
    a.checked_div(b).unwrap()
}

struct RadicalParts<'a> {
    surd_index: usize,
    surd: &'a MathBox,
    rule: &'a MathBox,
    radicand: &'a MathBox,
}

fn radical_parts(tree: &MathBox) -> RadicalParts<'_> {
    let BoxContent::HList(children) = &tree.content else {
        panic!("expected radical HList");
    };
    let column_index = children
        .iter()
        .position(|child| {
            let BoxContent::Overlap(column) = &child.content else {
                return false;
            };
            matches!(
                column.first().map(|part| &part.content),
                Some(BoxContent::Rule)
            )
        })
        .expect("radical rule/radicand column");
    let surd_index = column_index
        .checked_sub(1)
        .expect("surd before radical column");
    let surd = &children[surd_index];
    let BoxContent::Overlap(column) = &children[column_index].content else {
        unreachable!();
    };
    let [rule, radicand] = column.as_slice() else {
        panic!("expected radical rule and radicand");
    };
    RadicalParts {
        surd_index,
        surd,
        rule,
        radicand,
    }
}

fn expected_vertical_geometry(
    parts: &RadicalParts<'_>,
    params: &MathParams,
    style: MathStyle,
) -> (Dim, Dim, Dim) {
    let scale = params.scale(style);
    let gap0 = if style.is_display() {
        mul(&params.radical_display_style_vertical_gap, &scale)
    } else {
        mul(&params.radical_vertical_gap, &scale)
    };
    let thickness = mul(&params.radical_rule_thickness, &scale);
    let radicand_span = add(&parts.radicand.height, &parts.radicand.depth);
    let surd_span = add(&parts.surd.height, &parts.surd.depth);
    let distributed = div(
        &add(&sub(&sub(&surd_span, &thickness), &radicand_span), &gap0),
        &Dim::from_i64(2),
    );
    let gap = gap0.max_ref(&distributed);
    let ascent = add(&add(&parts.radicand.height, &gap), &thickness);
    let descent = sub(&surd_span, &ascent).clamp_nonneg();
    let shift = sub(&ascent, &parts.surd.height);
    (gap, descent, shift)
}

#[test]
fn radical_variant_slack_is_redistributed_into_gap_and_descent() {
    let font = common::stix_two_math().expect("STIX Two Math");
    let params = MathParams::from_font(&font).expect("MATH constants");
    let style = MathStyle::Display;
    let source = r"\sqrt{x^2+y^2}";
    let ast = parse(source).expect("radical");
    let tree = layout(&ast, &font, style).expect("radical layout");
    let parts = radical_parts(&tree);
    let scale = params.scale(style);
    let gap0 = mul(&params.radical_display_style_vertical_gap, &scale);
    let thickness = mul(&params.radical_rule_thickness, &scale);
    let extra = mul(&params.radical_extra_ascender, &scale);
    let (gap, expected_descent, expected_surd_shift) =
        expected_vertical_geometry(&parts, &params, style);

    assert_ne!(
        gap, gap0,
        "fixture must expose discrete radical-variant slack"
    );
    assert_eq!(parts.surd.shift, expected_surd_shift);
    assert_eq!(parts.rule.shift, add(&parts.radicand.height, &gap));
    assert_eq!(tree.depth, parts.radicand.depth.max_ref(&expected_descent));
    assert_eq!(
        tree.height,
        add(&add(&add(&parts.radicand.height, &gap), &thickness), &extra)
    );
}

#[test]
fn radical_degree_uses_bottom_of_corrected_surd_span() {
    let font = common::stix_two_math().expect("STIX Two Math");
    let params = MathParams::from_font(&font).expect("MATH constants");
    let style = MathStyle::Display;
    let source = r"\sqrt[\frac{1+\alpha}{2}]{x}";
    let ast = parse(source).expect("indexed radical");
    let tree = layout(&ast, &font, style).expect("indexed radical layout");
    let parts = radical_parts(&tree);
    let BoxContent::HList(children) = &tree.content else {
        unreachable!();
    };
    assert_eq!(parts.surd_index, 3, "expected indexed radical topology");
    let degree = &children[1];
    let (_, corrected_descent, _) = expected_vertical_geometry(&parts, &params, style);
    let surd_span = add(&parts.surd.height, &parts.surd.depth);
    let pct = div(
        &Dim::from_i64(i64::from(params.radical_degree_bottom_raise_percent)),
        &Dim::from_i64(100),
    );
    let expected_shift = sub(&mul(&surd_span, &pct), &corrected_descent);

    assert_eq!(degree.shift, expected_shift);
    assert_eq!(
        children[0].width,
        mul(&params.radical_kern_before_degree, &params.scale(style))
    );
    assert_eq!(
        children[2].width,
        mul(&params.radical_kern_after_degree, &params.scale(style))
    );
}

#[test]
fn radical_bar_preserves_direct_radicand_italic_extent_once() {
    let font = common::stix_two_math().expect("STIX Two Math");
    let style = MathStyle::Display;
    let tree = layout(&parse(r"\sqrt{x}").expect("radical"), &font, style).expect("radical layout");
    let parts = radical_parts(&tree);

    assert!(
        parts.radicand.italic > Dim::zero(),
        "fixture must expose a nonzero terminal MATH italic correction"
    );
    let expected_rule_width = parts
        .radicand
        .width
        .checked_add(&parts.radicand.italic)
        .unwrap();
    assert_eq!(parts.rule.width, expected_rule_width);
    assert_eq!(
        tree.width,
        parts.surd.width.checked_add(&expected_rule_width).unwrap()
    );
}

#[test]
fn narrow_degree_clamps_negative_after_kern_before_the_surd_origin() {
    let font = common::stix_two_math().expect("STIX Two Math");
    let params = MathParams::from_font(&font).expect("MATH constants");
    let style = MathStyle::Display;
    let tree = layout(
        &parse(r"\sqrt[i]{x}").expect("indexed radical"),
        &font,
        style,
    )
    .expect("indexed radical layout");
    let BoxContent::HList(children) = &tree.content else {
        panic!("expected indexed radical HList");
    };
    let parts = radical_parts(&tree);
    assert_eq!(parts.surd_index, 3, "expected degree/before/after topology");

    let before = &children[0];
    let degree = &children[1];
    let after = &children[2];
    let configured_after = params
        .radical_kern_after_degree
        .checked_mul(&params.scale(style))
        .unwrap();
    let prefix = before.width.checked_add(&degree.width).unwrap();
    let minimum_after = -prefix.clone();
    assert!(
        configured_after < minimum_after,
        "fixture must require the LuaTeX narrow-degree clamp"
    );
    assert_eq!(after.width, minimum_after);
    assert_eq!(
        before
            .width
            .checked_add(&degree.width)
            .unwrap()
            .checked_add(&after.width)
            .unwrap(),
        Dim::zero(),
        "degree prefix must not pull the radical sign left of its origin"
    );
}

#[test]
fn radical_surd_and_rule_geometry_is_root_em_invariant_across_required_sweep() {
    let font = common::stix_two_math().expect("STIX Two Math");
    let ast = parse(r"\sqrt[\frac{1+\alpha}{2}]{\rule{0pt}{5em}x}").expect("tall indexed radical");
    let mut baseline = None;

    for size in [6, 10, 20, 40] {
        let output = layout_with_em_size_pt_and_diagnostics(
            &ast,
            &font,
            MathStyle::Display,
            &Dim::from_i64(size),
        )
        .expect("radical sweep layout");
        assert!(
            output.diagnostics.is_empty(),
            "valid radical construction must not degrade at {size}pt: {:?}",
            output.diagnostics
        );
        let parts = radical_parts(&output.math_box);
        assert!(
            matches!(parts.surd.content, BoxContent::Overlap(_)),
            "fixture must exercise a vertical radical assembly at {size}pt"
        );
        let geometry = (
            parts.surd.width.clone(),
            parts.surd.height.clone(),
            parts.surd.depth.clone(),
            parts.surd.shift.clone(),
            parts.rule.width.clone(),
            parts.rule.height.clone(),
            parts.rule.shift.clone(),
        );
        if let Some(expected) = &baseline {
            assert_eq!(
                &geometry, expected,
                "normalized radical geometry at {size}pt"
            );
        } else {
            baseline = Some(geometry);
        }
    }
}
