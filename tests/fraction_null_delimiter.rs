mod common;

use texpose::{layout, layout_with_em_size_pt, layout_with_max_depth, parse, Dim, MathStyle};

#[test]
fn fraction_null_delimiter_space_remains_physical_across_em_sizes() {
    let font = common::stix_two_math().expect("STIX Two Math");
    let ast = parse(r"\frac{1}{2}").expect("fraction");

    let ten_pt = layout_with_em_size_pt(&ast, &font, MathStyle::Text, &Dim::from_i64(10))
        .expect("10 pt fraction");
    let twenty_pt = layout_with_em_size_pt(&ast, &font, MathStyle::Text, &Dim::from_i64(20))
        .expect("20 pt fraction");
    let default = layout(&ast, &font, MathStyle::Text).expect("default fraction");

    assert_eq!(default.width, ten_pt.width);
    assert_eq!(
        ten_pt.width.checked_sub(&twenty_pt.width).unwrap(),
        Dim::ratio(3, 25).unwrap()
    );
    assert_eq!(ten_pt.height, twenty_pt.height);
    assert_eq!(ten_pt.depth, twenty_pt.depth);

    let nested = parse(r"\frac{\frac{1}{2}}{3}").expect("nested fraction");
    let nested_ten = layout_with_em_size_pt(&nested, &font, MathStyle::Text, &Dim::from_i64(10))
        .expect("10 pt nested fraction");
    let nested_twenty = layout_with_em_size_pt(&nested, &font, MathStyle::Text, &Dim::from_i64(20))
        .expect("20 pt nested fraction");
    assert_eq!(
        nested_ten.width.checked_sub(&nested_twenty.width).unwrap(),
        Dim::ratio(6, 25).unwrap()
    );
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
