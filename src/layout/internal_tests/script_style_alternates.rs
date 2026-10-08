use crate::test_support::{
    layout, parse, BoxContent, Dim, MathBox, MathFont, MathParams, MathStyle,
};

#[derive(Clone, Copy)]
struct ScriptProfile {
    name: &'static str,
    bytes: &'static [u8],
    covered: char,
    script_percent: i64,
    scriptscript_percent: i64,
    base_gid: u16,
    script_gid: u16,
    scriptscript_gid: u16,
}

const PROFILES: &[ScriptProfile] = &[
    ScriptProfile {
        name: "stix",
        bytes: include_bytes!(
            "../../../tests/fixtures/fonts/stix-two-math/STIXTwoMath-Regular.otf",
        ),
        covered: '\u{210E}',
        script_percent: 70,
        scriptscript_percent: 55,
        base_gid: 1224,
        script_gid: 4429,
        scriptscript_gid: 4678,
    },
    ScriptProfile {
        name: "libertinus",
        bytes: include_bytes!(
            "../../../tests/fixtures/fonts/libertinus-math/LibertinusMath-Regular.otf",
        ),
        covered: '\u{2032}',
        script_percent: 80,
        scriptscript_percent: 60,
        base_gid: 1781,
        script_gid: 3791,
        scriptscript_gid: 3791,
    },
    ScriptProfile {
        name: "fira",
        bytes: include_bytes!("../../../tests/fixtures/fonts/fira-math/FiraMath-Regular.otf"),
        covered: '\u{2032}',
        script_percent: 72,
        scriptscript_percent: 58,
        base_gid: 556,
        script_gid: 1567,
        scriptscript_gid: 1574,
    },
    ScriptProfile {
        name: "dejavu",
        bytes: include_bytes!("../../../tests/fixtures/fonts/dejavu-math/DejaVuMathTeXGyre.ttf"),
        covered: '2',
        script_percent: 80,
        scriptscript_percent: 65,
        base_gid: 21,
        script_gid: 1237,
        scriptscript_gid: 1238,
    },
];

fn font(profile: ScriptProfile) -> MathFont {
    MathFont::from_bytes(profile.bytes)
        .unwrap_or_else(|error| panic!("{}: font construction failed: {error}", profile.name))
}

fn glyph_box(source: &str, style: MathStyle, font: &MathFont) -> MathBox {
    let ast = parse(source).unwrap_or_else(|error| panic!("parse {source:?}: {error}"));
    let bx = layout(&ast, font, style)
        .unwrap_or_else(|error| panic!("layout {source:?} in {}: {error}", style.gold()));
    assert!(
        matches!(&bx.content, BoxContent::Glyph { .. }),
        "single-glyph fixture must remain a glyph box: {source:?} / {}",
        style.gold()
    );
    bx
}

fn glyph_id(source: &str, style: MathStyle, font: &MathFont) -> u16 {
    let bx = glyph_box(source, style, font);
    let BoxContent::Glyph { glyph_id, .. } = bx.content else {
        unreachable!("glyph_box already proved this is a glyph")
    };
    glyph_id
}

fn glyph_ids(b: &MathBox, out: &mut Vec<u16>) {
    match &b.content {
        BoxContent::Glyph { glyph_id, .. } => out.push(*glyph_id),
        BoxContent::HList(children)
        | BoxContent::VList(children)
        | BoxContent::Overlap(children) => {
            for child in children {
                glyph_ids(child, out);
            }
        }
        BoxContent::Color(_, inner)
        | BoxContent::BackColor(_, inner)
        | BoxContent::Frame { inner, .. } => glyph_ids(inner, out),
        BoxContent::Empty | BoxContent::Rule | BoxContent::Kern(_) | BoxContent::Line { .. } => {}
    }
}

fn layout_glyph_ids(source: &str, font: &MathFont) -> Vec<u16> {
    let ast = parse(source).unwrap_or_else(|error| panic!("parse {source:?}: {error}"));
    let bx = layout(&ast, font, MathStyle::Text)
        .unwrap_or_else(|error| panic!("layout {source:?}: {error}"));
    let mut out = Vec::new();
    glyph_ids(&bx, &mut out);
    out
}

fn expected_scale(percent: i64) -> Dim {
    Dim::ratio(percent, 100).expect("fixture percentage over 100 fits Dim")
}

fn assert_selected_metrics(
    profile: ScriptProfile,
    font: &MathFont,
    style: MathStyle,
    percent: i64,
) {
    let source = profile.covered.to_string();
    let bx = glyph_box(&source, style, font);
    let BoxContent::Glyph {
        glyph_id, scale, ..
    } = &bx.content
    else {
        unreachable!("glyph_box already proved this is a glyph")
    };

    let expected_gid = match style.script_level() {
        1 => profile.script_gid,
        2 => profile.scriptscript_gid,
        other => panic!(
            "{}: metric fixture requires script level 1 or 2, got {other}",
            profile.name
        ),
    };
    assert_eq!(
        *glyph_id,
        expected_gid,
        "{} / {}: pinned ssty glyph",
        profile.name,
        style.gold()
    );

    let expected_scale = expected_scale(percent);
    assert_eq!(
        scale,
        &expected_scale,
        "{} / {}: MathConstants script scale",
        profile.name,
        style.gold()
    );

    let selected = font
        .glyph_id(profile.covered, *glyph_id)
        .unwrap_or_else(|error| panic!("{}: selected glyph metrics: {error}", profile.name));
    let selected_italic = font.italic_correction(*glyph_id);
    assert_eq!(
        bx.width,
        selected
            .advance
            .checked_mul(&expected_scale)
            .expect("selected advance scaling"),
        "{} / {}: width must use the substituted glyph metrics",
        profile.name,
        style.gold()
    );
    assert_eq!(
        bx.height,
        selected
            .height
            .checked_mul(&expected_scale)
            .expect("selected height scaling"),
        "{} / {}: height must use the substituted glyph metrics",
        profile.name,
        style.gold()
    );
    assert_eq!(
        bx.depth,
        selected
            .depth
            .checked_mul(&expected_scale)
            .expect("selected depth scaling"),
        "{} / {}: depth must use the substituted glyph metrics",
        profile.name,
        style.gold()
    );
    assert_eq!(
        bx.italic,
        selected_italic
            .checked_mul(&expected_scale)
            .expect("selected italic scaling"),
        "{} / {}: italic correction must use the substituted glyph",
        profile.name,
        style.gold()
    );

    let base = font
        .glyph(profile.covered)
        .unwrap_or_else(|error| panic!("{}: base glyph metrics: {error}", profile.name));
    assert_ne!(
        *glyph_id,
        base.glyph_id,
        "{} / {}: covered glyph must select an ssty alternate",
        profile.name,
        style.gold()
    );
    let wrong_width = base
        .advance
        .checked_mul(&expected_scale)
        .expect("base advance scaling");
    assert_ne!(
        bx.width,
        wrong_width,
        "{} / {}: fixture must distinguish substituted from base metrics",
        profile.name,
        style.gold()
    );
}

#[test]
fn script_scales_and_ssty_alternates_follow_each_real_font() {
    for &profile in PROFILES {
        let font = font(profile);
        let params = MathParams::from_font(&font)
            .unwrap_or_else(|error| panic!("{}: MathParams: {error}", profile.name));
        assert_eq!(
            params.scale(MathStyle::Script),
            expected_scale(profile.script_percent),
            "{}: scriptPercentScaleDown",
            profile.name
        );
        assert_eq!(
            params.scale(MathStyle::ScriptScript),
            expected_scale(profile.scriptscript_percent),
            "{}: scriptScriptPercentScaleDown",
            profile.name
        );

        let base = font
            .glyph(profile.covered)
            .unwrap_or_else(|error| panic!("{}: base glyph: {error}", profile.name))
            .glyph_id;
        let source = profile.covered.to_string();
        let text = glyph_id(&source, MathStyle::Text, &font);
        let script = glyph_id(&source, MathStyle::Script, &font);
        let scriptscript = glyph_id(&source, MathStyle::ScriptScript, &font);

        assert_eq!(
            base, profile.base_gid,
            "{}: pinned base glyph",
            profile.name
        );
        assert_eq!(
            text, profile.base_gid,
            "{}: ssty must be inactive at level 0",
            profile.name
        );
        assert_eq!(
            script, profile.script_gid,
            "{}: level-1 ssty glyph",
            profile.name
        );
        assert_eq!(
            scriptscript, profile.scriptscript_gid,
            "{}: level-2 ssty glyph or Single fallback",
            profile.name
        );

        assert_selected_metrics(profile, &font, MathStyle::Script, profile.script_percent);
        assert_selected_metrics(
            profile,
            &font,
            MathStyle::ScriptScript,
            profile.scriptscript_percent,
        );
    }
}

#[test]
fn uncovered_ssty_glyph_keeps_its_base_glyph_but_not_text_scale() {
    for &profile in PROFILES {
        let font = font(profile);
        let base = font
            .glyph('*')
            .unwrap_or_else(|error| panic!("{}: asterisk glyph: {error}", profile.name))
            .glyph_id;
        let script = glyph_box("*", MathStyle::Script, &font);
        let scriptscript = glyph_box("*", MathStyle::ScriptScript, &font);

        let BoxContent::Glyph {
            glyph_id: script_id,
            scale: script_scale,
            ..
        } = &script.content
        else {
            unreachable!("glyph_box already proved this is a glyph")
        };
        let BoxContent::Glyph {
            glyph_id: scriptscript_id,
            scale: scriptscript_scale,
            ..
        } = &scriptscript.content
        else {
            unreachable!("glyph_box already proved this is a glyph")
        };

        assert_eq!(*script_id, base, "{}: uncovered level 1", profile.name);
        assert_eq!(
            *scriptscript_id, base,
            "{}: uncovered level 2",
            profile.name
        );
        assert_eq!(
            script_scale,
            &expected_scale(profile.script_percent),
            "{}: uncovered glyph still receives script scaling",
            profile.name
        );
        assert_eq!(
            scriptscript_scale,
            &expected_scale(profile.scriptscript_percent),
            "{}: uncovered glyph still receives scriptscript scaling",
            profile.name
        );
    }
}

#[test]
fn actual_script_nesting_selects_level_one_then_level_two() {
    let profile = PROFILES[0];
    assert_eq!(
        profile.name, "stix",
        "fixture order is part of this focused test"
    );
    let font = font(profile);

    let base = glyph_id("x", MathStyle::Text, &font);
    let two_script = glyph_id("2", MathStyle::Script, &font);
    let three_scriptscript = glyph_id("3", MathStyle::ScriptScript, &font);
    assert_eq!(two_script, 4275, "STIX pinned level-1 digit alternate");
    assert_eq!(
        three_scriptscript, 4525,
        "STIX pinned level-2 digit alternate"
    );

    assert_eq!(layout_glyph_ids("x^2", &font), vec![base, 4275]);
    assert_eq!(layout_glyph_ids("x_2", &font), vec![base, 4275]);
    assert_eq!(layout_glyph_ids("x^{2^3}", &font), vec![base, 4275, 4525]);
}
