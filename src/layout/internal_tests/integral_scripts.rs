use super::{common, vertical_variants};

use core::cmp::Ordering;

use crate::test_support::{
    layout, parse, BoxContent, Dim, MathBox, MathFont, MathParams, MathStyle,
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

fn layout_source(source: &str, style: MathStyle, font: &MathFont) -> MathBox {
    let ast = parse(source).expect("parse integral case");
    layout(&ast, font, style).expect("layout integral case")
}

struct ScriptConstants {
    subscript_top_max: Dim,
    subscript_baseline_drop_min: Dim,
    superscript_bottom_min: Dim,
    superscript_bottom_max_with_subscript: Dim,
    superscript_baseline_drop_max: Dim,
}

fn script_constants(font: &MathFont) -> ScriptConstants {
    let face =
        ttf_parser::Face::parse(font.bytes(), font.face_index()).expect("parse STIX fixture face");
    let constants = face
        .tables()
        .math
        .and_then(|math| math.constants)
        .expect("STIX fixture MATH constants");
    let units_per_em = font.units_per_em();
    let fu = |value: i16| Dim::from_font_units(i64::from(value), units_per_em).expect("font units");

    ScriptConstants {
        subscript_top_max: fu(constants.subscript_top_max().value),
        subscript_baseline_drop_min: fu(constants.subscript_baseline_drop_min().value),
        superscript_bottom_min: fu(constants.superscript_bottom_min().value),
        superscript_bottom_max_with_subscript: fu(constants
            .superscript_bottom_max_with_subscript()
            .value),
        superscript_baseline_drop_max: fu(constants.superscript_baseline_drop_max().value),
    }
}

fn assert_display_variant(
    font: &MathFont,
    params: &MathParams,
    ch: char,
    scale: &Dim,
    base: &MathBox,
) {
    let target = params
        .display_operator_min_height
        .checked_mul(scale)
        .expect("scaled display operator target");
    let expected = vertical_variants::select_by_advance(font, ch, &target, scale);
    assert!(matches!(
        &base.content,
        BoxContent::Glyph { glyph_id, .. } if *glyph_id == expected
    ));
}

fn side_attachment_parts(tree: &MathBox, expected: char) -> (&MathBox, &Dim, &MathBox) {
    let BoxContent::HList(children) = &tree.content else {
        panic!("scripted integral must be an HList");
    };
    let base = children.first().expect("integral base");
    assert!(matches!(
        &base.content,
        BoxContent::Glyph { ch, .. } if *ch == expected
    ));
    let BoxContent::Kern(backtrack) = &children.get(1).expect("operator italic backtrack").content
    else {
        panic!("scripted integral must backtrack the operator italic correction");
    };
    let slot = children
        .iter()
        .find(|child| matches!(&child.content, BoxContent::Overlap(_)))
        .expect("integral script slot");
    (base, backtrack, slot)
}

fn assert_axis_centered(base: &MathBox, params: &MathParams, style: MathStyle) {
    let raw_center = div(&sub(&base.height, &base.depth), &Dim::from_i64(2));
    let axis = mul(&params.axis_height, &params.scale(style));
    assert!(add(&raw_center, &base.shift).eq_dim(&axis));
}

#[test]
fn display_contour_integral_uses_axis_baseline_drop_and_subscript_origin() {
    let font = common::stix_two_math().expect("STIX Two Math fixture");
    let params = MathParams::from_font(&font).expect("OpenType MATH parameters");
    let constants = script_constants(&font);
    let style = MathStyle::Display;
    let scale = params.scale(style);
    let tree = layout_source(r"\oint_\Gamma", style, &font);
    let (base, backtrack, slot) = side_attachment_parts(&tree, '∮');

    assert_display_variant(&font, &params, '∮', &scale, base);
    assert_axis_centered(base, &params, style);
    assert!(
        !base.italic.is_zero(),
        "fixture needs nonzero integral italic correction"
    );
    assert!(backtrack.eq_dim(&(-base.italic.clone())));

    let BoxContent::Overlap(branches) = &slot.content else {
        unreachable!();
    };
    let [lower] = branches.as_slice() else {
        panic!("subscript-only contour integral must contain one lower branch");
    };
    assert!(lower.shift.cmp(&Dim::zero()) == Ordering::Less);

    let lower_plain = layout_source(r"\Gamma", MathStyle::ScriptCramped, &font);
    assert!(lower.width.eq_dim(&lower_plain.width));
    let after = mul(&params.space_after_script, &scale);
    let expected_width = add(
        &sub(&base.width, &base.italic),
        &add(&lower_plain.width, &after),
    );
    assert!(
        tree.width.eq_dim(&expected_width),
        "subscript-only op_noad width must backtrack one italic correction"
    );

    let base_depth = sub(&base.depth, &base.shift).clamp_nonneg();
    let standard = mul(&params.subscript_shift_down, &scale);
    let from_base = add(
        &base_depth,
        &mul(&constants.subscript_baseline_drop_min, &scale),
    );
    let from_top = sub(
        &lower_plain.height,
        &mul(&constants.subscript_top_max, &scale),
    )
    .clamp_nonneg();
    let expected_drop = standard.max_ref(&from_base).max_ref(&from_top);
    assert!((-lower.shift.clone()).eq_dim(&expected_drop));
}

#[test]
fn display_integral_side_scripts_reuse_generic_script_attachment() {
    let font = common::stix_two_math().expect("STIX Two Math fixture");
    let params = MathParams::from_font(&font).expect("OpenType MATH parameters");
    let constants = script_constants(&font);
    let style = MathStyle::Display;
    let scale = params.scale(style);
    let tree = layout_source(r"\int_0^1", style, &font);
    let (base, backtrack, slot) = side_attachment_parts(&tree, '∫');

    assert_display_variant(&font, &params, '∫', &scale, base);
    assert_axis_centered(base, &params, style);
    assert!(
        !base.italic.is_zero(),
        "fixture needs nonzero integral italic correction"
    );
    assert!(backtrack.eq_dim(&(-base.italic.clone())));

    let BoxContent::Overlap(branches) = &slot.content else {
        unreachable!();
    };
    let [upper, lower] = branches.as_slice() else {
        panic!("paired integral scripts must contain upper and lower branches");
    };
    let BoxContent::HList(upper_parts) = &upper.content else {
        panic!("upper script must include the italic-correction lead-in");
    };
    let [upper_kern, upper_body] = upper_parts.as_slice() else {
        panic!("upper script must contain italic kern and body");
    };
    let BoxContent::Kern(upper_lead) = &upper_kern.content else {
        panic!("upper script first child must be a kern");
    };
    assert!(upper_lead.eq_dim(&base.italic));

    let lower_plain = layout_source("0", MathStyle::ScriptCramped, &font);
    assert!(lower.width.eq_dim(&lower_plain.width));
    let expected_slot_width = upper.width.max_ref(&lower.width);
    let after = mul(&params.space_after_script, &scale);
    let expected_width = add(
        &sub(&base.width, &base.italic),
        &add(&expected_slot_width, &after),
    );
    assert!(
        tree.width.eq_dim(&expected_width),
        "TeX op_noad width backtracks one italic correction when a subscript is present"
    );

    let base_height = add(&base.height, &base.shift).clamp_nonneg();
    let base_depth = sub(&base.depth, &base.shift).clamp_nonneg();
    let mut expected_upper = mul(&params.superscript_shift_up, &scale)
        .max_ref(&sub(
            &base_height,
            &mul(&constants.superscript_baseline_drop_max, &scale),
        ))
        .max_ref(&add(
            &upper_body.depth,
            &mul(&constants.superscript_bottom_min, &scale),
        ));
    let mut expected_lower = mul(&params.subscript_shift_down, &scale)
        .max_ref(&add(
            &base_depth,
            &mul(&constants.subscript_baseline_drop_min, &scale),
        ))
        .max_ref(
            &sub(
                &lower_plain.height,
                &mul(&constants.subscript_top_max, &scale),
            )
            .clamp_nonneg(),
        );

    let gap = sub(
        &sub(&add(&expected_upper, &expected_lower), &upper_body.depth),
        &lower_plain.height,
    );
    let min_gap = mul(&params.sub_superscript_gap_min, &scale);
    if gap.cmp(&min_gap) == Ordering::Less {
        expected_lower = add(&expected_lower, &sub(&min_gap, &gap));
    }

    let current_bottom = sub(&expected_upper, &upper_body.depth);
    let target_bottom = mul(&constants.superscript_bottom_max_with_subscript, &scale);
    if current_bottom.cmp(&target_bottom) == Ordering::Less {
        let raise = sub(&target_bottom, &current_bottom);
        let lowered_subscript = sub(&expected_lower, &raise);
        if lowered_subscript.cmp(&Dim::zero()) != Ordering::Less {
            expected_upper = add(&expected_upper, &raise);
            expected_lower = lowered_subscript;
        }
    }

    assert!(upper.shift.eq_dim(&expected_upper));
    assert!((-lower.shift.clone()).eq_dim(&expected_lower));
}

#[test]
fn cramped_script_integral_keeps_operator_box_baseline_drop_after_ssty() {
    let font = common::stix_two_math().expect("STIX Two Math fixture");
    let params = MathParams::from_font(&font).expect("OpenType MATH parameters");
    let constants = script_constants(&font);
    let style = MathStyle::ScriptCramped;
    let scale = params.scale(style);
    let tree = layout_source(r"\int_0^\infty", style, &font);
    let (base, _, slot) = side_attachment_parts(&tree, '∫');

    let selected_gid = match &base.content {
        BoxContent::Glyph { glyph_id, .. } => *glyph_id,
        _ => unreachable!(),
    };
    let cmap_gid = font.glyph('∫').expect("fixture integral glyph").glyph_id;
    assert_ne!(
        selected_gid, cmap_gid,
        "fixture must exercise the level-1 ssty integral alternate"
    );

    let face = font.face();
    let extended_shapes = face
        .tables()
        .math
        .and_then(|math| math.glyph_info)
        .and_then(|info| info.extended_shapes)
        .expect("fixture ExtendedShapeCoverage");
    assert!(
        extended_shapes.get(ttf_parser::GlyphId(cmap_gid)).is_some(),
        "base integral must be an extended shape"
    );
    assert!(
        extended_shapes
            .get(ttf_parser::GlyphId(selected_gid))
            .is_none(),
        "fixture must expose the ssty/ExtendedShape coverage boundary"
    );

    let BoxContent::Overlap(branches) = &slot.content else {
        unreachable!();
    };
    let [upper, lower] = branches.as_slice() else {
        panic!("paired integral scripts must contain upper and lower branches");
    };

    let base_height = add(&base.height, &base.shift).clamp_nonneg();
    let base_depth = sub(&base.depth, &base.shift).clamp_nonneg();
    let standard_upper = mul(&params.superscript_shift_up_cramped, &scale).max_ref(&add(
        &upper.depth,
        &mul(&constants.superscript_bottom_min, &scale),
    ));
    let from_base_upper = sub(
        &base_height,
        &mul(&constants.superscript_baseline_drop_max, &scale),
    )
    .clamp_nonneg();
    let standard_lower = mul(&params.subscript_shift_down, &scale)
        .max_ref(&sub(&lower.height, &mul(&constants.subscript_top_max, &scale)).clamp_nonneg());
    let from_base_lower = add(
        &base_depth,
        &mul(&constants.subscript_baseline_drop_min, &scale),
    );
    assert!(
        from_base_upper > standard_upper,
        "fixture must make the operator baseline-drop constraint observable above"
    );
    assert!(
        from_base_lower > standard_lower,
        "fixture must make the operator baseline-drop constraint observable below"
    );

    let mut expected_upper = standard_upper.max_ref(&from_base_upper);
    let mut expected_lower = standard_lower.max_ref(&from_base_lower);
    let gap = sub(
        &sub(&add(&expected_upper, &expected_lower), &upper.depth),
        &lower.height,
    );
    let min_gap = mul(&params.sub_superscript_gap_min, &scale);
    if gap.cmp(&min_gap) == Ordering::Less {
        expected_lower = add(&expected_lower, &sub(&min_gap, &gap));

        let current_bottom = sub(&expected_upper, &upper.depth);
        let target_bottom = mul(&constants.superscript_bottom_max_with_subscript, &scale);
        if current_bottom.cmp(&target_bottom) == Ordering::Less {
            let raise = sub(&target_bottom, &current_bottom);
            let lowered_subscript = sub(&expected_lower, &raise);
            if lowered_subscript.cmp(&Dim::zero()) != Ordering::Less {
                expected_upper = add(&expected_upper, &raise);
                expected_lower = lowered_subscript;
            }
        }
    }

    assert!(upper.shift.eq_dim(&expected_upper));
    assert!((-lower.shift.clone()).eq_dim(&expected_lower));
}

#[test]
fn text_integral_with_subscript_uses_tex_operator_width_backtrack() {
    let font = common::stix_two_math().expect("STIX Two Math fixture");
    let params = MathParams::from_font(&font).expect("OpenType MATH parameters");
    let style = MathStyle::Text;
    let scale = params.scale(style);
    let tree = layout_source(r"\int_0^1", style, &font);
    let (base, backtrack, slot) = side_attachment_parts(&tree, '∫');

    assert_axis_centered(base, &params, style);
    assert!(!base.italic.is_zero());
    assert!(backtrack.eq_dim(&(-base.italic.clone())));

    let BoxContent::Overlap(branches) = &slot.content else {
        unreachable!();
    };
    let [upper, lower] = branches.as_slice() else {
        panic!("paired integral scripts must contain upper and lower branches");
    };
    let expected_slot_width = upper.width.max_ref(&lower.width);
    let after = mul(&params.space_after_script, &scale);
    let expected_width = add(
        &sub(&base.width, &base.italic),
        &add(&expected_slot_width, &after),
    );
    assert!(tree.width.eq_dim(&expected_width));
}

#[test]
fn top_level_row_propagates_repaired_integral_geometry() {
    let font = common::stix_two_math().expect("STIX Two Math fixture");
    let standalone = layout_source(r"\oint_\Gamma", MathStyle::Display, &font);
    let row = layout_source(r"\oint_\Gamma z", MathStyle::Display, &font);
    let BoxContent::HList(children) = &row.content else {
        panic!("top-level expression must be an HList");
    };

    let integral = children
        .iter()
        .find(|child| {
            let BoxContent::HList(parts) = &child.content else {
                return false;
            };
            parts.first().is_some_and(
                |base| matches!(&base.content, BoxContent::Glyph { ch, .. } if *ch == '∮'),
            )
        })
        .expect("row must retain the scripted contour integral child");

    assert!(integral.width.eq_dim(&standalone.width));
    assert!(integral.height.eq_dim(&standalone.height));
    assert!(integral.depth.eq_dim(&standalone.depth));
    let expected_depth = children
        .iter()
        .fold(Dim::zero(), |depth, child| depth.max_ref(&child.depth));
    assert!(row.depth.eq_dim(&expected_depth));
}

#[test]
fn unscripted_double_integral_uses_display_variant_and_math_axis() {
    let font = common::stix_two_math().expect("STIX Two Math fixture");
    let params = MathParams::from_font(&font).expect("OpenType MATH parameters");
    let style = MathStyle::Display;
    let scale = params.scale(style);
    let tree = layout_source(r"\iint", style, &font);

    assert!(matches!(&tree.content, BoxContent::Glyph { ch, .. } if *ch == '∬'));
    assert_display_variant(&font, &params, '∬', &scale, &tree);
    assert_axis_centered(&tree, &params, style);
    assert!(
        !tree.shift.is_zero(),
        "fixture must distinguish axis centering from retaining the raw glyph baseline"
    );

    let row = layout_source(r"\iint\,", style, &font);
    let expected_height = add(&tree.height, &tree.shift).clamp_nonneg();
    let expected_depth = sub(&tree.depth, &tree.shift).clamp_nonneg();
    assert!(row.height.eq_dim(&expected_height));
    assert!(row.depth.eq_dim(&expected_depth));
}
