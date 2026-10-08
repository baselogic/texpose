use super::common;

use crate::test_support::{
    layout, layout_with_diagnostics, parse, BoxContent, Dim, LayoutDiagnostic, MathBox, MathFont,
    MathParams, MathStyle,
};

const STIX: &[u8] = include_bytes!("../../../data/fonts/stix-two-math/STIXTwoMath-Regular.otf");
const LIBERTINUS: &[u8] =
    include_bytes!("../../../tests/fixtures/fonts/libertinus-math/LibertinusMath-Regular.otf");
const FIRA: &[u8] = include_bytes!("../../../tests/fixtures/fonts/fira-math/FiraMath-Regular.otf");
const MISSING: char = '\u{10FFFF}';

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

fn add(a: &Dim, b: &Dim) -> Dim {
    a.checked_add(b).expect("test dimension addition")
}

fn shifted_child(boxed: &MathBox, above: bool) -> &MathBox {
    let BoxContent::Overlap(children) = &boxed.content else {
        panic!("over/under construct must be an overlap");
    };
    children
        .iter()
        .find(|child| {
            if above {
                child.shift > Dim::zero()
            } else {
                child.shift < Dim::zero()
            }
        })
        .expect("shifted annotation child")
}

#[test]
fn math_phantoms_measure_math_hbox_without_terminal_italic_and_are_ordinary() {
    for (name, font) in profile_fonts() {
        let raw = layout(&parse("x").unwrap(), &font, MathStyle::Text).unwrap();
        let clean = raw.width.clone();

        let full = layout(&parse(r"\phantom{x}").unwrap(), &font, MathStyle::Text).unwrap();
        assert_eq!(full.width, clean, "{name} full phantom width");
        assert_eq!(full.height, raw.height, "{name} full phantom height");
        assert_eq!(full.depth, raw.depth, "{name} full phantom depth");
        assert_eq!(full.italic, Dim::zero(), "{name} full phantom italic");
        assert!(matches!(full.content, BoxContent::Empty));

        let vertical = layout(&parse(r"\vphantom{x}").unwrap(), &font, MathStyle::Text).unwrap();
        assert_eq!(vertical.width, Dim::zero(), "{name} vertical phantom width");
        assert_eq!(
            vertical.height, raw.height,
            "{name} vertical phantom height"
        );
        assert_eq!(vertical.depth, raw.depth, "{name} vertical phantom depth");
        assert_eq!(
            vertical.italic,
            Dim::zero(),
            "{name} vertical phantom italic"
        );
        assert!(matches!(vertical.content, BoxContent::Empty));

        let horizontal = layout(&parse(r"\hphantom{x}").unwrap(), &font, MathStyle::Text).unwrap();
        assert_eq!(horizontal.width, clean, "{name} horizontal phantom width");
        assert_eq!(
            horizontal.height,
            Dim::zero(),
            "{name} horizontal phantom height"
        );
        assert_eq!(
            horizontal.depth,
            Dim::zero(),
            "{name} horizontal phantom depth"
        );
        assert_eq!(
            horizontal.italic,
            Dim::zero(),
            "{name} horizontal phantom italic"
        );
        assert!(matches!(horizontal.content, BoxContent::Empty));

        let around = layout(&parse(r"a\phantom{=}b").unwrap(), &font, MathStyle::Text).unwrap();
        let plain = layout(&parse("ab").unwrap(), &font, MathStyle::Text).unwrap();
        let phantom_rel = layout(&parse(r"\phantom{=}").unwrap(), &font, MathStyle::Text).unwrap();
        assert_eq!(
            around.width,
            add(&plain.width, &phantom_rel.width),
            "{name} phantom box must not inherit relation spacing"
        );
    }
}

#[test]
fn overset_and_underset_use_clean_limit_boxes_and_independent_constraints() {
    for (name, font) in profile_fonts() {
        let params = MathParams::from_font(&font).expect("validated MATH constants");
        let scale = params.scale(MathStyle::Display);

        let base = layout(&parse("=").unwrap(), &font, MathStyle::Display).unwrap();
        let over_raw = layout(&parse("g").unwrap(), &font, MathStyle::Script).unwrap();
        let over_width = add(&over_raw.width, &over_raw.italic);
        let over = layout(
            &parse(r"\overset{g}{=}").unwrap(),
            &font,
            MathStyle::Display,
        )
        .unwrap();
        assert_eq!(
            over.width,
            base.width.max_ref(&over_width),
            "{name} overset width must include terminal annotation italic"
        );
        let upper_gap = params.upper_limit_gap_min.checked_mul(&scale).unwrap();
        let upper_rise = params
            .upper_limit_baseline_rise_min
            .checked_mul(&scale)
            .unwrap();
        let upper_offset = upper_rise.max_ref(&add(&upper_gap, &over_raw.depth));
        let expected_upper_shift = add(&base.height, &upper_offset);
        assert_eq!(
            shifted_child(&over, true).shift,
            expected_upper_shift,
            "{name} overset must enforce rise and edge-gap independently"
        );

        let under_style = MathStyle::ScriptCramped;
        let under_raw = layout(&parse("y").unwrap(), &font, under_style).unwrap();
        let under_width = add(&under_raw.width, &under_raw.italic);
        let under = layout(
            &parse(r"\underset{y}{=}").unwrap(),
            &font,
            MathStyle::Display,
        )
        .unwrap();
        assert_eq!(
            under.width,
            base.width.max_ref(&under_width),
            "{name} underset width must include terminal annotation italic"
        );
        let lower_gap = params.lower_limit_gap_min.checked_mul(&scale).unwrap();
        let lower_drop = params
            .lower_limit_baseline_drop_min
            .checked_mul(&scale)
            .unwrap();
        let lower_offset = lower_drop.max_ref(&add(&lower_gap, &under_raw.height));
        let expected_lower_shift = -add(&base.depth, &lower_offset);
        assert_eq!(
            shifted_child(&under, false).shift,
            expected_lower_shift,
            "{name} underset must enforce drop and edge-gap independently"
        );

        let stackrel = layout(
            &parse(r"\stackrel{g}{=}").unwrap(),
            &font,
            MathStyle::Display,
        )
        .unwrap();
        assert_eq!(
            stackrel, over,
            "{name} supported stackrel relation geometry"
        );
    }
}

#[test]
fn phantom_and_overunder_degrade_missing_glyphs_without_losing_structure() {
    let font = common::stix_two_math().expect("STIX Two Math");

    let phantom_source = format!(r"\phantom{{{MISSING}}}");
    let phantom = layout_with_diagnostics(
        &parse(&phantom_source).expect("phantom with missing scalar"),
        &font,
        MathStyle::Text,
    )
    .expect("phantom missing glyph must use deterministic fallback");
    assert_eq!(
        phantom.diagnostics,
        vec![LayoutDiagnostic::MissingGlyph { ch: MISSING }]
    );
    assert!(matches!(phantom.math_box.content, BoxContent::Empty));
    assert!(phantom.math_box.width > Dim::zero());

    let overset_source = format!(r"\overset{{{MISSING}}}{{=}}");
    let overset = layout_with_diagnostics(
        &parse(&overset_source).expect("overset with missing scalar"),
        &font,
        MathStyle::Display,
    )
    .expect("overset missing glyph must use deterministic fallback");
    assert_eq!(
        overset.diagnostics,
        vec![LayoutDiagnostic::MissingGlyph { ch: MISSING }]
    );
    let BoxContent::Overlap(children) = &overset.math_box.content else {
        panic!("overset fallback must preserve over/under overlap structure");
    };
    assert!(children.iter().any(|child| child.shift > Dim::zero()));
}
