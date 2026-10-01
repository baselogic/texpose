mod common;

use core::cmp::Ordering;

use texpose::{layout, parse, BoxContent, Dim, MathBox, MathFont, MathParams, MathStyle};

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

    if gap.cmp(&min_gap) == Ordering::Less {
        expected_lower = add(&expected_lower, &sub(&min_gap, &gap));
    }

    let legacy_upper = expected_upper.clone();

    let legacy_lower = expected_lower.clone();

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
fn explicit_style_declaration_drives_following_script_semantics() {
    let font = common::stix_two_math().expect("STIX Two Math fixture");

    let declared = layout_source(r"\scriptstyle x_2^3", MathStyle::Text, &font);
    let direct = layout_source("x_2^3", MathStyle::Script, &font);

    assert!(declared.width.eq_dim(&direct.width));
    assert!(declared.height.eq_dim(&direct.height));
    assert!(declared.depth.eq_dim(&direct.depth));
}
