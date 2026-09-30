use latex_rust::{layout, parse, BoxContent, Dim, MathBox, MathFont, MathParams, MathStyle};

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
    let (surd_index, surd) = children
        .iter()
        .enumerate()
        .find(|(_, child)| matches!(child.content, BoxContent::Glyph { ch: '\u{221a}', .. }))
        .expect("radical glyph");
    let BoxContent::Overlap(column) = &children[surd_index + 1].content else {
        panic!("expected radical overlap column");
    };
    let [rule, radicand] = column.as_slice() else {
        panic!("expected radical rule and radicand");
    };
    assert!(matches!(rule.content, BoxContent::Rule));
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
        &params.radical_display_style_vertical_gap * &scale
    } else {
        &params.radical_vertical_gap * &scale
    };
    let thickness = &params.radical_rule_thickness * &scale;
    let radicand_span = &parts.radicand.height + &parts.radicand.depth;
    let surd_span = &parts.surd.height + &parts.surd.depth;
    let distributed = (&(&(&surd_span - &thickness) - &radicand_span) + &gap0) / &Dim::from_i64(2);
    let gap = gap0.max(&distributed);
    let ascent = &parts.radicand.height + &gap + &thickness;
    let descent = (&surd_span - &ascent).clamp_nonneg();
    let shift = &ascent - &parts.surd.height;
    (gap, descent, shift)
}

#[test]
fn radical_variant_slack_is_redistributed_into_gap_and_descent() {
    let font = MathFont::stix_two_math().expect("STIX Two Math");
    let params = MathParams::from_font(&font).expect("MATH constants");
    let style = MathStyle::Display;
    let source = r"\sqrt{x^2+y^2}";
    let ast = parse(source).expect("radical");
    let tree = layout(&ast, &font, style).expect("radical layout");
    let parts = radical_parts(&tree);
    let scale = params.scale(style);
    let gap0 = &params.radical_display_style_vertical_gap * &scale;
    let thickness = &params.radical_rule_thickness * &scale;
    let extra = &params.radical_extra_ascender * &scale;
    let (gap, expected_descent, expected_surd_shift) =
        expected_vertical_geometry(&parts, &params, style);

    assert_ne!(
        gap, gap0,
        "fixture must expose discrete radical-variant slack"
    );
    assert_eq!(parts.surd.shift, expected_surd_shift);
    assert_eq!(parts.rule.shift, &parts.radicand.height + &gap);
    assert_eq!(tree.depth, parts.radicand.depth.max(&expected_descent));
    assert_eq!(
        tree.height,
        &(&parts.radicand.height + &gap + &thickness) + &extra
    );
}

#[test]
fn radical_degree_uses_bottom_of_corrected_surd_span() {
    let font = MathFont::stix_two_math().expect("STIX Two Math");
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
    let surd_span = &parts.surd.height + &parts.surd.depth;
    let pct =
        Dim::from_i64(i64::from(params.radical_degree_bottom_raise_percent)) / Dim::from_i64(100);
    let expected_shift = &(&surd_span * &pct) - &corrected_descent;

    assert_eq!(degree.shift, expected_shift);
    assert_eq!(
        children[0].width,
        &params.radical_kern_before_degree * &params.scale(style)
    );
    assert_eq!(
        children[2].width,
        &params.radical_kern_after_degree * &params.scale(style)
    );
}
