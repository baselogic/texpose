#![no_main]

use std::sync::OnceLock;

use libfuzzer_sys::fuzz_target;
use num_bigint::BigInt;
use num_rational::BigRational;
use texpose::{
    layout_with_em_size_pt, Dim, Error, Length, MathFont, MathNode, MathStyle, NumericError,
    SpaceKind,
};
use texpose_fuzz::{
    assert_matches, dim_from_fraction, dim_ratio, fits_dim, physical_layout_prelude_fits,
    read_i128, read_i64, read_u16,
};

const INPUT_BYTES: usize = 75;
const STIX: &[u8] =
    include_bytes!("../../tests/fixtures/fonts/stix-two-math/STIXTwoMath-Regular.otf");

fuzz_target!(|data: &[u8]| {
    if data.len() < INPUT_BYTES {
        return;
    }

    check_font_units(read_i64(data, 0), read_u16(data, 8));

    let (value, value_ref) = dim_from_fraction(read_i128(data, 10), read_i128(data, 26));
    let (root_raw, _) = dim_from_fraction(read_i128(data, 42), read_i128(data, 58));
    let root = root_raw.abs();
    let root = if root.is_zero() { Dim::one() } else { root };
    let root_ref = dim_ratio(&root);
    let layout_prelude_fits = physical_layout_prelude_fits(&root_ref);
    let style = style_from_byte(data[74]);
    let font = stix_font();

    let pt_expected = value_ref.clone() / root_ref.clone();
    check_physical_length(
        font,
        style,
        Length::TexPt(value.clone()),
        &root,
        &pt_expected,
        true,
        layout_prelude_fits,
    );

    let bp_in_tex_pt = value_ref * BigRational::new(BigInt::from(7227), BigInt::from(7200));
    let bp_expected = bp_in_tex_pt.clone() / root_ref;
    check_physical_length(
        font,
        style,
        Length::BigPt(value),
        &root,
        &bp_expected,
        fits_dim(&bp_in_tex_pt),
        layout_prelude_fits,
    );
});

fn stix_font() -> &'static MathFont {
    static FONT: OnceLock<MathFont> = OnceLock::new();
    FONT.get_or_init(|| MathFont::from_bytes(STIX).expect("pinned STIX fuzz fixture"))
}

fn check_font_units(units: i64, units_per_em: u16) {
    let actual = Dim::from_font_units(units, units_per_em);
    if units_per_em == 0 {
        assert_eq!(actual, Err(NumericError::ZeroDenominator));
        return;
    }

    let expected = BigRational::new(BigInt::from(units), BigInt::from(units_per_em));
    assert_matches(&actual.expect("nonzero units-per-em"), &expected);
}

fn check_physical_length(
    font: &MathFont,
    style: MathStyle,
    length: Length,
    root_em_size_pt: &Dim,
    expected: &BigRational,
    intermediate_fits: bool,
    layout_prelude_fits: bool,
) {
    let node = MathNode::Space(SpaceKind::Hspace(length));
    let actual = layout_with_em_size_pt(&node, font, style, root_em_size_pt);

    if layout_prelude_fits && intermediate_fits && fits_dim(expected) {
        let math_box = actual.expect("representable physical length must layout");
        assert_matches(&math_box.width, expected);
        assert!(math_box.height.is_zero());
        assert!(math_box.depth.is_zero());
    } else {
        assert_eq!(
            actual,
            Err(Error::Numeric(NumericError::ArithmeticOverflow))
        );
    }
}

fn style_from_byte(value: u8) -> MathStyle {
    match value & 7 {
        0 => MathStyle::Display,
        1 => MathStyle::DisplayCramped,
        2 => MathStyle::Text,
        3 => MathStyle::TextCramped,
        4 => MathStyle::Script,
        5 => MathStyle::ScriptCramped,
        6 => MathStyle::ScriptScript,
        _ => MathStyle::ScriptScriptCramped,
    }
}
