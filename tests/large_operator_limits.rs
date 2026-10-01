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

fn limit_branches(tree: &MathBox) -> (&MathBox, &MathBox, &MathBox) {
    let BoxContent::Overlap(branches) = &tree.content else {
        panic!("display sum with limits must be an overlap");
    };
    let [base, upper, lower] = branches.as_slice() else {
        panic!("display sum must contain base, upper limit, and lower limit");
    };
    (base, upper, lower)
}

#[test]
fn display_large_operator_uses_math_axis_and_independent_limit_constraints() {
    let font = MathFont::stix_two_math().expect("STIX Two Math");
    let params = MathParams::from_font(&font).expect("MATH constants");
    let style = MathStyle::Display;
    let scale = params.scale(style);
    let axis = mul(&params.axis_height, &scale);

    let ast = parse(r"\sum_{i=1}^{n}").expect("display sum");
    let tree = layout(&ast, &font, style).expect("display sum layout");
    let (base, upper, lower) = limit_branches(&tree);

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
