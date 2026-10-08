use super::common;

use crate::test_support::{layout, parse, styled_char, MathStyle, TextStyle};

#[test]
fn math_accents_and_bars_keep_the_direct_nucleus_italic_advance() {
    let font = common::stix_two_math().expect("STIX Two Math fixture");

    let italic_j = font
        .glyph(styled_char('J', TextStyle::It))
        .expect("default math italic J");

    let italic = font.italic_correction(italic_j.glyph_id);

    assert!(
        !italic.is_zero(),
        "STIX fixture must have nonzero italic correction for math italic J"
    );

    let expected = italic_j.advance.checked_add(&italic).expect("accent width");

    for source in [
        r"\hat{J}",
        r"\tilde{J}",
        r"\widehat{J}",
        r"\widetilde{J}",
        r"\vec{J}",
        r"\overrightarrow{J}",
        r"\underleftarrow{J}",
        r"\overbrace{J}",
        r"\underbrace{J}",
        r"\overline{J}",
        r"\underline{J}",
    ] {
        let ast = parse(source).expect("parse accent nucleus width case");

        let laid = layout(&ast, &font, MathStyle::Text).expect("layout accent nucleus width case");

        assert!(
            laid.width.eq_dim(&expected),
            "{source}: width {} != direct-nucleus width {}",
            laid.width.to_dec_string(),
            expected.to_dec_string()
        );
    }
}
