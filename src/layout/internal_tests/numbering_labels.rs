use super::common;

use crate::test_support::{
    layout, layout_with_numbering, parse, Error, MathBox, MathStyle, NumberFormat, NumberStyle,
    NumberingConfig, NumberingState,
};

fn layout_numbered(
    source: &str,
    state: &mut NumberingState,
    font: &texpose::MathFont,
) -> Result<MathBox, Error> {
    let ast = parse(source)?;
    layout_with_numbering(&ast, font, MathStyle::Display, state)
}

fn layout_plain(source: &str, font: &texpose::MathFont) -> MathBox {
    let ast = parse(source).expect("parse source");
    layout(&ast, font, MathStyle::Display).expect("layout source")
}

fn assert_same_dims(left: &MathBox, right: &MathBox) {
    assert!(left.width.eq_dim(&right.width), "width");
    assert!(left.height.eq_dim(&right.height), "height");
    assert!(left.depth.eq_dim(&right.depth), "depth");
}

#[test]
fn references_use_unwrapped_value_while_labels_keep_display_format() {
    let font = common::stix_two_math().expect("STIX Two Math");
    let mut state = NumberingState::default();

    layout_numbered(
        r"\begin{equation}\label{eq:auto}x\end{equation}",
        &mut state,
        &font,
    )
    .expect("numbered equation");
    assert_eq!(state.label("eq:auto"), Some("(1)"));

    let reference = layout_numbered(r"\ref{eq:auto}", &mut state, &font).expect("reference");
    let plain = layout_plain(r"\mathrm{1}", &font);
    let wrapped = layout_plain(r"\mathrm{(1)}", &font);
    assert_same_dims(&reference, &plain);
    assert!(!reference.width.eq_dim(&wrapped.width));
}

#[test]
fn numbering_styles_and_formats_do_not_leak_into_reference_payloads() {
    let font = common::stix_two_math().expect("STIX Two Math");

    let roman_cfg = NumberingConfig::new()
        .with_style(NumberStyle::Roman)
        .with_start(4)
        .with_format(NumberFormat::Bracketed);
    let mut roman = NumberingState::new(roman_cfg);
    layout_numbered(
        r"\begin{equation}\label{eq:roman}x\end{equation}",
        &mut roman,
        &font,
    )
    .expect("roman equation");
    assert_eq!(roman.label("eq:roman"), Some("[iv]"));
    let roman_ref = layout_numbered(r"\ref{eq:roman}", &mut roman, &font).expect("roman ref");
    assert_same_dims(&roman_ref, &layout_plain(r"\mathrm{iv}", &font));

    let alpha_cfg = NumberingConfig::new()
        .with_style(NumberStyle::Alphabetic)
        .with_start(27)
        .with_format(NumberFormat::Plain);
    let mut alpha = NumberingState::new(alpha_cfg);
    layout_numbered(
        r"\begin{equation}\label{eq:alpha}x\end{equation}",
        &mut alpha,
        &font,
    )
    .expect("alphabetic equation");
    assert_eq!(alpha.label("eq:alpha"), Some("aa"));
}

#[test]
fn tags_and_suppression_do_not_consume_the_automatic_counter() {
    let font = common::stix_two_math().expect("STIX Two Math");
    let mut state = NumberingState::default();

    layout_numbered(
        r"\begin{equation}\tag{A}\label{eq:tag}x\end{equation}",
        &mut state,
        &font,
    )
    .expect("tagged equation");
    assert_eq!(state.label("eq:tag"), Some("(A)"));

    layout_numbered(
        r"\begin{equation}\label{eq:first}x\end{equation}",
        &mut state,
        &font,
    )
    .expect("first automatic equation");
    assert_eq!(state.label("eq:first"), Some("(1)"));

    layout_numbered(
        r"\begin{equation}\nonumber\label{eq:suppressed}x\end{equation}",
        &mut state,
        &font,
    )
    .expect("suppressed equation");
    assert_eq!(state.label("eq:suppressed"), None);

    layout_numbered(
        r"\begin{equation}\tag*{B}\label{eq:tagstar}x\end{equation}",
        &mut state,
        &font,
    )
    .expect("starred tag");
    assert_eq!(state.label("eq:tagstar"), Some("B"));

    layout_numbered(
        r"\begin{equation}\label{eq:second}x\end{equation}",
        &mut state,
        &font,
    )
    .expect("second automatic equation");
    assert_eq!(state.label("eq:second"), Some("(2)"));
}

#[test]
fn failed_layout_does_not_commit_numbers_or_labels() {
    let font = common::stix_two_math().expect("STIX Two Math");
    let mut state = NumberingState::default();

    let err = layout_numbered(
        r"\begin{equation}\label{eq:failed}x\end{equation}\hline",
        &mut state,
        &font,
    )
    .expect_err("hline outside array must fail");
    assert!(matches!(err, Error::Unsupported { .. }));
    assert_eq!(state.label("eq:failed"), None);

    layout_numbered(
        r"\begin{equation}\label{eq:after}x\end{equation}",
        &mut state,
        &font,
    )
    .expect("number after failed layout");
    assert_eq!(state.label("eq:after"), Some("(1)"));
}

#[test]
fn equation_counter_overflow_fails_without_state_mutation() {
    let font = common::stix_two_math().expect("STIX Two Math");

    let config = NumberingConfig::new().with_start(usize::MAX);
    let mut state = NumberingState::new(config);
    let err = layout_numbered(
        r"\begin{equation}\label{eq:max}x\end{equation}",
        &mut state,
        &font,
    )
    .expect_err("counter overflow must fail");
    assert!(matches!(err, Error::InvalidOption { .. }));
    assert_eq!(state.label("eq:max"), None);
}
