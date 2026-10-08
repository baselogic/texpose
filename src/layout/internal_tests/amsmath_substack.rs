use super::common;

use core::cmp::Ordering;

use crate::test_support::{
    layout, parse, BoxContent, Dim, MathBox, MathFont, MathNode, MathParams, MathStyle,
};

const STIX: &[u8] =
    include_bytes!("../../../tests/fixtures/fonts/stix-two-math/STIXTwoMath-Regular.otf");
const LIBERTINUS: &[u8] =
    include_bytes!("../../../tests/fixtures/fonts/libertinus-math/LibertinusMath-Regular.otf");
const FIRA: &[u8] = include_bytes!("../../../tests/fixtures/fonts/fira-math/FiraMath-Regular.otf");

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

fn substack_rows(ast: &MathNode) -> &[MathNode] {
    let MathNode::Substack(lines) = ast else {
        panic!("expected substack AST");
    };

    lines
}

fn stack_constants(font: &MathFont, params: &MathParams) -> (Dim, Dim) {
    let math = font.face().tables().math.expect("MATH table");

    let constants = math.constants.expect("MATH constants");

    let units_per_em = font.units_per_em();

    let fu = |value: i16| Dim::from_font_units(i64::from(value), units_per_em).expect("font units");

    let scale = params.scale(MathStyle::Script);

    let baseline_skip = mul(
        &add(
            &fu(constants.stack_top_shift_up().value),
            &fu(constants.stack_bottom_shift_down().value),
        ),
        &scale,
    );

    let line_skip = mul(&fu(constants.stack_gap_min().value), &scale);

    (baseline_skip, line_skip)
}

fn expected_gaps(rows: &[MathBox], baseline_skip: &Dim, line_skip: &Dim) -> Vec<Dim> {
    rows.windows(2)
        .map(|pair| {
            let candidate = sub(&sub(baseline_skip, &pair[0].depth), &pair[1].height);

            if candidate.cmp(line_skip) != Ordering::Less {
                candidate
            } else {
                line_skip.clone()
            }
        })
        .collect()
}

#[test]
fn substack_uses_scriptstyle_rows_math_stack_spacing_and_vcenter() {
    let font = common::stix_two_math().expect("STIX Two Math");

    let params = MathParams::from_font(&font).expect("MATH constants");

    let ast = parse(r"\substack{1\le i\le n\\i\ne j}").expect("substack");

    let lines = substack_rows(&ast);

    let script_rows: Vec<_> = lines
        .iter()
        .map(|line| layout(line, &font, MathStyle::Script).expect("script row"))
        .collect();

    let scriptscript_rows: Vec<_> = lines
        .iter()
        .map(|line| layout(line, &font, MathStyle::ScriptScript).expect("scriptscript row"))
        .collect();

    let script_width = script_rows
        .iter()
        .fold(Dim::zero(), |width, row| width.max_ref(&row.width));

    let scriptscript_width = scriptscript_rows
        .iter()
        .fold(Dim::zero(), |width, row| width.max_ref(&row.width));

    assert_ne!(
        script_width, scriptscript_width,
        "fixture must distinguish Script from ScriptScript rows"
    );

    let tree = layout(&ast, &font, MathStyle::Script).expect("substack layout");

    assert!(tree.width.eq_dim(&script_width,));

    let (baseline_skip, line_skip) = stack_constants(&font, &params);

    let gaps = expected_gaps(&script_rows, &baseline_skip, &line_skip);

    let rows_span = script_rows.iter().fold(Dim::zero(), |span, row| {
        add(&add(&span, &row.height), &row.depth)
    });

    let expected_span = gaps.iter().fold(rows_span, |span, gap| add(&span, gap));

    let actual_span = add(&tree.height, &tree.depth);

    assert!(actual_span.eq_dim(&expected_span,));

    let axis = mul(&params.axis_height, &params.scale(MathStyle::Script));

    let center = div(&sub(&tree.height, &tree.depth), &Dim::from_i64(2));

    assert!(
        center.eq_dim(&axis),
        "substack center {} != Script axis {}",
        center.to_dec_string(),
        axis.to_dec_string()
    );
}

#[test]
fn substack_single_atom_rows_clean_terminal_math_italic_across_profiles() {
    let ast = parse(r"\substack{x\\x}").expect("substack");
    let x_ast = parse("x").expect("x");

    for (name, font) in profile_fonts() {
        let direct = layout(&x_ast, &font, MathStyle::Script)
            .unwrap_or_else(|error| panic!("{name} script x: {error}"));
        assert!(
            !direct.italic.is_zero(),
            "{name}: fixture must expose script terminal math italic"
        );
        let expected = add(&direct.width, &direct.italic);
        let stack = layout(&ast, &font, MathStyle::Display)
            .unwrap_or_else(|error| panic!("{name} substack: {error}"));
        assert!(
            stack.width.eq_dim(&expected),
            "{name}: substack width {} != clean script width {}",
            stack.width.to_dec_string(),
            expected.to_dec_string()
        );
    }
}

#[test]
fn display_sum_preserves_substack_vcenter_inside_lower_limit() {
    let font = common::stix_two_math().expect("STIX Two Math");

    let params = MathParams::from_font(&font).expect("MATH constants");

    let lower_ast =
        parse(r"\substack{1\le i\le n\\1\le j\le m\\i\ne j}").expect("substack lower limit");

    let lower = layout(&lower_ast, &font, MathStyle::Script).expect("standalone substack");

    let sum_ast =
        parse(r"\sum_{\substack{1\le i\le n\\1\le j\le m\\i\ne j}}").expect("display sum");

    let sum = layout(&sum_ast, &font, MathStyle::Display).expect("display sum layout");

    assert!(sum.width.eq_dim(&lower.width,));

    let BoxContent::Overlap(branches) = &sum.content else {
        panic!("display sum with a lower limit must be an overlap");
    };

    let [_, lower_branch] = branches.as_slice() else {
        panic!("expected operator and lower-limit branches");
    };

    assert!(lower_branch.shift.cmp(&Dim::zero()) == Ordering::Less);

    let axis = mul(&params.axis_height, &params.scale(MathStyle::Script));

    let intrinsic_center = div(
        &sub(&lower_branch.height, &lower_branch.depth),
        &Dim::from_i64(2),
    );

    assert!(
        intrinsic_center.eq_dim(&axis),
        "external limit shift must not overwrite substack vcenter"
    );
}

#[test]
fn substack_script_rows_and_vcenter_hold_across_verification_fonts() {
    let ast = parse(r"\substack{1\le i\le n\\i\ne j}").expect("substack");

    for (name, font) in profile_fonts() {
        let params = MathParams::from_font(&font).expect("MATH constants");
        let tree = layout(&ast, &font, MathStyle::Script)
            .unwrap_or_else(|error| panic!("{name} substack layout: {error}"));
        let axis = mul(&params.axis_height, &params.scale(MathStyle::Script));
        let center = div(&sub(&tree.height, &tree.depth), &Dim::from_i64(2));
        assert!(
            center.eq_dim(&axis),
            "{name} substack center {} != script axis {}",
            center.to_dec_string(),
            axis.to_dec_string()
        );

        let MathNode::Substack(lines) = &ast else {
            unreachable!();
        };
        let expected_width = lines
            .iter()
            .map(|line| {
                layout(line, &font, MathStyle::Script)
                    .expect("script row")
                    .width
            })
            .fold(Dim::zero(), |width, row| width.max_ref(&row));
        assert!(tree.width.eq_dim(&expected_width), "{name} substack width");
    }
}
