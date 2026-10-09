use super::{common, vertical_variants};

use crate::test_support::{
    layout_with_em_size_pt, parse, BoxContent, Dim, MathBox, MathFont, MathParams, MathStyle,
};

const STIX: &[u8] = include_bytes!("../../../fonts/stix-two-math/STIXTwoMath-Regular.otf");
const LIBERTINUS: &[u8] =
    include_bytes!("../../../fonts/libertinus-math/LibertinusMath-Regular.otf");
const FIRA: &[u8] = include_bytes!("../../../fonts/fira-math/FiraMath-Regular.otf");

fn profile_fonts() -> [(&'static str, MathFont); 3] {
    [
        ("stix", MathFont::from_bytes(STIX).expect("STIX Two Math")),
        (
            "libertinus",
            MathFont::from_bytes(LIBERTINUS).expect("Libertinus Math"),
        ),
        ("fira", MathFont::from_bytes(FIRA).expect("Fira Math")),
    ]
}

fn tex_delimiter_target(max_distance: &Dim, em_size_pt: &Dim) -> Dim {
    let factor_target = max_distance
        .checked_mul(&Dim::ratio(901, 500).unwrap())
        .unwrap();
    let shortfall = Dim::from_i64(5).checked_div(em_size_pt).unwrap();
    let shortfall_target = max_distance
        .checked_mul(&Dim::from_i64(2))
        .unwrap()
        .checked_sub(&shortfall)
        .unwrap()
        .clamp_nonneg();
    factor_target.max_ref(&shortfall_target)
}

fn delimited_children(tree: &MathBox) -> (&MathBox, &MathBox, &MathBox) {
    let BoxContent::HList(children) = &tree.content else {
        panic!("expected delimited HList");
    };
    let [left, body, right] = children.as_slice() else {
        panic!("expected left/body/right delimiter branches");
    };
    (left, body, right)
}

#[test]
fn left_right_null_delimiters_keep_physical_nulldelimiterspace() {
    for (name, font) in profile_fonts() {
        for em_size_pt in [6_i64, 10, 20, 40] {
            let em_size = Dim::from_i64(em_size_pt);
            let expected = Dim::ratio(6, 5)
                .expect("1.2pt")
                .checked_div(&em_size)
                .expect("positive root em");

            for (source, left_is_null) in [(r"\left.a\right|", true), (r"\left|a\right.", false)] {
                let ast = parse(source).expect("null-delimiter expression");
                let tree = layout_with_em_size_pt(&ast, &font, MathStyle::Display, &em_size)
                    .unwrap_or_else(|error| panic!("{name} {em_size_pt}pt {source}: {error}"));
                let (left, _, right) = delimited_children(&tree);
                let null_side = if left_is_null { left } else { right };
                let BoxContent::Kern(width) = &null_side.content else {
                    panic!("{name} {em_size_pt}pt {source}: null delimiter must be a kern");
                };

                assert!(
                    width.eq_dim(&expected),
                    "{name} {em_size_pt}pt {source}: null delimiter {} != {}",
                    width.to_dec_string(),
                    expected.to_dec_string()
                );
            }
        }
    }
}

fn glyph_id(bx: &MathBox, expected_ch: char) -> u16 {
    let BoxContent::Glyph { ch, glyph_id, .. } = &bx.content else {
        panic!("expected delimiter glyph");
    };
    assert_eq!(*ch, expected_ch);
    *glyph_id
}

#[test]
fn libertinus_widehat_body_drives_delimiter_variants_by_g2_target() {
    let font = MathFont::from_bytes(LIBERTINUS).expect("Libertinus Math");
    let params = MathParams::from_font(&font).expect("MATH constants");
    let style = MathStyle::Text;
    let scale = params.scale(style);
    let em_size_pt = Dim::from_i64(10);
    let axis = params.axis_height.checked_mul(&scale).unwrap();

    let body_ast = parse(r"y_i-\widehat y_i").expect("Libertinus stat-r2 delimiter body");
    let body = layout_with_em_size_pt(&body_ast, &font, style, &em_size_pt)
        .expect("Libertinus stat-r2 delimiter body layout");
    let above = body.height.checked_sub(&axis).unwrap().clamp_nonneg();
    let below = body.depth.checked_add(&axis).unwrap();
    let target = tex_delimiter_target(&above.max_ref(&below), &em_size_pt);

    let expected_left = vertical_variants::select_by_advance(&font, '(', &target, &scale);
    let expected_right = vertical_variants::select_by_advance(&font, ')', &target, &scale);
    assert_eq!(expected_left, 3798, "pinned Libertinus left variant");
    assert_eq!(expected_right, 3799, "pinned Libertinus right variant");

    let tree = layout_with_em_size_pt(
        &parse(r"\left(y_i-\widehat y_i\right)").expect("minimal stat-r2 delimiter reproducer"),
        &font,
        style,
        &em_size_pt,
    )
    .expect("minimal stat-r2 delimiter layout");
    let (left, actual_body, right) = delimited_children(&tree);

    assert_eq!(glyph_id(left, '('), expected_left);
    assert_eq!(glyph_id(right, ')'), expected_right);
    assert!(actual_body.width.eq_dim(&body.width));
    assert!(actual_body.height.eq_dim(&body.height));
    assert!(actual_body.depth.eq_dim(&body.depth));
}

fn assert_no_vertical_prebuilt_reaches(font: &MathFont, ch: char, target: &Dim, scale: &Dim) {
    let base = font.glyph(ch).expect("base delimiter glyph");
    let base_span = base
        .height
        .checked_add(&base.depth)
        .unwrap()
        .checked_mul(scale)
        .unwrap();
    assert!(
        base_span < *target,
        "base {ch:?} unexpectedly reaches target"
    );

    let construction = font
        .face()
        .tables()
        .math
        .and_then(|math| math.variants)
        .and_then(|variants| {
            variants
                .vertical_constructions
                .get(ttf_parser::GlyphId(base.glyph_id))
        })
        .expect("fixture vertical MATH construction");
    for index in 0..construction.variants.len() {
        let variant = construction
            .variants
            .get(index)
            .expect("variant index below MATH variant count");
        let advance =
            Dim::from_font_units(i64::from(variant.advance_measurement), font.units_per_em())
                .expect("validated unitsPerEm")
                .checked_mul(scale)
                .unwrap();
        assert!(
            advance < *target,
            "prebuilt {ch:?} variant {} unexpectedly reaches target",
            variant.variant_glyph.0
        );
    }
}

fn assert_vertical_assembly_reaches(bx: &MathBox, expected_ch: char, target: &Dim) {
    let BoxContent::Overlap(parts) = &bx.content else {
        panic!("expected assembled delimiter");
    };
    assert!(
        parts.len() >= 2,
        "assembled delimiter must contain multiple parts"
    );
    for part in parts {
        let BoxContent::Glyph { ch, .. } = &part.content else {
            panic!("vertical assembly parts must be glyphs");
        };
        assert_eq!(*ch, expected_ch);
    }
    let span = bx.height.checked_add(&bx.depth).unwrap();
    assert!(span >= *target, "assembled delimiter must reach target");
}

fn expected_center_shift(font: &MathFont, ch: char, glyph_id: u16, scale: &Dim, axis: &Dim) -> Dim {
    let metrics = font
        .glyph_id(ch, glyph_id)
        .expect("selected delimiter metrics");
    let height = metrics.height.checked_mul(scale).unwrap();
    let depth = metrics.depth.checked_mul(scale).unwrap();
    let center = height
        .checked_sub(&depth)
        .unwrap()
        .checked_div(&Dim::from_i64(2))
        .unwrap();
    axis.checked_sub(&center).unwrap()
}

#[test]
fn delimited_fraction_uses_tex_target_and_math_axis_centering() {
    let font = common::stix_two_math().expect("STIX Two Math");
    let params = MathParams::from_font(&font).expect("MATH constants");
    let style = MathStyle::Display;
    let scale = params.scale(style);
    let em_size_pt = Dim::from_i64(10);
    let axis = params.axis_height.checked_mul(&scale).unwrap();

    let body_ast = parse(r"\frac{a+b}{c+d}").expect("fraction body");
    let body =
        layout_with_em_size_pt(&body_ast, &font, style, &em_size_pt).expect("fraction body layout");
    let above = body.height.checked_sub(&axis).unwrap().clamp_nonneg();
    let below = body.depth.checked_add(&axis).unwrap();
    let max_distance = above.max_ref(&below);
    let expected_target = tex_delimiter_target(&max_distance, &em_size_pt);
    let legacy_target = max_distance.checked_mul(&Dim::from_i64(2)).unwrap();

    let expected_left = vertical_variants::select_by_advance(&font, '(', &expected_target, &scale);
    let legacy_left = vertical_variants::select_by_advance(&font, '(', &legacy_target, &scale);
    assert_ne!(
        expected_left, legacy_left,
        "fixture must cross a delimiter variant boundary under the legacy 2*max-distance rule"
    );
    let expected_right = vertical_variants::select_by_advance(&font, ')', &expected_target, &scale);

    let ast = parse(r"\left(\frac{a+b}{c+d}\right)").expect("delimited fraction");
    let tree =
        layout_with_em_size_pt(&ast, &font, style, &em_size_pt).expect("delimited fraction layout");
    let (left, actual_body, right) = delimited_children(&tree);

    assert_eq!(glyph_id(left, '('), expected_left);
    assert_eq!(glyph_id(right, ')'), expected_right);
    assert!(actual_body.width.eq_dim(&body.width));
    assert!(actual_body.height.eq_dim(&body.height));
    assert!(actual_body.depth.eq_dim(&body.depth));

    let expected_left_shift = expected_center_shift(&font, '(', expected_left, &scale, &axis);
    let expected_right_shift = expected_center_shift(&font, ')', expected_right, &scale, &axis);
    assert!(left.shift.eq_dim(&expected_left_shift));
    assert!(right.shift.eq_dim(&expected_right_shift));

    let expected_height = body
        .height
        .max_ref(&left.height.checked_add(&left.shift).unwrap().clamp_nonneg())
        .max_ref(
            &right
                .height
                .checked_add(&right.shift)
                .unwrap()
                .clamp_nonneg(),
        );
    let expected_depth = body
        .depth
        .max_ref(&left.depth.checked_sub(&left.shift).unwrap().clamp_nonneg())
        .max_ref(
            &right
                .depth
                .checked_sub(&right.shift)
                .unwrap()
                .clamp_nonneg(),
        );
    assert!(tree.height.eq_dim(&expected_height));
    assert!(tree.depth.eq_dim(&expected_depth));
}

#[test]
fn delimiter_shortfall_remains_a_physical_five_points() {
    let font = common::stix_two_math().expect("STIX Two Math");
    let params = MathParams::from_font(&font).expect("MATH constants");
    let style = MathStyle::Display;
    let scale = params.scale(style);
    let em_size_pt = Dim::from_i64(20);
    let axis = params.axis_height.checked_mul(&scale).unwrap();

    let body_ast = parse(r"\rule{0pt}{1.96em}").expect("tall rule");
    let body =
        layout_with_em_size_pt(&body_ast, &font, style, &em_size_pt).expect("tall rule layout");
    let above = body.height.checked_sub(&axis).unwrap().clamp_nonneg();
    let below = body.depth.checked_add(&axis).unwrap();
    let max_distance = above.max_ref(&below);

    let expected_target = tex_delimiter_target(&max_distance, &em_size_pt);
    let wrongly_normalized_target = tex_delimiter_target(&max_distance, &Dim::from_i64(10));
    let expected_left = vertical_variants::select_by_advance(&font, '(', &expected_target, &scale);
    let wrongly_normalized_left =
        vertical_variants::select_by_advance(&font, '(', &wrongly_normalized_target, &scale);
    assert_ne!(
        expected_left, wrongly_normalized_left,
        "fixture must distinguish 5 physical pt from a hard-coded 0.5 em shortfall"
    );

    let ast = parse(r"\left(\rule{0pt}{1.96em}\right)").expect("delimited tall rule");
    let tree = layout_with_em_size_pt(&ast, &font, style, &em_size_pt)
        .expect("delimited tall rule layout");
    let (left, _, _) = delimited_children(&tree);
    assert_eq!(glyph_id(left, '('), expected_left);
}

#[test]
fn left_right_prebuilt_delimiter_families_select_by_opentype_vertical_advance() {
    let font = common::stix_two_math().expect("STIX Two Math");
    let params = MathParams::from_font(&font).expect("MATH constants");
    let style = MathStyle::Display;
    let scale = params.scale(style);
    let em_size_pt = Dim::from_i64(10);
    let axis = params.axis_height.checked_mul(&scale).unwrap();

    let body_ast = parse(r"\frac{a+b}{c+d}").expect("fraction body");
    let body =
        layout_with_em_size_pt(&body_ast, &font, style, &em_size_pt).expect("fraction body layout");
    let above = body.height.checked_sub(&axis).unwrap().clamp_nonneg();
    let below = body.depth.checked_add(&axis).unwrap();
    let target = tex_delimiter_target(&above.max_ref(&below), &em_size_pt);

    for (source, left_ch, right_ch) in [
        (r"\left(\frac{a+b}{c+d}\right)", '(', ')'),
        (r"\left[\frac{a+b}{c+d}\right]", '[', ']'),
        (r"\left\{\frac{a+b}{c+d}\right\}", '{', '}'),
        (r"\left\langle\frac{a+b}{c+d}\right\rangle", '⟨', '⟩'),
    ] {
        let expected_left = vertical_variants::select_by_advance(&font, left_ch, &target, &scale);
        let expected_right = vertical_variants::select_by_advance(&font, right_ch, &target, &scale);
        let tree = layout_with_em_size_pt(
            &parse(source).expect("delimiter family source"),
            &font,
            style,
            &em_size_pt,
        )
        .expect("delimiter family layout");
        let (left, _, right) = delimited_children(&tree);

        assert_eq!(glyph_id(left, left_ch), expected_left, "{source}");
        assert_eq!(glyph_id(right, right_ch), expected_right, "{source}");
    }
}

#[test]
fn left_right_bar_families_assemble_when_prebuilt_variants_are_short() {
    let font = common::stix_two_math().expect("STIX Two Math");
    let params = MathParams::from_font(&font).expect("MATH constants");
    let style = MathStyle::Display;
    let scale = params.scale(style);
    let em_size_pt = Dim::from_i64(10);
    let axis = params.axis_height.checked_mul(&scale).unwrap();

    let body_ast = parse(r"\frac{a+b}{c+d}").expect("fraction body");
    let body =
        layout_with_em_size_pt(&body_ast, &font, style, &em_size_pt).expect("fraction body layout");
    let above = body.height.checked_sub(&axis).unwrap().clamp_nonneg();
    let below = body.depth.checked_add(&axis).unwrap();
    let target = tex_delimiter_target(&above.max_ref(&below), &em_size_pt);

    for (source, ch) in [
        (r"\left|\frac{a+b}{c+d}\right|", '|'),
        (r"\left\Vert\frac{a+b}{c+d}\right\Vert", '‖'),
    ] {
        assert_no_vertical_prebuilt_reaches(&font, ch, &target, &scale);
        let tree = layout_with_em_size_pt(
            &parse(source).expect("bar delimiter source"),
            &font,
            style,
            &em_size_pt,
        )
        .expect("bar delimiter layout");
        let (left, _, right) = delimited_children(&tree);
        assert_vertical_assembly_reaches(left, ch, &target);
        assert_vertical_assembly_reaches(right, ch, &target);
    }
}

#[test]
fn evaluation_delimiter_keeps_null_side_and_assembles_visible_bar_when_prebuilt_is_short() {
    let font = common::stix_two_math().expect("STIX Two Math");
    let params = MathParams::from_font(&font).expect("MATH constants");
    let style = MathStyle::Display;
    let scale = params.scale(style);
    let em_size_pt = Dim::from_i64(10);
    let axis = params.axis_height.checked_mul(&scale).unwrap();

    let body_ast = parse(r"\frac{d}{dx}x^n").expect("evaluation body");
    let body = layout_with_em_size_pt(&body_ast, &font, style, &em_size_pt)
        .expect("evaluation body layout");
    let above = body.height.checked_sub(&axis).unwrap().clamp_nonneg();
    let below = body.depth.checked_add(&axis).unwrap();
    let target = tex_delimiter_target(&above.max_ref(&below), &em_size_pt);
    assert_no_vertical_prebuilt_reaches(&font, '|', &target, &scale);

    let tree = layout_with_em_size_pt(
        &parse(r"\left.\frac{d}{dx}x^n\right|").expect("evaluation delimiter"),
        &font,
        style,
        &em_size_pt,
    )
    .expect("evaluation delimiter layout");
    let (null, _, right) = delimited_children(&tree);
    assert!(matches!(&null.content, BoxContent::Kern(_)));
    assert_vertical_assembly_reaches(right, '|', &target);
}

fn raw_textstyle_parenthesis_span(font: &MathFont) -> Dim {
    let face = font.face();
    let glyph_id = face
        .glyph_index('(')
        .expect("fixture textstyle parenthesis glyph");
    let bbox = face
        .glyph_bounding_box(glyph_id)
        .expect("fixture textstyle parenthesis bounding box");
    let height = i64::from(bbox.y_max).max(0);
    let depth = i64::from(-bbox.y_min).max(0);
    Dim::from_font_units(height + depth, font.units_per_em()).expect("validated unitsPerEm")
}

fn amsmath_explicit_delimiter_target(
    font: &MathFont,
    factor_numerator: i64,
    factor_denominator: i64,
    em_size_pt: &Dim,
) -> Dim {
    let big_size = raw_textstyle_parenthesis_span(font)
        .checked_mul(&Dim::ratio(6, 5).unwrap())
        .unwrap();
    let vcenter_extent = big_size
        .checked_mul(&Dim::from_i64(factor_numerator))
        .unwrap()
        .checked_div(&Dim::from_i64(factor_denominator))
        .unwrap();
    let max_distance = vcenter_extent.checked_div(&Dim::from_i64(2)).unwrap();
    tex_delimiter_target(&max_distance, em_size_pt)
}

fn explicit_delimiter_glyph(tree: &MathBox, expected_ch: char) -> &MathBox {
    match &tree.content {
        BoxContent::Glyph { ch, .. } => {
            assert_eq!(*ch, expected_ch);
            tree
        }
        BoxContent::HList(children) => children
            .iter()
            .find(|child| {
                matches!(
                    &child.content,
                    BoxContent::Glyph { ch, .. } if *ch == expected_ch
                )
            })
            .expect("explicit delimiter glyph in row"),
        _ => panic!("unexpected explicit delimiter layout"),
    }
}

#[test]
fn explicit_big_family_replays_amsmath_parenthesis_and_tex_delimiter_target() {
    let font = common::stix_two_math().expect("STIX Two Math");
    let params = MathParams::from_font(&font).expect("MATH constants");
    let text_scale = params.scale(MathStyle::Text);
    let em_size_pt = Dim::from_i64(10);
    let axis = params.axis_height.checked_mul(&text_scale).unwrap();

    for (source, factor_numerator, factor_denominator) in [
        (r"\bigl(", 1, 1),
        (r"\Bigl(", 3, 2),
        (r"\biggl(", 2, 1),
        (r"\Biggl(", 5, 2),
    ] {
        let target = amsmath_explicit_delimiter_target(
            &font,
            factor_numerator,
            factor_denominator,
            &em_size_pt,
        );
        let expected = vertical_variants::select_by_advance(&font, '(', &target, &text_scale);
        let tree = layout_with_em_size_pt(
            &parse(source).expect("explicit delimiter"),
            &font,
            MathStyle::Display,
            &em_size_pt,
        )
        .expect("explicit delimiter layout");
        let actual = explicit_delimiter_glyph(&tree, '(');
        let BoxContent::Glyph { glyph_id, .. } = &actual.content else {
            unreachable!();
        };
        assert_eq!(*glyph_id, expected, "{source}");

        let expected_shift = expected_center_shift(&font, '(', expected, &text_scale, &axis);
        assert!(actual.shift.eq_dim(&expected_shift), "{source}");
    }
}

#[test]
fn explicit_bigg_stress_brace_rejects_direct_em_target_and_stays_textstyle_fixed() {
    let font = common::stix_two_math().expect("STIX Two Math");
    let params = MathParams::from_font(&font).expect("MATH constants");
    let text_scale = params.scale(MathStyle::Text);
    let em_size_pt = Dim::from_i64(10);
    let target = amsmath_explicit_delimiter_target(&font, 5, 2, &em_size_pt);
    let expected = vertical_variants::select_by_advance(&font, '{', &target, &text_scale);

    let legacy_target = params
        .em(MathStyle::Display)
        .expect("display em")
        .checked_mul(&Dim::from_i64(3))
        .unwrap();
    let legacy = vertical_variants::select_by_advance(&font, '{', &legacy_target, &text_scale);
    assert_ne!(
        expected, legacy,
        "STIX fixture must reproduce the hard-explicit-big direct-em variant bug"
    );

    let axis = params.axis_height.checked_mul(&text_scale).unwrap();
    let expected_shift = expected_center_shift(&font, '{', expected, &text_scale, &axis);
    let ast = parse(r"\Biggl\{").expect("explicit stress brace");
    for style in [
        MathStyle::Display,
        MathStyle::Text,
        MathStyle::Script,
        MathStyle::ScriptScript,
    ] {
        let tree = layout_with_em_size_pt(&ast, &font, style, &em_size_pt)
            .expect("explicit stress brace layout");
        let actual = explicit_delimiter_glyph(&tree, '{');
        let BoxContent::Glyph { glyph_id, .. } = &actual.content else {
            unreachable!();
        };
        assert_eq!(*glyph_id, expected, "{style:?}");
        assert!(actual.shift.eq_dim(&expected_shift), "{style:?}");
    }
}
