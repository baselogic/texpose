mod common;

use core::cmp::Ordering;

use texpose::{
    layout_with_em_size_pt, parse, BoxContent, Dim, MathBox, MathFont, MathParams, MathStyle,
};

fn select_vertical_variant(font: &MathFont, ch: char, target: &Dim, scale: &Dim) -> u16 {
    let base = font.glyph(ch).expect("delimiter glyph");
    let mut best_fitting: Option<(u16, Dim)> = None;
    let mut tallest_short: Option<(u16, Dim)> = None;

    for glyph_id in font.vertical_variants(base.glyph_id) {
        let metrics = font
            .glyph_id(ch, glyph_id)
            .expect("delimiter variant metrics");
        let span = metrics
            .height
            .checked_add(&metrics.depth)
            .unwrap()
            .checked_mul(scale)
            .unwrap();
        match span.cmp(target) {
            Ordering::Equal | Ordering::Greater => {
                let tighter = best_fitting
                    .as_ref()
                    .map_or(true, |(_, best_span)| span.cmp(best_span) == Ordering::Less);
                if tighter {
                    best_fitting = Some((glyph_id, span));
                }
            }
            Ordering::Less => {
                let taller = tallest_short.as_ref().map_or(true, |(_, best_span)| {
                    span.cmp(best_span) == Ordering::Greater
                });
                if taller {
                    tallest_short = Some((glyph_id, span));
                }
            }
        }
    }

    best_fitting
        .or(tallest_short)
        .expect("at least the base delimiter glyph")
        .0
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

fn glyph_id(bx: &MathBox, expected_ch: char) -> u16 {
    let BoxContent::Glyph { ch, glyph_id, .. } = &bx.content else {
        panic!("expected delimiter glyph");
    };
    assert_eq!(*ch, expected_ch);
    *glyph_id
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

    let expected_left = select_vertical_variant(&font, '(', &expected_target, &scale);
    let legacy_left = select_vertical_variant(&font, '(', &legacy_target, &scale);
    assert_ne!(
        expected_left, legacy_left,
        "fixture must cross a delimiter variant boundary under the legacy 2*max-distance rule"
    );
    let expected_right = select_vertical_variant(&font, ')', &expected_target, &scale);

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
    let expected_left = select_vertical_variant(&font, '(', &expected_target, &scale);
    let wrongly_normalized_left =
        select_vertical_variant(&font, '(', &wrongly_normalized_target, &scale);
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
