use std::sync::Arc;

use texpose::{layout_with_numbering, parse, Error, MathFont, MathStyle, NumberingState};

const STIX: &[u8] = include_bytes!("../data/fonts/stix-two-math/STIXTwoMath-Regular.otf");

fn numbered(source: &str, state: &mut NumberingState, font: &MathFont) -> Result<(), Error> {
    let ast = parse(source)?;
    layout_with_numbering(&ast, font, MathStyle::Display, state)?;
    Ok(())
}

#[test]
fn nested_ams_numbers_follow_prepass_order_with_tags_and_suppression() {
    let font = MathFont::from_shared_bytes(Arc::from(STIX), 0).unwrap();
    let mut state = NumberingState::default();
    let source = r"\begin{gather}
        \begin{align}a&=b\label{inner:first}\\c&=d\tag{S}\label{inner:tag}\end{align}\label{outer:first}\\
        \begin{align}u&=v\nonumber\label{inner:skip}\\p&=q\label{inner:second}\end{align}\tag*{G}\label{outer:tag}\\
        z\label{outer:last}
    \end{gather}";
    numbered(source, &mut state, &font).unwrap();

    for (key, expected) in [
        ("inner:first", Some("(1)")),
        ("inner:tag", Some("(S)")),
        ("inner:skip", None),
        ("inner:second", Some("(2)")),
        ("outer:first", Some("(3)")),
        ("outer:tag", Some("G")),
        ("outer:last", Some("(4)")),
    ] {
        assert_eq!(state.label(key), expected, "{key}");
    }

    numbered(
        r"\begin{equation}x\label{after:gather}\end{equation}",
        &mut state,
        &font,
    )
    .unwrap();
    assert_eq!(state.label("after:gather"), Some("(5)"));
}

#[test]
fn split_does_not_consume_an_extra_number_in_align() {
    let font = MathFont::from_shared_bytes(Arc::from(STIX), 0).unwrap();
    let mut state = NumberingState::default();
    let source = r"\begin{align}
        a&=b\label{first}\\
        \begin{split}c&=d\\e&=f\end{split}\label{split}\\
        g&=h\tag{T}\label{tagged}\\
        i&=j\nonumber\label{suppressed}\\
        k&=l\label{last}
    \end{align}";
    numbered(source, &mut state, &font).unwrap();

    for (key, expected) in [
        ("first", Some("(1)")),
        ("split", Some("(2)")),
        ("tagged", Some("(T)")),
        ("suppressed", None),
        ("last", Some("(3)")),
    ] {
        assert_eq!(state.label(key), expected, "{key}");
    }
}

#[test]
fn unconsumed_number_plan_aborts_without_advancing_state() {
    let font = MathFont::from_shared_bytes(Arc::from(STIX), 0).unwrap();
    let mut state = NumberingState::default();

    // An empty numbered environment currently has a prepass assignment but
    // returns from matrix layout before consuming it. Do not commit that drift.
    for source in [
        r"\begin{equation}\end{equation}",
        r"\begin{multline}\end{multline}",
    ] {
        let err = numbered(source, &mut state, &font).unwrap_err();
        assert!(matches!(&err, Error::Malformed { .. }), "{source}: {err}");
    }

    numbered(
        r"\begin{equation}x\label{after:failure}\end{equation}",
        &mut state,
        &font,
    )
    .unwrap();
    assert_eq!(state.label("after:failure"), Some("(1)"));
}
