use super::common;

use crate::test_support::{
    layout, layout_with_em_size_pt, layout_with_max_depth, parse, Dim, MathStyle,
};

#[test]
fn fraction_null_delimiter_space_remains_physical_across_em_sizes() {
    let font = common::stix_two_math().expect("STIX Two Math");
    let ast = parse(r"\frac{1}{2}").expect("fraction");

    let sizes = [6_i64, 10, 20, 40];
    let layouts = sizes.map(|size| {
        layout_with_em_size_pt(&ast, &font, MathStyle::Text, &Dim::from_i64(size))
            .unwrap_or_else(|error| panic!("{size} pt fraction: {error}"))
    });
    let default = layout(&ast, &font, MathStyle::Text).expect("default fraction");

    assert_eq!(default.width, layouts[1].width);
    for (size, boxed) in sizes.into_iter().zip(layouts.iter()) {
        assert_eq!(boxed.height, layouts[1].height, "{size}pt height");
        assert_eq!(boxed.depth, layouts[1].depth, "{size}pt depth");
    }
    for ((left_size, left), (right_size, right)) in sizes
        .into_iter()
        .zip(layouts.iter())
        .zip(sizes.into_iter().zip(layouts.iter()).skip(1))
    {
        let expected = Dim::ratio(12, 5)
            .unwrap()
            .checked_mul(
                &Dim::ratio(1, left_size)
                    .unwrap()
                    .checked_sub(&Dim::ratio(1, right_size).unwrap())
                    .unwrap(),
            )
            .unwrap();
        assert_eq!(
            left.width.checked_sub(&right.width).unwrap(),
            expected,
            "{left_size}pt -> {right_size}pt null-delimiter delta"
        );
    }

    let nested = parse(r"\frac{\frac{1}{2}}{3}").expect("nested fraction");
    let nested_layouts = sizes.map(|size| {
        layout_with_em_size_pt(&nested, &font, MathStyle::Text, &Dim::from_i64(size))
            .unwrap_or_else(|error| panic!("{size} pt nested fraction: {error}"))
    });
    for (size, boxed) in sizes.into_iter().zip(nested_layouts.iter()) {
        assert_eq!(
            boxed.height, nested_layouts[1].height,
            "{size}pt nested height"
        );
        assert_eq!(
            boxed.depth, nested_layouts[1].depth,
            "{size}pt nested depth"
        );
    }
    for ((left_size, left), (right_size, right)) in sizes
        .into_iter()
        .zip(nested_layouts.iter())
        .zip(sizes.into_iter().zip(nested_layouts.iter()).skip(1))
    {
        let expected = Dim::ratio(24, 5)
            .unwrap()
            .checked_mul(
                &Dim::ratio(1, left_size)
                    .unwrap()
                    .checked_sub(&Dim::ratio(1, right_size).unwrap())
                    .unwrap(),
            )
            .unwrap();
        assert_eq!(
            left.width.checked_sub(&right.width).unwrap(),
            expected,
            "{left_size}pt -> {right_size}pt nested null-delimiter delta"
        );
    }
}

#[test]
fn explicit_em_size_validation_and_max_depth_keep_compatible_defaults() {
    let font = common::stix_two_math().expect("STIX Two Math");
    let ast = parse(r"\frac{1}{2}").expect("fraction");

    let default = layout(&ast, &font, MathStyle::Text).expect("default layout");
    let bounded = layout_with_max_depth(&ast, &font, MathStyle::Text, 64).expect("bounded layout");

    assert_eq!(default.width, bounded.width);
    assert_eq!(default.height, bounded.height);
    assert_eq!(default.depth, bounded.depth);

    assert!(layout_with_em_size_pt(&ast, &font, MathStyle::Text, &Dim::zero()).is_err());
    assert!(layout_with_em_size_pt(&ast, &font, MathStyle::Text, &Dim::from_i64(-1)).is_err());
}
