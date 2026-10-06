mod common;

use core::cmp::Ordering;

use texpose::{layout, parse, BoxContent, Dim, MathBox, MathFont, MathParams, MathStyle};

const FIRA: &[u8] = include_bytes!("fixtures/fonts/fira-math/FiraMath-Regular.otf");

fn add(a: &Dim, b: &Dim) -> Dim {
    a.checked_add(b).unwrap()
}
fn sub(a: &Dim, b: &Dim) -> Dim {
    a.checked_sub(b).unwrap()
}
fn mul(a: &Dim, b: &Dim) -> Dim {
    a.checked_mul(b).unwrap()
}

fn layout_source(source: &str, style: MathStyle, font: &MathFont) -> MathBox {
    let ast = parse(source).expect("parse math case");

    layout(&ast, font, style).expect("layout math case")
}

fn paired_script_shifts(bx: &MathBox) -> Option<(&Dim, &Dim)> {
    match &bx.content {
        BoxContent::Overlap(children) => {
            let mut upper = None;
            let mut lower = None;

            for child in children {
                match child.shift.cmp(&Dim::zero()) {
                    Ordering::Greater if upper.is_none() => {
                        upper = Some(&child.shift);
                    }

                    Ordering::Less if lower.is_none() => {
                        lower = Some(&child.shift);
                    }

                    _ => {}
                }
            }

            upper.zip(lower)
        }

        BoxContent::HList(children) | BoxContent::VList(children) => {
            children.iter().find_map(paired_script_shifts)
        }

        BoxContent::Color(_, inner)
        | BoxContent::BackColor(_, inner)
        | BoxContent::Frame { inner, .. } => paired_script_shifts(inner),

        BoxContent::Empty
        | BoxContent::Rule
        | BoxContent::Glyph { .. }
        | BoxContent::Kern(_)
        | BoxContent::Line { .. } => None,
    }
}

fn attached_script_shift(bx: &MathBox, upper: bool) -> Option<Dim> {
    let BoxContent::HList(children) = &bx.content else {
        return None;
    };
    children.iter().rev().find_map(|child| {
        let BoxContent::Overlap(scripts) = &child.content else {
            return None;
        };
        scripts
            .iter()
            .find(|script| {
                if upper {
                    script.shift > Dim::zero()
                } else {
                    script.shift < Dim::zero()
                }
            })
            .map(|script| script.shift.clone())
    })
}

#[test]
fn paired_scripts_obey_open_type_vertical_constraints() {
    let font = common::stix_two_math().expect("STIX Two Math fixture");

    let params = MathParams::from_font(&font).expect("OpenType MATH constants");

    let constants = font
        .face()
        .tables()
        .math
        .and_then(|math| math.constants)
        .expect("fixture font MATH constants");

    let units_per_em = font.units_per_em();

    let fu = |value: i16| Dim::from_font_units(i64::from(value), units_per_em).expect("font units");

    let scale = params.scale(MathStyle::Text);

    let upper = layout_source("3", MathStyle::Script, &font);

    let lower = layout_source("2", MathStyle::ScriptCramped, &font);

    let mut expected_upper = mul(&params.superscript_shift_up, &scale);

    let min_bottom = mul(&fu(constants.superscript_bottom_min().value), &scale);

    expected_upper = expected_upper.max_ref(&add(&upper.depth, &min_bottom));

    let mut expected_lower = mul(&params.subscript_shift_down, &scale);

    let max_top = mul(&fu(constants.subscript_top_max().value), &scale);

    expected_lower = expected_lower.max_ref(&sub(&lower.height, &max_top).clamp_nonneg());

    let gap = sub(
        &sub(&add(&expected_upper, &expected_lower), &upper.depth),
        &lower.height,
    );

    let min_gap = mul(&params.sub_superscript_gap_min, &scale);

    let legacy_upper = expected_upper.clone();

    let legacy_lower = expected_lower.clone();

    if gap.cmp(&min_gap) == Ordering::Less {
        expected_lower = add(&expected_lower, &sub(&min_gap, &gap));

        let current_bottom = sub(&expected_upper, &upper.depth);

        let paired_bottom = mul(
            &fu(constants.superscript_bottom_max_with_subscript().value),
            &scale,
        );

        if current_bottom.cmp(&paired_bottom) == Ordering::Less {
            let raise = sub(&paired_bottom, &current_bottom);
            let lowered_subscript = sub(&expected_lower, &raise);

            if lowered_subscript.cmp(&Dim::zero()) != Ordering::Less {
                expected_upper = add(&expected_upper, &raise);

                expected_lower = lowered_subscript;
            }
        }
    }

    assert!(
        !expected_upper.eq_dim(&legacy_upper) || !expected_lower.eq_dim(&legacy_lower),
        "STIX fixture must distinguish the paired-script constraint from legacy placement"
    );

    let scripted = layout_source("x_2^3", MathStyle::Text, &font);

    let (actual_upper, actual_lower) =
        paired_script_shifts(&scripted).expect("paired script overlap");

    assert!(
        actual_upper.eq_dim(&expected_upper),
        "superscript shift was {}, expected {}",
        actual_upper.to_dec_string(),
        expected_upper.to_dec_string()
    );

    let expected_lower_shift = -expected_lower.clone();

    assert!(
        actual_lower.eq_dim(&expected_lower_shift),
        "subscript shift was {}, expected {}",
        actual_lower.to_dec_string(),
        expected_lower_shift.to_dec_string()
    );
}

#[test]
fn paired_bottom_max_only_redistributes_an_actual_gap_repair() {
    let font = MathFont::from_bytes(FIRA).expect("Fira Math fixture");
    let params = MathParams::from_font(&font).expect("OpenType MATH constants");
    let constants = font
        .face()
        .tables()
        .math
        .and_then(|math| math.constants)
        .expect("fixture font MATH constants");
    let units_per_em = font.units_per_em();
    let fu = |value: i16| Dim::from_font_units(i64::from(value), units_per_em).expect("font units");
    let scale = params.scale(MathStyle::Text);

    let upper = layout_source("F", MathStyle::Script, &font);
    let lower = layout_source("B", MathStyle::ScriptCramped, &font);

    let mut expected_upper = mul(&params.superscript_shift_up, &scale);
    let min_bottom = mul(&fu(constants.superscript_bottom_min().value), &scale);
    expected_upper = expected_upper.max_ref(&add(&upper.depth, &min_bottom));

    let mut expected_lower = mul(&params.subscript_shift_down, &scale);
    let max_top = mul(&fu(constants.subscript_top_max().value), &scale);
    expected_lower = expected_lower.max_ref(&sub(&lower.height, &max_top).clamp_nonneg());

    let gap = sub(
        &sub(&add(&expected_upper, &expected_lower), &upper.depth),
        &lower.height,
    );
    let min_gap = mul(&params.sub_superscript_gap_min, &scale);
    assert!(
        gap.cmp(&min_gap) != Ordering::Less,
        "Fira fixture must already satisfy the paired-script gap"
    );

    let current_bottom = sub(&expected_upper, &upper.depth);
    let paired_bottom = mul(
        &fu(constants.superscript_bottom_max_with_subscript().value),
        &scale,
    );
    assert!(
        current_bottom.cmp(&paired_bottom) == Ordering::Less,
        "Fira fixture must expose the obsolete unconditional paired-bottom adjustment"
    );
    let raise = sub(&paired_bottom, &current_bottom);
    assert!(
        expected_lower.cmp(&raise) != Ordering::Less,
        "old placement must be able to translate the pair instead of failing closed"
    );

    let scripted = layout_source("A_B^F", MathStyle::Text, &font);
    let (actual_upper, actual_lower) =
        paired_script_shifts(&scripted).expect("paired script overlap");

    assert!(
        actual_upper.eq_dim(&expected_upper),
        "superscript shifted without a gap repair: got {}, expected {}",
        actual_upper.to_dec_string(),
        expected_upper.to_dec_string()
    );

    let expected_lower_shift = -expected_lower;
    assert!(
        actual_lower.eq_dim(&expected_lower_shift),
        "subscript shifted without a gap repair: got {}, expected {}",
        actual_lower.to_dec_string(),
        expected_lower_shift.to_dec_string()
    );
}

#[test]
fn extended_shapes_and_box_bases_use_ink_box_baseline_drop_constraints() {
    let font = common::stix_two_math().expect("STIX Two Math fixture");
    let params = MathParams::from_font(&font).expect("OpenType MATH constants");
    let constants = font
        .face()
        .tables()
        .math
        .and_then(|math| math.constants)
        .expect("fixture font MATH constants");
    let units_per_em = font.units_per_em();
    let fu = |value: i16| Dim::from_font_units(i64::from(value), units_per_em).expect("font units");
    let scale = params.scale(MathStyle::Text);

    let script = layout_source("2", MathStyle::Script, &font);
    let standard_sup = mul(&params.superscript_shift_up, &scale);
    let min_bottom = mul(&fu(constants.superscript_bottom_min().value), &scale);
    let ordinary_sup = standard_sup.max_ref(&add(&script.depth, &min_bottom));
    let max_drop = mul(&fu(constants.superscript_baseline_drop_max().value), &scale);

    let ordinary_tall = layout_source("h", MathStyle::Text, &font);
    let wrong_if_all_tall_glyphs_were_boxes = sub(&ordinary_tall.height, &max_drop).clamp_nonneg();
    assert!(
        wrong_if_all_tall_glyphs_were_boxes > ordinary_sup,
        "fixture must distinguish an ordinary tall glyph from ExtendedShapeCoverage"
    );
    let ordinary_scripted = layout_source("h^2", MathStyle::Text, &font);
    assert_eq!(
        attached_script_shift(&ordinary_scripted, true).expect("ordinary superscript shift"),
        ordinary_sup,
        "ordinary glyphs keep the standard vertical script position"
    );

    let extended = layout_source("|", MathStyle::Text, &font);
    let extended_sup = ordinary_sup.max_ref(&sub(&extended.height, &max_drop).clamp_nonneg());
    let extended_scripted = layout_source("|^2", MathStyle::Text, &font);
    assert_eq!(
        attached_script_shift(&extended_scripted, true).expect("extended superscript shift"),
        extended_sup,
        "ExtendedShapeCoverage makes the glyph use its ink box for vertical positioning"
    );

    let subscript = layout_source("2", MathStyle::ScriptCramped, &font);
    let standard_sub = mul(&params.subscript_shift_down, &scale);
    let max_top = mul(&fu(constants.subscript_top_max().value), &scale);
    let ordinary_sub = standard_sub.max_ref(&sub(&subscript.height, &max_top).clamp_nonneg());
    let min_drop = mul(&fu(constants.subscript_baseline_drop_min().value), &scale);
    let extended_sub = ordinary_sub.max_ref(&add(&extended.depth, &min_drop));
    let extended_subscripted = layout_source("|_2", MathStyle::Text, &font);
    assert_eq!(
        attached_script_shift(&extended_subscripted, false).expect("extended subscript shift"),
        -extended_sub,
        "extended-shape subscript baseline must clear the ink-box bottom"
    );

    let boxed = layout_source(r"\frac{a}{b}", MathStyle::Text, &font);
    let boxed_sup = ordinary_sup.max_ref(&sub(&boxed.height, &max_drop).clamp_nonneg());
    let boxed_scripted = layout_source(r"\frac{a}{b}^2", MathStyle::Text, &font);
    assert_eq!(
        attached_script_shift(&boxed_scripted, true).expect("boxed superscript shift"),
        boxed_sup,
        "non-glyph bases use the same ink-box baseline-drop rule"
    );
}

#[test]
fn explicit_style_declaration_drives_following_script_semantics() {
    let font = common::stix_two_math().expect("STIX Two Math fixture");

    let declared = layout_source(r"\scriptstyle x_2^3", MathStyle::Text, &font);
    let direct = layout_source("x_2^3", MathStyle::Script, &font);

    assert!(declared.width.eq_dim(&direct.width));
    assert!(declared.height.eq_dim(&direct.height));
    assert!(declared.depth.eq_dim(&direct.depth));
}

#[test]
fn single_character_math_alphabet_remains_a_direct_script_nucleus() {
    let font = MathFont::from_bytes(FIRA).expect("Fira Math fixture");
    let direct = layout_source("𝐗_{ij}^{2}", MathStyle::TextCramped, &font);
    let styled = layout_source(r"\mathbf X_{ij}^{2}", MathStyle::TextCramped, &font);

    let direct_shifts = paired_script_shifts(&direct).expect("direct bold-X paired script shifts");
    let styled_shifts = paired_script_shifts(&styled).expect("styled bold-X paired script shifts");

    assert!(direct_shifts.0.eq_dim(styled_shifts.0));
    assert!(direct_shifts.1.eq_dim(styled_shifts.1));
}
