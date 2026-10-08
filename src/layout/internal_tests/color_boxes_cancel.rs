use super::common;

use crate::test_support::{
    layout, layout_with_diagnostics, layout_with_em_size_pt, parse, BoxContent, Dim,
    LayoutDiagnostic, MathBox, MathFont, MathParams, MathStyle,
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

fn math_hbox(source: &str, font: &MathFont, style: MathStyle) -> MathBox {
    layout(&parse(source).unwrap(), font, style).unwrap()
}

fn first_frame(boxed: &MathBox) -> Option<(&Dim, &MathBox)> {
    match &boxed.content {
        BoxContent::Frame {
            thickness, inner, ..
        } => Some((thickness, inner)),
        BoxContent::HList(children)
        | BoxContent::VList(children)
        | BoxContent::Overlap(children) => children.iter().find_map(first_frame),
        BoxContent::Color(_, inner) | BoxContent::BackColor(_, inner) => first_frame(inner),
        _ => None,
    }
}

fn lines<'a>(boxed: &'a MathBox, out: &mut Vec<(&'a Dim, &'a Dim, &'a Dim, &'a Dim, &'a Dim)>) {
    match &boxed.content {
        BoxContent::Line {
            x1,
            y1,
            x2,
            y2,
            thickness,
        } => out.push((x1, y1, x2, y2, thickness)),
        BoxContent::HList(children)
        | BoxContent::VList(children)
        | BoxContent::Overlap(children) => {
            for child in children {
                lines(child, out);
            }
        }
        BoxContent::Color(_, inner)
        | BoxContent::BackColor(_, inner)
        | BoxContent::Frame { inner, .. } => lines(inner, out),
        _ => {}
    }
}

fn glyph_scales(boxed: &MathBox, out: &mut Vec<Dim>) {
    match &boxed.content {
        BoxContent::Glyph { scale, .. } => out.push(scale.clone()),
        BoxContent::HList(children)
        | BoxContent::VList(children)
        | BoxContent::Overlap(children) => {
            for child in children {
                glyph_scales(child, out);
            }
        }
        BoxContent::Color(_, inner)
        | BoxContent::BackColor(_, inner)
        | BoxContent::Frame { inner, .. } => glyph_scales(inner, out),
        _ => {}
    }
}

#[test]
fn color_wrappers_are_geometry_transparent_and_color_boxes_are_ordinary() {
    for (name, font) in profile_fonts() {
        let plain = layout(&parse("a=b").unwrap(), &font, MathStyle::Text).unwrap();
        let colored = layout(
            &parse(r"a\textcolor{red}{=}b").unwrap(),
            &font,
            MathStyle::Text,
        )
        .unwrap();
        assert_eq!(colored.width, plain.width, "{name} textcolor width");
        assert_eq!(colored.height, plain.height, "{name} textcolor height");
        assert_eq!(colored.depth, plain.depth, "{name} textcolor depth");

        let raw = math_hbox("x", &font, MathStyle::Text);
        let boxed = layout(
            &parse(r"\colorbox{yellow}{x}").unwrap(),
            &font,
            MathStyle::Text,
        )
        .unwrap();
        assert_eq!(
            boxed.width.checked_sub(&raw.width).unwrap(),
            Dim::ratio(3, 5).unwrap(),
            "{name} colorbox must add only 6pt at a 10pt root em"
        );
    }
}

#[test]
fn latex_box_padding_and_rules_remain_physical_across_root_em_sizes() {
    let sizes = [6_i64, 10, 20, 40];
    for (name, font) in profile_fonts() {
        let raw = math_hbox("x", &font, MathStyle::Text);
        for size in sizes {
            let root = Dim::from_i64(size);
            let color = layout_with_em_size_pt(
                &parse(r"\colorbox{yellow}{x}").unwrap(),
                &font,
                MathStyle::Text,
                &root,
            )
            .unwrap();
            assert_eq!(
                color.width.checked_sub(&raw.width).unwrap(),
                Dim::ratio(6, size).unwrap(),
                "{name} {size}pt colorbox width padding"
            );
            assert_eq!(
                color.height.checked_sub(&raw.height).unwrap(),
                Dim::ratio(3, size).unwrap(),
                "{name} {size}pt colorbox top padding"
            );
            assert_eq!(
                color.depth.checked_sub(&raw.depth).unwrap(),
                Dim::ratio(3, size).unwrap(),
                "{name} {size}pt colorbox bottom padding"
            );

            let framed = layout_with_em_size_pt(
                &parse(r"\fcolorbox{red}{yellow}{x}").unwrap(),
                &font,
                MathStyle::Text,
                &root,
            )
            .unwrap();
            assert_eq!(
                framed.width.checked_sub(&raw.width).unwrap(),
                Dim::ratio(34, 5 * size).unwrap(),
                "{name} {size}pt fcolorbox width"
            );
            assert_eq!(
                framed.height.checked_sub(&raw.height).unwrap(),
                Dim::ratio(17, 5 * size).unwrap(),
                "{name} {size}pt fcolorbox height"
            );
            assert_eq!(
                framed.depth.checked_sub(&raw.depth).unwrap(),
                Dim::ratio(17, 5 * size).unwrap(),
                "{name} {size}pt fcolorbox depth"
            );
            let (thickness, _) = first_frame(&framed).expect("fcolorbox frame");
            assert_eq!(
                *thickness,
                Dim::ratio(2, 5 * size).unwrap(),
                "{name} {size}pt fboxrule"
            );
        }
    }
}

#[test]
fn boxed_forces_displaystyle_and_uses_latex_fbox_geometry() {
    for (name, font) in profile_fonts() {
        let text = layout(
            &parse(r"\boxed{\frac{1}{2}}").unwrap(),
            &font,
            MathStyle::Text,
        )
        .unwrap();
        let script = layout(
            &parse(r"\boxed{\frac{1}{2}}").unwrap(),
            &font,
            MathStyle::Script,
        )
        .unwrap();
        assert_eq!(
            script.width, text.width,
            "{name} boxed width style independence"
        );
        assert_eq!(
            script.height, text.height,
            "{name} boxed height style independence"
        );
        assert_eq!(
            script.depth, text.depth,
            "{name} boxed depth style independence"
        );

        let boxed_x = layout(&parse(r"\boxed{x}").unwrap(), &font, MathStyle::Script).unwrap();
        let raw_x = math_hbox("x", &font, MathStyle::Display);
        assert_eq!(
            boxed_x.width.checked_sub(&raw_x.width).unwrap(),
            Dim::ratio(17, 25).unwrap(),
            "{name} boxed math hbox must not append terminal italic"
        );

        let inner = layout(&parse(r"\dfrac{1}{2}").unwrap(), &font, MathStyle::Text).unwrap();
        assert_eq!(
            text.width.checked_sub(&inner.width).unwrap(),
            Dim::ratio(17, 25).unwrap(),
            "{name} boxed 6.8pt width at 10pt"
        );
        assert_eq!(
            text.height.checked_sub(&inner.height).unwrap(),
            Dim::ratio(17, 50).unwrap(),
            "{name} boxed 3.4pt top extent at 10pt"
        );
        assert_eq!(
            text.depth.checked_sub(&inner.depth).unwrap(),
            Dim::ratio(17, 50).unwrap(),
            "{name} boxed 3.4pt bottom extent at 10pt"
        );
    }
}

#[test]
fn cancel_marks_preserve_width_and_use_physical_line_geometry() {
    let sizes = [6_i64, 10, 20, 40];
    for (name, font) in profile_fonts() {
        let clean = math_hbox("x", &font, MathStyle::Text);
        for size in sizes {
            let root = Dim::from_i64(size);
            let cancelled = layout_with_em_size_pt(
                &parse(r"\cancel{x}").unwrap(),
                &font,
                MathStyle::Text,
                &root,
            )
            .unwrap();
            assert_eq!(cancelled.width, clean.width, "{name} {size}pt cancel width");
            assert_eq!(
                cancelled.height.checked_sub(&clean.height).unwrap(),
                Dim::ratio(1, size).unwrap(),
                "{name} {size}pt cancel top overshoot"
            );
            assert_eq!(
                cancelled.depth.checked_sub(&clean.depth).unwrap(),
                Dim::ratio(1, size).unwrap(),
                "{name} {size}pt cancel bottom overshoot"
            );
            let mut marks = Vec::new();
            lines(&cancelled, &mut marks);
            assert_eq!(marks.len(), 1, "{name} cancel line count");
            let expected_thickness = Dim::ratio(2, 5 * size).unwrap();
            assert_eq!(
                marks[0].4, &expected_thickness,
                "{name} {size}pt cancel thinlines thickness"
            );
            let overshoot = Dim::ratio(1, size).unwrap();
            let x1 = -overshoot.clone();
            let y1 = -clean.depth.checked_add(&overshoot).unwrap();
            let x2 = clean.width.checked_add(&overshoot).unwrap();
            let y2 = clean.height.checked_add(&overshoot).unwrap();
            assert_eq!(marks[0].0, &x1, "{name} {size}pt cancel x1");
            assert_eq!(marks[0].1, &y1, "{name} {size}pt cancel y1");
            assert_eq!(marks[0].2, &x2, "{name} {size}pt cancel x2");
            assert_eq!(marks[0].3, &y2, "{name} {size}pt cancel y2");
        }
    }
}

#[test]
fn cancelto_keeps_overlap_width_and_uses_default_smaller_value_style() {
    for (name, font) in profile_fonts() {
        let params = MathParams::from_font(&font).unwrap();
        let clean = math_hbox("x", &font, MathStyle::Display);
        let display = layout(
            &parse(r"\cancelto{A}{x}").unwrap(),
            &font,
            MathStyle::Display,
        )
        .unwrap();
        assert_eq!(display.width, clean.width, "{name} display cancelto width");
        let mut display_scales = Vec::new();
        glyph_scales(&display, &mut display_scales);
        assert_eq!(
            display_scales.len(),
            2,
            "{name} display cancelto glyph count"
        );
        assert_eq!(
            display_scales[1],
            params.scale(MathStyle::Text),
            "{name} display cancelto value must be textstyle"
        );
        let mut marks = Vec::new();
        lines(&display, &mut marks);
        assert_eq!(marks.len(), 3, "{name} cancelto shaft plus arrowhead");
        let thin = Dim::ratio(1, 25).unwrap();
        assert!(
            marks.iter().all(|mark| mark.4 == &thin),
            "{name} cancelto must use physical 0.4pt thinlines at 10pt"
        );

        let text = layout(&parse(r"\cancelto{A}{x}").unwrap(), &font, MathStyle::Text).unwrap();
        let mut text_scales = Vec::new();
        glyph_scales(&text, &mut text_scales);
        assert_eq!(text_scales.len(), 2, "{name} text cancelto glyph count");
        assert_eq!(
            text_scales[1],
            params.scale(MathStyle::Script),
            "{name} text cancelto value must be scriptstyle"
        );
    }
}

#[test]
fn boxed_colorbox_and_cancelto_degrade_missing_glyph_once() {
    let font = common::stix_two_math().expect("STIX Two Math");
    for source in [
        format!(r"\boxed{{{MISSING}}}"),
        format!(r"\colorbox{{yellow}}{{{MISSING}}}"),
        format!(r"\cancelto{{{MISSING}}}{{x}}"),
    ] {
        let ast = parse(&source).expect("degradation source");
        let out = layout_with_diagnostics(&ast, &font, MathStyle::Text).expect("degraded layout");
        assert_eq!(
            out.diagnostics,
            vec![LayoutDiagnostic::MissingGlyph { ch: MISSING }],
            "{source} must diagnose the missing scalar exactly once"
        );
        assert!(
            !out.math_box.width.is_zero(),
            "{source} must retain box structure"
        );
    }
}
