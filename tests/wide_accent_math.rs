mod common;

use texpose::{
    layout, layout_with_diagnostics, parse, styled_char, BoxContent, Dim, MathBox, MathFont,
    MathParams, MathStyle, TextStyle,
};

const LIBERTINUS_MATH_OTF: &[u8] =
    include_bytes!("fixtures/fonts/libertinus-math/LibertinusMath-Regular.otf");
const FIRA_MATH_OTF: &[u8] = include_bytes!("fixtures/fonts/fira-math/FiraMath-Regular.otf");
const DEJAVU_MATH_TTF: &[u8] = include_bytes!("fixtures/fonts/dejavu-math/DejaVuMathTeXGyre.ttf");

fn add(a: &Dim, b: &Dim) -> Dim {
    a.checked_add(b).unwrap()
}
fn sub(a: &Dim, b: &Dim) -> Dim {
    a.checked_sub(b).unwrap()
}
fn mul(a: &Dim, b: &Dim) -> Dim {
    a.checked_mul(b).unwrap()
}
fn div(a: &Dim, b: &Dim) -> Dim {
    a.checked_div(b).unwrap()
}

fn single_glyph_x(bx: &MathBox) -> Option<(Dim, char, u16)> {
    match &bx.content {
        BoxContent::Glyph { ch, glyph_id, .. } => Some((Dim::zero(), *ch, *glyph_id)),

        BoxContent::HList(children) => {
            let mut x = Dim::zero();
            let mut found = None;

            for child in children {
                if let Some((inner_x, ch, glyph_id)) = single_glyph_x(child) {
                    if found.is_some() {
                        return None;
                    }
                    found = Some((add(&x, &inner_x), ch, glyph_id));
                }
                x = add(&x, &child.width);
            }
            found
        }

        BoxContent::Color(_, inner)
        | BoxContent::BackColor(_, inner)
        | BoxContent::Frame { inner, .. } => single_glyph_x(inner),

        _ => None,
    }
}

fn accent_branches(source: &str, style: MathStyle, font: &MathFont) -> (MathBox, MathBox, MathBox) {
    let ast = parse(source).expect("parse accent case");
    let tree = layout(&ast, font, style).expect("layout accent case");
    let BoxContent::Overlap(children) = &tree.content else {
        panic!("accent did not produce an overlap");
    };
    let [accent, base] = children.as_slice() else {
        panic!("top-accent overlap did not contain accent then base");
    };
    (tree.clone(), base.clone(), accent.clone())
}

fn under_accent_branches(
    source: &str,
    style: MathStyle,
    font: &MathFont,
) -> (MathBox, MathBox, MathBox) {
    let ast = parse(source).expect("parse under-accent case");
    let tree = layout(&ast, font, style).expect("layout under-accent case");
    let BoxContent::Overlap(children) = &tree.content else {
        panic!("under-accent did not produce an overlap");
    };
    let [base, accent] = children.as_slice() else {
        panic!("under-accent overlap did not contain base then accent");
    };
    (tree.clone(), base.clone(), accent.clone())
}

fn has_raw_horizontal_construction(font_bytes: &[u8], ch: char) -> bool {
    let face = ttf_parser::Face::parse(font_bytes, 0).expect("verification font face");
    let Some(glyph) = face.glyph_index(ch) else {
        return false;
    };
    let Some(math) = face.tables().math else {
        return false;
    };
    let Some(variants) = math.variants else {
        return false;
    };
    variants.horizontal_constructions.get(glyph).is_some()
}

fn raw_horizontal_variant_for_target(font_bytes: &[u8], ch: char, target: &Dim) -> u16 {
    let face = ttf_parser::Face::parse(font_bytes, 0).expect("verification font face");
    let glyph = face.glyph_index(ch).expect("accent construction glyph");
    let math = face.tables().math.expect("MATH table");
    let variants = math.variants.expect("MathVariants table");
    let construction = variants
        .horizontal_constructions
        .get(glyph)
        .expect("horizontal accent construction");
    let units_per_em = face.units_per_em();

    let mut best_glyph = glyph.0;
    let mut best_extent = Dim::zero();
    for index in 0..construction.variants.len() {
        let variant = construction
            .variants
            .get(index)
            .expect("MATH variant record");
        let extent = Dim::from_font_units(i64::from(variant.advance_measurement), units_per_em)
            .expect("variant advance");
        let better = match (&best_extent >= target, &extent >= target) {
            (false, false) => extent > best_extent,
            (false, true) => true,
            (true, false) => false,
            (true, true) => extent < best_extent,
        };
        if better {
            best_glyph = variant.variant_glyph.0;
            best_extent = extent;
        }
    }
    assert!(
        best_extent >= *target,
        "fixture must expose a prebuilt horizontal variant satisfying the target"
    );
    best_glyph
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct GlyphPosition {
    ch: char,
    x: Dim,
    baseline: Dim,
}

fn trace_glyphs(bx: &MathBox, x: &Dim, parent_baseline: &Dim, out: &mut Vec<GlyphPosition>) {
    let baseline = add(parent_baseline, &bx.shift);
    match &bx.content {
        BoxContent::Glyph { ch, .. } => out.push(GlyphPosition {
            ch: *ch,
            x: x.clone(),
            baseline,
        }),
        BoxContent::HList(children) => {
            let mut child_x = x.clone();
            for child in children {
                trace_glyphs(child, &child_x, &baseline, out);
                child_x = add(&child_x, &child.width);
            }
        }
        BoxContent::VList(children) => {
            if let Some((first, rest)) = children.split_first() {
                let mut child_baseline = baseline.clone();
                trace_glyphs(first, x, &child_baseline, out);
                child_baseline = sub(&child_baseline, &first.depth);
                for child in rest {
                    child_baseline = sub(&child_baseline, &child.height);
                    trace_glyphs(child, x, &child_baseline, out);
                    child_baseline = sub(&child_baseline, &child.depth);
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
        BoxContent::Empty | BoxContent::Kern(_) | BoxContent::Rule | BoxContent::Line { .. } => {}
    }
}

fn body_script_trace(source: &str, font: &MathFont) -> Vec<GlyphPosition> {
    let ast = parse(source).expect("scripted accent source");
    let bx = layout(&ast, font, MathStyle::Text).expect("scripted accent layout");
    let wanted = [
        styled_char('x', TextStyle::It),
        styled_char('i', TextStyle::It),
        '2',
    ];
    let mut all = Vec::new();
    trace_glyphs(&bx, &Dim::zero(), &Dim::zero(), &mut all);
    all.into_iter()
        .filter(|glyph| wanted.contains(&glyph.ch))
        .collect()
}

#[test]
fn hat_tilde_accents_follow_math_attachment_and_accent_base_height() {
    let font = common::stix_two_math().expect("STIX Two Math fixture");
    let params = MathParams::from_font(&font).expect("OpenType MATH constants");
    let italic_j = styled_char('J', TextStyle::It);
    let base_metrics = font.glyph(italic_j).expect("italic J glyph");
    let expected_raise = sub(&base_metrics.height, &params.accent_base_height).clamp_nonneg();

    for source in [r"\hat J", r"\widehat J", r"\tilde J", r"\widetilde J"] {
        let (_, base, accent) = accent_branches(source, MathStyle::Text, &font);
        let (base_x, _, base_glyph_id) = single_glyph_x(&base).expect("single base glyph");
        let (accent_x, accent_ch, accent_glyph_id) =
            single_glyph_x(&accent).expect("single accent glyph");

        assert_eq!(base_glyph_id, base_metrics.glyph_id, "{source}: base glyph");
        assert!(
            matches!(accent_ch, '\u{0302}' | '\u{0303}'),
            "{source}: MATH accent source must be the combining construction"
        );
        assert!(
            sub(&accent.shift, &base.shift).eq_dim(&expected_raise),
            "{source}: accent raise was {}, expected {}",
            sub(&accent.shift, &base.shift).to_dec_string(),
            expected_raise.to_dec_string()
        );

        let accent_metrics = font
            .glyph_id(accent_ch, accent_glyph_id)
            .expect("selected accent metrics");
        let base_attachment = font
            .top_accent_attachment(base_metrics.glyph_id)
            .unwrap_or_else(|| div(&base_metrics.advance, &Dim::from_i64(2)));
        let accent_attachment = font
            .top_accent_attachment(accent_glyph_id)
            .unwrap_or_else(|| div(&accent_metrics.advance, &Dim::from_i64(2)));
        let expected_x = sub(&base_attachment, &accent_attachment);
        let actual_x = sub(&accent_x, &base_x);
        assert!(
            actual_x.eq_dim(&expected_x),
            "{source}: accent x offset was {}, expected {}",
            actual_x.to_dec_string(),
            expected_x.to_dec_string()
        );
    }
}

#[test]
fn bar_uses_unicode_math_combining_macron_across_verification_fonts() {
    for (profile, bytes) in [
        ("stix", common::STIX_TWO_MATH_OTF),
        ("libertinus", LIBERTINUS_MATH_OTF),
        ("fira", FIRA_MATH_OTF),
    ] {
        let face = ttf_parser::Face::parse(bytes, 0).expect("verification font face");
        let combining = face
            .glyph_index('\u{0304}')
            .expect("verification font combining macron");
        let spacing = face
            .glyph_index('¯')
            .expect("verification font spacing macron");
        assert_ne!(
            combining, spacing,
            "{profile}: fixture must distinguish combining and spacing macron glyphs"
        );

        let font = MathFont::from_bytes(bytes)
            .unwrap_or_else(|error| panic!("{profile}: MATH font construction failed: {error}"));
        let (_, _, accent) = accent_branches(r"\bar y", MathStyle::Text, &font);
        let (_, accent_ch, accent_glyph_id) =
            single_glyph_x(&accent).expect("bar accent must be a single glyph");

        assert_eq!(
            accent_ch, '\u{0304}',
            "{profile}: unicode-math defines \\bar as U+0304"
        );
        assert_eq!(
            accent_glyph_id, combining.0,
            "{profile}: bar must select the combining-macron glyph"
        );
        assert_ne!(
            accent_glyph_id, spacing.0,
            "{profile}: spacing macron must not shadow the combining accent"
        );
    }
}

#[test]
fn script_style_hat_attachment_uses_script_scale() {
    let font = common::stix_two_math().expect("STIX Two Math fixture");
    let params = MathParams::from_font(&font).expect("OpenType MATH constants");
    let scale = params.scale(MathStyle::Script);
    assert!(
        !scale.eq_dim(&Dim::one()),
        "fixture requires a non-unit script scale"
    );

    let (_, base, accent) = accent_branches(r"\hat J", MathStyle::Script, &font);
    let (base_x, _, base_id) = single_glyph_x(&base).expect("single script base glyph");
    let (accent_x, accent_ch, accent_id) =
        single_glyph_x(&accent).expect("single script accent glyph");
    let base_metrics = font.glyph_id('J', base_id).expect("script base metrics");
    let accent_metrics = font
        .glyph_id(accent_ch, accent_id)
        .expect("script accent metrics");
    let base_attachment = font.top_accent_attachment(base_id).map_or_else(
        || div(&mul(&base_metrics.advance, &scale), &Dim::from_i64(2)),
        |value| mul(&value, &scale),
    );
    let accent_attachment = font.top_accent_attachment(accent_id).map_or_else(
        || div(&mul(&accent_metrics.advance, &scale), &Dim::from_i64(2)),
        |value| mul(&value, &scale),
    );
    let expected = sub(&base_attachment, &accent_attachment);
    let actual = sub(&accent_x, &base_x);
    assert!(
        actual.eq_dim(&expected),
        "script hat x offset was {}, expected {}",
        actual.to_dec_string(),
        expected.to_dec_string()
    );
}

#[test]
fn wide_accents_select_semantic_math_constructions_by_advance_across_fonts() {
    type AccentCase = (&'static str, char, char, i64);
    type ConstructionProfile = (&'static str, &'static [u8], &'static [AccentCase]);

    let construction_profiles: &[ConstructionProfile] = &[
        (
            "stix",
            common::STIX_TWO_MATH_OTF,
            &[
                (r"\widehat{j\hspace{0.1225em}}", '\u{0302}', 'ˆ', 5735),
                (r"\widetilde{j\hspace{0.1225em}}", '\u{0303}', '˜', 5735),
            ],
        ),
        (
            "libertinus",
            LIBERTINUS_MATH_OTF,
            &[
                (r"\widehat{j\hspace{0.2435em}}", '\u{0302}', 'ˆ', 5735),
                (r"\widetilde{j\hspace{0.2435em}}", '\u{0303}', '˜', 5735),
            ],
        ),
        (
            "dejavu",
            DEJAVU_MATH_TTF,
            &[
                (r"\widehat{j\hspace{0.1195em}}", '\u{0302}', 'ˆ', 6765),
                (r"\widetilde{j\hspace{0.1385em}}", '\u{0303}', '˜', 6955),
            ],
        ),
    ];

    for &(profile, bytes, cases) in construction_profiles {
        let font = MathFont::from_bytes(bytes)
            .unwrap_or_else(|error| panic!("{profile}: MATH font construction failed: {error}"));

        for &(source, combining, spacing, target_numerator) in cases {
            let target = Dim::ratio(target_numerator, 10_000).expect("static target ratio");
            assert!(
                has_raw_horizontal_construction(bytes, combining),
                "{profile}/{source}: combining candidate must expose a MATH construction"
            );
            assert!(
                !has_raw_horizontal_construction(bytes, spacing),
                "{profile}/{source}: spacing candidate must not shadow the combining construction"
            );

            let expected = raw_horizontal_variant_for_target(bytes, combining, &target);
            let face = ttf_parser::Face::parse(bytes, 0).expect("verification font face");
            let physical_advance = face
                .glyph_hor_advance(ttf_parser::GlyphId(expected))
                .expect("selected accent horizontal advance");
            let physical_advance =
                Dim::from_font_units(i64::from(physical_advance), face.units_per_em())
                    .expect("selected accent advance");
            assert!(
                physical_advance < target,
                "{profile}/{source}: fixture must distinguish glyph advance from MATH advanceMeasurement"
            );

            let (_, base, accent) = accent_branches(source, MathStyle::Text, &font);
            assert!(
                base.width.eq_dim(&target),
                "{profile}/{source}: target base width was {}, expected {}",
                base.width.to_dec_string(),
                target.to_dec_string()
            );
            let (_, actual_ch, actual) = single_glyph_x(&accent)
                .unwrap_or_else(|| panic!("{profile}/{source}: single prebuilt accent glyph"));
            assert_eq!(
                actual_ch, combining,
                "{profile}/{source}: semantic construction source"
            );
            assert_eq!(
                actual, expected,
                "{profile}/{source}: MATH advanceMeasurement selection"
            );
        }
    }
}

#[test]
fn fira_overbrace_prebuilt_variant_follows_math_advance_measurement() {
    let font = MathFont::from_bytes(FIRA_MATH_OTF).expect("Fira Math fixture");
    let (_, base, accent) = accent_branches(r"\overbrace{a+b+c}", MathStyle::Text, &font);
    let source = '\u{23DE}';
    let expected = raw_horizontal_variant_for_target(FIRA_MATH_OTF, source, &base.width);
    let (_, actual_ch, actual) =
        single_glyph_x(&accent).expect("Fira overbrace must select a prebuilt variant");

    assert_eq!(actual_ch, source);
    assert_eq!(
        actual, expected,
        "Fira overbrace must select by MATH advanceMeasurement"
    );
    assert_eq!(
        actual, 2016,
        "pinned Fira fixture must keep the G12 overbrace selection signature"
    );
}

#[test]
fn wide_accents_fall_back_to_the_base_glyph_when_the_font_has_no_construction() {
    let font = MathFont::from_bytes(FIRA_MATH_OTF).expect("Fira Math fixture");

    for (source, combining, spacing) in [
        (r"\widehat{XYZ}", '\u{0302}', 'ˆ'),
        (r"\widetilde{XYZ}", '\u{0303}', '˜'),
    ] {
        assert!(
            !has_raw_horizontal_construction(FIRA_MATH_OTF, combining),
            "{source}: Fira combining candidate unexpectedly gained a MATH construction"
        );
        assert!(
            !has_raw_horizontal_construction(FIRA_MATH_OTF, spacing),
            "{source}: Fira spacing candidate unexpectedly gained a MATH construction"
        );

        let ast = parse(source).expect("fallback accent source");
        let output = layout_with_diagnostics(&ast, &font, MathStyle::Text)
            .unwrap_or_else(|error| panic!("{source}: fallback layout failed: {error}"));
        assert!(
            output.diagnostics.is_empty(),
            "{source}: absent MATH construction is a supported deterministic fallback"
        );
        let BoxContent::Overlap(children) = &output.math_box.content else {
            panic!("{source}: accent did not produce an overlap");
        };
        let [accent, _] = children.as_slice() else {
            panic!("{source}: accent overlap did not contain exactly two branches");
        };
        let (_, actual_ch, actual_id) =
            single_glyph_x(accent).unwrap_or_else(|| panic!("{source}: base-glyph fallback"));
        let expected_id = ttf_parser::Face::parse(FIRA_MATH_OTF, 0)
            .expect("Fira verification face")
            .glyph_index(combining)
            .expect("Fira combining accent glyph")
            .0;
        assert_eq!(actual_ch, combining, "{source}: fallback source glyph");
        assert_eq!(
            actual_id, expected_id,
            "{source}: absence of MathVariants must not synthesize stretching"
        );
    }
}

#[test]
fn compound_accent_base_uses_the_geometric_center_attachment_fallback() {
    let font = common::stix_two_math().expect("STIX Two Math fixture");
    let (_, base, accent) = accent_branches(r"\hat{AB}", MathStyle::Text, &font);
    let (accent_x, accent_ch, accent_id) =
        single_glyph_x(&accent).expect("single fixed accent glyph");
    let accent_metrics = font
        .glyph_id(accent_ch, accent_id)
        .expect("fixed accent metrics");
    let accent_attachment = font
        .top_accent_attachment(accent_id)
        .unwrap_or_else(|| div(&accent_metrics.advance, &Dim::from_i64(2)));
    let expected_x = sub(&div(&base.width, &Dim::from_i64(2)), &accent_attachment);

    assert!(
        accent_x.eq_dim(&expected_x),
        "compound base must use its advance-width center: {} != {}",
        accent_x.to_dec_string(),
        expected_x.to_dec_string()
    );
}

#[test]
fn top_and_bottom_stretchy_accents_use_accent_geometry_not_bar_spacing() {
    let font = common::stix_two_math().expect("STIX Two Math fixture");
    let params = MathParams::from_font(&font).expect("OpenType MATH constants");
    let base_metrics = font
        .glyph(styled_char('J', TextStyle::It))
        .expect("italic J glyph");
    let expected_top_raise = sub(&base_metrics.height, &params.accent_base_height).clamp_nonneg();

    for source in [r"\overrightarrow{J}", r"\overbrace{J}"] {
        let (_, base, accent) = accent_branches(source, MathStyle::Text, &font);
        let actual = sub(&accent.shift, &base.shift);
        assert!(
            actual.eq_dim(&expected_top_raise),
            "{source}: top accent raise was {}, expected {}",
            actual.to_dec_string(),
            expected_top_raise.to_dec_string()
        );
    }

    for source in [r"\underleftarrow{J}", r"\underbrace{J}"] {
        let (_, base, accent) = under_accent_branches(source, MathStyle::Text, &font);
        let expected = -add(&base.depth, &accent.height);
        let actual = sub(&accent.shift, &base.shift);
        assert!(
            actual.eq_dim(&expected),
            "{source}: bottom accent shift was {}, expected {}",
            actual.to_dec_string(),
            expected.to_dec_string()
        );
    }
}

#[test]
fn scripts_on_a_character_accent_keep_the_unaccented_script_geometry() {
    let font = common::stix_two_math().expect("STIX Two Math fixture");
    let plain = body_script_trace(r"x_i^2", &font);
    let accented = body_script_trace(r"\widehat{x}_i^2", &font);
    assert_eq!(
        plain.len(),
        3,
        "fixture must expose nucleus, subscript, superscript"
    );
    assert_eq!(
        accented, plain,
        "accent must not become the script-placement nucleus"
    );
}
