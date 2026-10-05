use texpose::{layout, parse, BoxContent, Dim, MathBox, MathFont, MathStyle};

const STIX: &[u8] = include_bytes!("fixtures/fonts/stix-two-math/STIXTwoMath-Regular.otf");
const LIBERTINUS: &[u8] =
    include_bytes!("fixtures/fonts/libertinus-math/LibertinusMath-Regular.otf");
const FIRA: &[u8] = include_bytes!("fixtures/fonts/fira-math/FiraMath-Regular.otf");
const DEJAVU: &[u8] = include_bytes!("fixtures/fonts/dejavu-math/DejaVuMathTeXGyre.ttf");

#[derive(Clone, Debug, PartialEq, Eq)]
struct GlyphPosition {
    glyph_id: u16,
    x: Dim,
    baseline: Dim,
}

fn layout_source(source: &str, style: MathStyle, font: &MathFont) -> MathBox {
    let ast = parse(source).unwrap_or_else(|error| panic!("parse {source:?}: {error}"));
    layout(&ast, font, style)
        .unwrap_or_else(|error| panic!("layout {source:?} in {}: {error}", style.gold()))
}

fn trace_glyphs(math_box: &MathBox, x: &Dim, parent_baseline: &Dim, out: &mut Vec<GlyphPosition>) {
    let baseline = parent_baseline
        .checked_add(&math_box.shift)
        .expect("fixture baseline arithmetic");
    match &math_box.content {
        BoxContent::Glyph { glyph_id, .. } => out.push(GlyphPosition {
            glyph_id: *glyph_id,
            x: x.clone(),
            baseline,
        }),
        BoxContent::HList(children) => {
            let mut child_x = x.clone();
            for child in children {
                trace_glyphs(child, &child_x, &baseline, out);
                child_x = child_x
                    .checked_add(&child.width)
                    .expect("fixture horizontal trace arithmetic");
            }
        }
        BoxContent::VList(children) => {
            if let Some((first, rest)) = children.split_first() {
                let mut child_baseline = baseline.clone();
                trace_glyphs(first, x, &child_baseline, out);
                child_baseline = child_baseline
                    .checked_sub(&first.depth)
                    .expect("fixture vertical trace arithmetic");
                for child in rest {
                    child_baseline = child_baseline
                        .checked_sub(&child.height)
                        .expect("fixture vertical trace arithmetic");
                    trace_glyphs(child, x, &child_baseline, out);
                    child_baseline = child_baseline
                        .checked_sub(&child.depth)
                        .expect("fixture vertical trace arithmetic");
                }
            }
        }
        BoxContent::Overlap(children) => {
            for child in children {
                trace_glyphs(child, x, &baseline, out);
            }
        }
        BoxContent::Color(_, inner)
        | BoxContent::BackColor(_, inner)
        | BoxContent::Frame { inner, .. } => trace_glyphs(inner, x, &baseline, out),
        BoxContent::Empty | BoxContent::Rule | BoxContent::Kern(_) | BoxContent::Line { .. } => {}
    }
}

fn glyph_positions(source: &str, font: &MathFont) -> Vec<GlyphPosition> {
    let bx = layout_source(source, MathStyle::Text, font);
    let mut out = Vec::new();
    trace_glyphs(&bx, &Dim::zero(), &Dim::zero(), &mut out);
    out
}

fn direct_glyph(source: &str, style: MathStyle, font: &MathFont) -> MathBox {
    let bx = layout_source(source, style, font);
    assert!(
        matches!(&bx.content, BoxContent::Glyph { .. }),
        "{source:?} / {} must be a direct glyph fixture",
        style.gold()
    );
    bx
}

#[test]
fn stix_superscript_math_kern_can_select_the_first_correction_height() {
    let font = MathFont::from_bytes(STIX).expect("STIX Two Math fixture");
    let positions = glyph_positions("7^f", &font);
    assert_eq!(positions.len(), 2);

    let base = direct_glyph("7", MathStyle::Text, &font);
    let script = direct_glyph("f", MathStyle::Script, &font);
    let BoxContent::Glyph {
        glyph_id: base_gid, ..
    } = base.content
    else {
        unreachable!("direct_glyph proved the base")
    };
    let BoxContent::Glyph {
        glyph_id: script_gid,
        ..
    } = script.content
    else {
        unreachable!("direct_glyph proved the script")
    };

    assert_eq!(base_gid, 1144, "pinned STIX digit 7");
    assert_eq!(script_gid, 4426, "pinned STIX level-1 italic f alternate");
    assert_eq!(positions[0].glyph_id, base_gid);
    assert_eq!(positions[1].glyph_id, script_gid);
    assert_eq!(
        positions[1].baseline,
        Dim::ratio(9, 25).expect("0.36"),
        "superscript baseline is established before MathKern"
    );
    assert_eq!(
        positions[1].x,
        Dim::ratio(231, 500).expect("0.462"),
        "first correction-height sum (-0.033em) must beat the second (+0.020em)"
    );
}

#[test]
fn stix_superscript_math_kern_can_select_the_second_correction_height() {
    let font = MathFont::from_bytes(STIX).expect("STIX Two Math fixture");
    let positions = glyph_positions("A^f", &font);
    assert_eq!(positions.len(), 2);

    let base = direct_glyph("A", MathStyle::Text, &font);
    let script = direct_glyph("f", MathStyle::Script, &font);
    let BoxContent::Glyph {
        glyph_id: base_gid, ..
    } = base.content
    else {
        unreachable!("direct_glyph proved the base")
    };
    let BoxContent::Glyph {
        glyph_id: script_gid,
        ..
    } = script.content
    else {
        unreachable!("direct_glyph proved the script")
    };

    assert_eq!(base_gid, 3300, "pinned STIX mathematical italic A");
    assert_eq!(script_gid, 4426, "pinned STIX level-1 italic f alternate");
    assert_eq!(positions[0].glyph_id, base_gid);
    assert_eq!(positions[1].glyph_id, script_gid);
    assert_eq!(
        positions[1].baseline,
        Dim::ratio(9, 25).expect("0.36"),
        "superscript baseline is established before MathKern"
    );
    assert_eq!(
        positions[1].x,
        Dim::ratio(611, 1000).expect("0.611"),
        "second correction-height sum (-0.070em) must beat the first (+0.058em)"
    );
}

#[test]
fn stix_subscript_math_kern_can_select_the_first_correction_height() {
    let font = MathFont::from_bytes(STIX).expect("STIX Two Math fixture");
    let positions = glyph_positions("A_A", &font);
    assert_eq!(positions.len(), 2);

    let base = direct_glyph("A", MathStyle::Text, &font);
    let script = direct_glyph("A", MathStyle::ScriptCramped, &font);
    let BoxContent::Glyph {
        glyph_id: base_gid, ..
    } = base.content
    else {
        unreachable!("direct_glyph proved the base")
    };
    let BoxContent::Glyph {
        glyph_id: script_gid,
        ..
    } = script.content
    else {
        unreachable!("direct_glyph proved the script")
    };

    assert_eq!(base_gid, 3300, "pinned STIX mathematical italic A");
    assert_eq!(script_gid, 4395, "pinned STIX level-1 italic A alternate");
    assert_eq!(positions[0].glyph_id, base_gid);
    assert_eq!(positions[1].glyph_id, script_gid);
    assert_eq!(
        positions[1].baseline,
        -Dim::ratio(21, 100).expect("0.21"),
        "subscript baseline is established before MathKern"
    );
    assert_eq!(
        positions[1].x,
        Dim::ratio(1187, 2000).expect("0.5935"),
        "first correction-height sum (-0.0875em) must beat the second (+0.0161em)"
    );
}

#[test]
fn stix_subscript_math_kern_can_select_the_second_correction_height() {
    let font = MathFont::from_bytes(STIX).expect("STIX Two Math fixture");
    let positions = glyph_positions("V_A", &font);
    assert_eq!(positions.len(), 2);

    let base = direct_glyph("V", MathStyle::Text, &font);
    let script = direct_glyph("A", MathStyle::ScriptCramped, &font);
    let BoxContent::Glyph {
        glyph_id: base_gid, ..
    } = base.content
    else {
        unreachable!("direct_glyph proved the base")
    };
    let BoxContent::Glyph {
        glyph_id: script_gid,
        ..
    } = script.content
    else {
        unreachable!("direct_glyph proved the script")
    };

    assert_eq!(base_gid, 3321, "pinned STIX mathematical italic V");
    assert_eq!(script_gid, 4395, "pinned STIX level-1 italic A alternate");
    assert_eq!(positions[0].glyph_id, base_gid);
    assert_eq!(positions[1].glyph_id, script_gid);
    assert_eq!(
        positions[1].baseline,
        -Dim::ratio(21, 100).expect("0.21"),
        "subscript baseline is established before MathKern"
    );
    assert_eq!(
        positions[1].x,
        Dim::ratio(4341, 10000).expect("0.4341"),
        "second correction-height sum (-0.2059em) must beat the first (+0.1145em)"
    );
}

#[test]
fn boxed_side_zeroes_only_its_own_math_kern_corner() {
    let font = MathFont::from_bytes(STIX).expect("STIX Two Math fixture");

    let boxed_base = glyph_positions(r"\textcolor{red}{V}_A", &font);
    assert_eq!(boxed_base.len(), 2);
    assert_eq!(
        boxed_base[0].glyph_id, 3321,
        "pinned STIX mathematical italic V"
    );
    assert_eq!(
        boxed_base[1].glyph_id, 4395,
        "pinned STIX level-1 italic A alternate"
    );
    assert_eq!(
        boxed_base[1].x,
        Dim::ratio(221, 400).expect("0.5525"),
        "boxed base contributes zero while the script top-left corner still applies"
    );

    let boxed_script = glyph_positions(r"V_{\textcolor{red}{A}}", &font);
    assert_eq!(boxed_script.len(), 2);
    assert_eq!(
        boxed_script[0].glyph_id, 3321,
        "pinned STIX mathematical italic V"
    );
    assert_eq!(
        boxed_script[1].glyph_id, 4395,
        "pinned STIX level-1 italic A alternate"
    );
    assert_eq!(
        boxed_script[1].x,
        Dim::ratio(209, 500).expect("0.418"),
        "boxed script contributes zero while the base bottom-right corner still applies"
    );
}

#[test]
fn italic_correction_moves_only_the_superscript_default_origin() {
    let font = MathFont::from_bytes(STIX).expect("STIX Two Math fixture");
    let positions = glyph_positions("h_2^3", &font);
    assert_eq!(positions.len(), 3);

    let base = direct_glyph("h", MathStyle::Text, &font);
    assert_eq!(
        base.italic,
        Dim::ratio(1, 100).expect("0.010"),
        "fixture needs a nonzero base italic correction"
    );

    assert_eq!(
        positions[1].x,
        base.width
            .checked_add(&base.italic)
            .expect("base width plus italic"),
        "superscript default origin includes base italic correction"
    );
    assert_eq!(
        positions[2].x, base.width,
        "subscript default origin must not inherit base italic correction"
    );
}

#[test]
fn profiles_without_math_kern_degrade_to_zero_corner_adjustments() {
    for (name, bytes) in [
        ("libertinus", LIBERTINUS),
        ("fira", FIRA),
        ("dejavu", DEJAVU),
    ] {
        let font = MathFont::from_bytes(bytes)
            .unwrap_or_else(|error| panic!("{name}: font construction failed: {error}"));
        let base = direct_glyph("h", MathStyle::Text, &font);
        let positions = glyph_positions("h_2^3", &font);
        assert_eq!(positions.len(), 3, "{name}");

        assert_eq!(
            positions[1].x,
            base.width
                .checked_add(&base.italic)
                .expect("base width plus italic"),
            "{name}: absent MathKern leaves superscript at default origin"
        );
        assert_eq!(
            positions[2].x, base.width,
            "{name}: absent MathKern leaves subscript at default origin"
        );
    }
}
