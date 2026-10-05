mod common;
#[path = "common/vertical_variants.rs"]
mod vertical_variants;

use texpose::{layout, parse, BoxContent, Dim, MathBox, MathFont, MathParams, MathStyle};

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

fn layout_source(source: &str, style: MathStyle, font: &MathFont) -> MathBox {
    layout(&parse(source).expect("parse operator case"), font, style).expect("layout operator case")
}

fn centered_operator_glyph_id(base: &MathBox, expected_ch: char) -> u16 {
    match &base.content {
        BoxContent::Glyph { ch, glyph_id, .. } => {
            assert_eq!(*ch, expected_ch);
            *glyph_id
        }
        BoxContent::HList(children) => {
            let mut glyphs = children.iter().filter_map(|child| match &child.content {
                BoxContent::Glyph { ch, glyph_id, .. } => Some((*ch, *glyph_id)),
                BoxContent::Kern(_) => None,
                _ => panic!("unexpected centered large-operator child"),
            });
            let (ch, glyph_id) = glyphs.next().expect("centered large-operator glyph");
            assert_eq!(ch, expected_ch);
            assert!(
                glyphs.next().is_none(),
                "centered operator must contain one glyph"
            );
            glyph_id
        }
        _ => panic!("unexpected large-operator base content"),
    }
}

fn find_glyph_box(tree: &MathBox, expected_ch: char) -> Option<&MathBox> {
    match &tree.content {
        BoxContent::Glyph { ch, .. } if *ch == expected_ch => Some(tree),
        BoxContent::HList(children)
        | BoxContent::VList(children)
        | BoxContent::Overlap(children) => children
            .iter()
            .find_map(|child| find_glyph_box(child, expected_ch)),
        BoxContent::Color(_, inner)
        | BoxContent::BackColor(_, inner)
        | BoxContent::Frame { inner, .. } => find_glyph_box(inner, expected_ch),
        _ => None,
    }
}

fn assert_axis_centered(glyph: &MathBox, params: &MathParams, style: MathStyle, label: &str) {
    let axis = mul(&params.axis_height, &params.scale(style));
    let raw_center = div(&sub(&glyph.height, &glyph.depth), &Dim::from_i64(2));
    let actual = add(&raw_center, &glyph.shift);
    assert!(
        actual.eq_dim(&axis),
        "{label}: glyph center {} != axis {}",
        actual.to_dec_string(),
        axis.to_dec_string()
    );
}

fn limit_branches(tree: &MathBox) -> (&MathBox, &MathBox, &MathBox) {
    let BoxContent::Overlap(branches) = &tree.content else {
        panic!("display sum with limits must be an overlap");
    };
    let [upper, base, lower] = branches.as_slice() else {
        panic!("display sum must contain upper limit, base, and lower limit");
    };
    (base, upper, lower)
}

fn leading_kern(branch: &MathBox) -> &Dim {
    let BoxContent::HList(children) = &branch.content else {
        panic!("offset limit branch must be horizontally packed");
    };
    let Some(first) = children.first() else {
        panic!("offset limit branch is empty");
    };
    let BoxContent::Kern(width) = &first.content else {
        panic!("offset limit branch must begin with its left padding kern");
    };
    width
}

#[test]
fn display_large_operator_uses_math_axis_and_independent_limit_constraints() {
    let font = common::stix_two_math().expect("STIX Two Math");
    let params = MathParams::from_font(&font).expect("MATH constants");
    let style = MathStyle::Display;
    let scale = params.scale(style);
    let axis = mul(&params.axis_height, &scale);

    let ast = parse(r"\sum_{i=1}^{n}").expect("display sum");
    let tree = layout(&ast, &font, style).expect("display sum layout");
    let (base, upper, lower) = limit_branches(&tree);

    let target = mul(&params.display_operator_min_height, &scale);
    let expected_glyph = vertical_variants::select_by_advance(&font, '∑', &target, &scale);
    assert_eq!(centered_operator_glyph_id(base, '∑'), expected_glyph);

    let raw_center = div(&sub(&base.height, &base.depth), &Dim::from_i64(2));
    let expected_base_shift = sub(&axis, &raw_center);
    assert!(
        !expected_base_shift.is_zero(),
        "fixture must expose the raw-baseline large-operator bug"
    );
    assert!(base.shift.eq_dim(&expected_base_shift));
    let effective_center = add(&raw_center, &base.shift);
    assert!(effective_center.eq_dim(&axis));

    let base_ascent = add(&base.height, &base.shift).clamp_nonneg();
    let base_descent = sub(&base.depth, &base.shift).clamp_nonneg();

    let upper_gap = mul(&params.upper_limit_gap_min, &scale);
    let upper_rise = mul(&params.upper_limit_baseline_rise_min, &scale);
    let expected_upper_offset = upper_rise.max_ref(&add(&upper_gap, &upper.depth));
    let expected_upper_shift = add(&base_ascent, &expected_upper_offset);
    let legacy_upper_shift = add(
        &add(&base_ascent, &upper_gap.max_ref(&upper_rise)),
        &upper.depth,
    );
    assert!(
        !expected_upper_shift.eq_dim(&legacy_upper_shift),
        "fixture must distinguish independent upper constraints from max-then-add"
    );
    assert!(upper.shift.eq_dim(&expected_upper_shift));

    let lower_gap = mul(&params.lower_limit_gap_min, &scale);
    let lower_drop = mul(&params.lower_limit_baseline_drop_min, &scale);
    let expected_lower_offset = lower_drop.max_ref(&add(&lower_gap, &lower.height));
    let expected_lower_drop = add(&base_descent, &expected_lower_offset);
    let legacy_lower_drop = add(
        &add(&base_descent, &lower_gap.max_ref(&lower_drop)),
        &lower.height,
    );
    assert!(
        !expected_lower_drop.eq_dim(&legacy_lower_drop),
        "fixture must distinguish independent lower constraints from max-then-add"
    );
    let actual_lower_drop = -lower.shift.clone();
    assert!(actual_lower_drop.eq_dim(&expected_lower_drop));

    let expected_height = base_ascent.max_ref(&add(&upper.shift, &upper.height));
    let expected_depth = base_descent.max_ref(&add(&expected_lower_drop, &lower.depth));
    assert!(tree.height.eq_dim(&expected_height));
    assert!(tree.depth.eq_dim(&expected_depth));
}

#[test]
fn nary_operator_family_uses_display_variants_and_math_axis() {
    let font = common::stix_two_math().expect("STIX Two Math");
    let params = MathParams::from_font(&font).expect("MATH constants");
    let scale = params.scale(MathStyle::Display);
    let target = mul(&params.display_operator_min_height, &scale);

    for (source, ch) in [
        (r"\sum\nolimits", '∑'),
        (r"\prod", '∏'),
        (r"\int", '∫'),
        (r"\iint", '∬'),
        (r"\iiint", '∭'),
        (r"\oint", '∮'),
        (r"\oiint", '∯'),
    ] {
        let tree = layout_source(source, MathStyle::Display, &font);
        let glyph = find_glyph_box(&tree, ch).unwrap_or_else(|| panic!("{source}: operator glyph"));
        let expected = vertical_variants::select_by_advance(&font, ch, &target, &scale);
        let BoxContent::Glyph { glyph_id, .. } = &glyph.content else {
            unreachable!("find_glyph_box returned a glyph")
        };
        assert_eq!(*glyph_id, expected, "{source}: display variant");
        assert_axis_centered(glyph, &params, MathStyle::Display, source);
    }

    for (source, ch) in [(r"\sum\nolimits", '∑'), (r"\int", '∫')] {
        let tree = layout_source(source, MathStyle::Text, &font);
        let glyph = find_glyph_box(&tree, ch).unwrap_or_else(|| panic!("{source}: text glyph"));
        assert_eq!(
            centered_operator_glyph_id(glyph, ch),
            font.glyph(ch).expect("base operator glyph").glyph_id,
            "{source}: text style keeps the base glyph"
        );
        assert_axis_centered(glyph, &params, MathStyle::Text, source);
    }
}

#[test]
fn integral_limits_use_half_italic_correction_in_opposite_directions() {
    let font = common::stix_two_math().expect("STIX Two Math");
    let params = MathParams::from_font(&font).expect("MATH constants");
    let style = MathStyle::Text;
    let scale = params.scale(style);

    let nucleus = layout_source(r"\int", style, &font);
    let BoxContent::Glyph {
        glyph_id: nucleus_gid,
        ..
    } = nucleus.content
    else {
        panic!("text integral nucleus must stay a glyph");
    };
    let italic = font
        .italic_correction(nucleus_gid)
        .checked_mul(&scale)
        .expect("scaled italic correction");
    assert!(
        !italic.is_zero(),
        "fixture must expose the half-italic rule"
    );

    let upper_plain = layout_source("0", MathStyle::Script, &font);
    let lower_plain = layout_source("0", MathStyle::ScriptCramped, &font);
    let tree = layout_source(r"\int\limits_0^0", style, &font);
    let (base, upper, lower) = limit_branches(&tree);
    assert_eq!(centered_operator_glyph_id(base, '∫'), nucleus_gid);

    let expected_width = nucleus
        .width
        .max_ref(&upper_plain.width)
        .max_ref(&lower_plain.width);
    assert!(tree.width.eq_dim(&expected_width));

    let half_italic = div(&italic, &Dim::from_i64(2));
    let expected_upper_left = div(&sub(&tree.width, &upper_plain.width), &Dim::from_i64(2))
        .checked_add(&half_italic)
        .unwrap();
    let expected_lower_left = div(&sub(&tree.width, &lower_plain.width), &Dim::from_i64(2))
        .checked_sub(&half_italic)
        .unwrap();

    assert_eq!(
        upper_plain.width, lower_plain.width,
        "fixture needs equal-width digit limits to isolate italic correction"
    );
    assert!(leading_kern(upper).eq_dim(&expected_upper_left));
    assert!(leading_kern(lower).eq_dim(&expected_lower_left));
    assert!(
        sub(leading_kern(upper), leading_kern(lower)).eq_dim(&italic),
        "opposite half-italic offsets must separate equal-width limits by one full italic correction"
    );
}

#[test]
fn wide_substack_limit_centers_the_operator_nucleus_exactly_once() {
    let font = common::stix_two_math().expect("STIX Two Math");
    let params = MathParams::from_font(&font).expect("MATH constants");
    let tree = layout_source(
        r"\sum_{\substack{1\le i\le n\\1\le j\le m\\i\ne j}}",
        MathStyle::Display,
        &font,
    );

    let BoxContent::Overlap(branches) = &tree.content else {
        panic!("display sum with substack must use limits");
    };
    let [base, _lower] = branches.as_slice() else {
        panic!("expected operator and lower substack");
    };
    let BoxContent::HList(parts) = &base.content else {
        panic!("wide substack must force horizontal centering around the operator");
    };
    let glyph = parts
        .iter()
        .find(|part| matches!(&part.content, BoxContent::Glyph { ch, .. } if *ch == '∑'))
        .expect("centered sum glyph");
    assert!(
        glyph.shift.is_zero(),
        "the axis shift belongs to the centered operator wrapper, not both wrapper and glyph"
    );

    let axis = mul(&params.axis_height, &params.scale(MathStyle::Display));
    let raw_center = div(&sub(&glyph.height, &glyph.depth), &Dim::from_i64(2));
    assert!(add(&raw_center, &base.shift).eq_dim(&axis));
}

#[test]
fn nested_nary_operators_preserve_their_script_style_axis() {
    let font = common::stix_two_math().expect("STIX Two Math");
    let params = MathParams::from_font(&font).expect("MATH constants");
    let tree = layout_source(r"\sum_{\int}^{\prod_1^n}", MathStyle::Display, &font);

    let inner_integral = find_glyph_box(&tree, '∫').expect("nested integral");
    let inner_product = find_glyph_box(&tree, '∏').expect("nested product");
    assert_axis_centered(
        inner_integral,
        &params,
        MathStyle::ScriptCramped,
        "nested integral",
    );
    assert_axis_centered(inner_product, &params, MathStyle::Script, "nested product");
}

fn is_limit_overlap(tree: &MathBox) -> bool {
    matches!(&tree.content, BoxContent::Overlap(_))
}

#[test]
fn explicit_limit_controls_override_operator_defaults() {
    let font = common::stix_two_math().expect("STIX Two Math");

    let forced_sum = layout(
        &parse(r"\sum\limits_1^n").expect("forced sum limits"),
        &font,
        MathStyle::Text,
    )
    .expect("forced sum layout");
    assert!(is_limit_overlap(&forced_sum));

    let side_sum = layout(
        &parse(r"\sum\nolimits_1^n").expect("sum nolimits"),
        &font,
        MathStyle::Display,
    )
    .expect("sum nolimits layout");
    assert!(matches!(&side_sum.content, BoxContent::HList(_)));

    let forced_integral = layout(
        &parse(r"\int\limits_0^1").expect("forced integral limits"),
        &font,
        MathStyle::Text,
    )
    .expect("forced integral layout");
    assert!(is_limit_overlap(&forced_integral));

    let default_integral = layout(
        &parse(r"\int_0^1").expect("default integral limits"),
        &font,
        MathStyle::Display,
    )
    .expect("default integral layout");
    assert!(matches!(&default_integral.content, BoxContent::HList(_)));

    let named = layout(
        &parse(r"\operatorname{lim}\limits_x").expect("named operator limits"),
        &font,
        MathStyle::Text,
    )
    .expect("named operator limits layout");
    assert!(is_limit_overlap(&named));
}
