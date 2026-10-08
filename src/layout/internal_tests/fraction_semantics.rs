use super::common;

use crate::test_support::{
    layout, layout_with_diagnostics, layout_with_em_size_pt, parse, BoxContent, Dim, Error,
    FractionAlignment, FractionRule, FractionSpec, FractionStyle, LayoutDiagnostic, Length,
    MathBox, MathFont, MathNode, MathParams, MathStyle,
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

fn fraction(source: &str) -> texpose::FractionSpec {
    match parse(source).expect("fraction syntax must parse") {
        MathNode::Fraction(spec) => spec,
        other => panic!("expected fraction AST, got {}", other.gold()),
    }
}

fn fraction_layers(bx: &MathBox) -> &[MathBox] {
    let BoxContent::HList(parts) = &bx.content else {
        panic!("fraction must be an hlist");
    };
    let Some(inner) = parts.get(1) else {
        panic!("fraction hlist must contain its inner stack");
    };
    let BoxContent::Overlap(layers) = &inner.content else {
        panic!("fraction inner box must be an overlap");
    };
    layers
}

fn font_units(font: &MathFont, value: i16) -> Dim {
    Dim::from_font_units(i64::from(value), font.units_per_em()).expect("font units")
}

fn first_glyph_x(math_box: &MathBox) -> Option<Dim> {
    fn visit(math_box: &MathBox, x: &Dim) -> Option<Dim> {
        match &math_box.content {
            BoxContent::Glyph { .. } => Some(x.clone()),
            BoxContent::HList(children) => {
                let mut child_x = x.clone();
                for child in children {
                    if let Some(found) = visit(child, &child_x) {
                        return Some(found);
                    }
                    child_x = child_x
                        .checked_add(&child.width)
                        .expect("test x arithmetic");
                }
                None
            }
            BoxContent::VList(children) | BoxContent::Overlap(children) => {
                children.iter().find_map(|child| visit(child, x))
            }
            BoxContent::Color(_, inner) | BoxContent::BackColor(_, inner) => visit(inner, x),
            BoxContent::Frame { inner, .. } => visit(inner, x),
            BoxContent::Empty
            | BoxContent::Rule
            | BoxContent::Kern(_)
            | BoxContent::Line { .. } => None,
        }
    }

    visit(math_box, &Dim::zero())
}

fn glyph_scales(math_box: &MathBox, scales: &mut Vec<Dim>) {
    match &math_box.content {
        BoxContent::Glyph { scale, .. } => scales.push(scale.clone()),
        BoxContent::HList(children)
        | BoxContent::VList(children)
        | BoxContent::Overlap(children) => {
            for child in children {
                glyph_scales(child, scales);
            }
        }
        BoxContent::Color(_, inner) | BoxContent::BackColor(_, inner) => {
            glyph_scales(inner, scales);
        }
        BoxContent::Frame { inner, .. } => glyph_scales(inner, scales),
        BoxContent::Empty | BoxContent::Rule | BoxContent::Kern(_) | BoxContent::Line { .. } => {}
    }
}

fn count_rules(content: &BoxContent) -> usize {
    match content {
        BoxContent::Rule => 1,
        BoxContent::HList(children)
        | BoxContent::VList(children)
        | BoxContent::Overlap(children) => children
            .iter()
            .map(|child| count_rules(&child.content))
            .sum(),
        BoxContent::Color(_, child) | BoxContent::BackColor(_, child) => {
            count_rules(&child.content)
        }
        BoxContent::Frame { inner, .. } => count_rules(&inner.content),
        _ => 0,
    }
}

#[test]
fn fraction_commands_preserve_distinct_semantics() {
    let ordinary = fraction(r"\frac{a}{b}");
    assert_eq!(ordinary.style, FractionStyle::Inherit);
    assert_eq!(ordinary.rule, FractionRule::Default);
    assert_eq!(ordinary.numerator_alignment, FractionAlignment::Default);

    assert_eq!(fraction(r"\dfrac{a}{b}").style, FractionStyle::Display);
    assert_eq!(fraction(r"\tfrac{a}{b}").style, FractionStyle::Text);

    let centered = fraction(r"\cfrac{a}{b}");
    assert_eq!(centered.style, FractionStyle::Display);
    assert_eq!(centered.numerator_alignment, FractionAlignment::Center);
    assert_eq!(
        fraction(r"\cfrac[l]{a}{b}").numerator_alignment,
        FractionAlignment::Left
    );
    assert_eq!(
        fraction(r"\cfrac[r]{a}{b}").numerator_alignment,
        FractionAlignment::Right
    );

    let genfrac = fraction(r"\genfrac{(}{)}{0pt}{1}{a}{b}");
    assert_eq!(genfrac.style, FractionStyle::Text);
    assert_eq!(genfrac.rule, FractionRule::None);
    assert!(matches!(
        genfrac.left_delimiter,
        texpose::Delimiter::Char('(')
    ));
    assert!(matches!(
        genfrac.right_delimiter,
        texpose::Delimiter::Char(')')
    ));

    let exact = fraction(r"\genfrac{}{}{2pt}{3}{a}{b}");
    assert_eq!(exact.style, FractionStyle::ScriptScript);
    assert_eq!(
        exact.rule,
        FractionRule::Exact(Length::TexPt(Dim::from_i64(2)))
    );
    assert!(parse(r"\genfrac{}{}{-1pt}{}{a}{b}").is_err());
}

#[test]
fn binom_choose_over_and_nested_forms_are_structural() {
    let binom = fraction(r"\binom{n}{k}");
    assert_eq!(binom.rule, FractionRule::None);
    assert_eq!(fraction(r"\dbinom{n}{k}").style, FractionStyle::Display);
    assert_eq!(fraction(r"\tbinom{n}{k}").style, FractionStyle::Text);
    assert!(matches!(
        binom.left_delimiter,
        texpose::Delimiter::Char('(')
    ));
    assert!(matches!(
        binom.right_delimiter,
        texpose::Delimiter::Char(')')
    ));

    let choose = fraction(r"{n \choose k}");
    assert_eq!(choose.rule, FractionRule::None);

    let nested_over = parse(r"{{a \over b} \over {c \over d}}").unwrap();
    let MathNode::Fraction(outer) = nested_over else {
        panic!("outer generalized fraction lost structurally");
    };
    assert!(matches!(*outer.numerator, MathNode::Fraction(_)));
    assert!(matches!(*outer.denominator, MathNode::Fraction(_)));

    let nested_choose = parse(r"{{n \choose k} \choose {r \choose s}}").unwrap();
    let MathNode::Fraction(outer) = nested_choose else {
        panic!("outer choose lost structurally");
    };
    assert!(matches!(*outer.numerator, MathNode::Fraction(_)));
    assert!(matches!(*outer.denominator, MathNode::Fraction(_)));

    assert!(parse(r"{a \over b \over c}").is_err());
    assert!(parse(r"{a \over \color{red} b \over c}").is_err());
    assert!(parse(r"a \over b").is_err());
}

#[test]
fn plain_tex_font_switches_and_mbox_are_parsed_without_preprocessing() {
    assert_eq!(parse(r"{\rm ab}").unwrap().gold(), r#"(mathalpha rm "ab")"#);
    assert_eq!(
        parse(r"{x\bf yz}").unwrap().gold(),
        r#"(row (atom Ord "x") (mathalpha bf "yz"))"#
    );
    assert_eq!(
        parse(r"\mbox{Diagonal}").unwrap().gold(),
        r#"(literal "Diagonal")"#
    );
}

#[test]
fn style_overrides_are_observable_in_layout() {
    let font = common::stix_two_math().unwrap();

    let dfrac = layout(&parse(r"\dfrac{1}{2}").unwrap(), &font, MathStyle::Script).unwrap();
    let frac_in_script = layout(&parse(r"\frac{1}{2}").unwrap(), &font, MathStyle::Script).unwrap();
    assert!(dfrac.height > frac_in_script.height || dfrac.depth > frac_in_script.depth);

    let tfrac = layout(&parse(r"\tfrac{1}{2}").unwrap(), &font, MathStyle::Display).unwrap();
    let frac_in_display =
        layout(&parse(r"\frac{1}{2}").unwrap(), &font, MathStyle::Display).unwrap();
    assert!(tfrac.height < frac_in_display.height || tfrac.depth < frac_in_display.depth);
}

#[test]
fn binomial_fraction_suppresses_the_fraction_rule() {
    let font = common::stix_two_math().unwrap();
    let frac = layout(&parse(r"\frac{n}{k}").unwrap(), &font, MathStyle::Text).unwrap();
    let binom = layout(&parse(r"\binom{n}{k}").unwrap(), &font, MathStyle::Text).unwrap();
    assert!(count_rules(&frac.content) >= 1);
    assert_eq!(count_rules(&binom.content), 0);
}

#[test]
fn ruled_fraction_uses_math_fraction_constants() {
    let font = common::stix_two_math().unwrap();
    let constants = font
        .face()
        .tables()
        .math
        .expect("MATH table")
        .constants
        .expect("MATH constants");

    for (style, num_shift, den_shift, num_gap, den_gap) in [
        (
            MathStyle::Text,
            constants.fraction_numerator_shift_up().value,
            constants.fraction_denominator_shift_down().value,
            constants.fraction_numerator_gap_min().value,
            constants.fraction_denominator_gap_min().value,
        ),
        (
            MathStyle::Display,
            constants.fraction_numerator_display_style_shift_up().value,
            constants
                .fraction_denominator_display_style_shift_down()
                .value,
            constants.fraction_num_display_style_gap_min().value,
            constants.fraction_denom_display_style_gap_min().value,
        ),
    ] {
        let boxed = layout(&parse(r"\frac{b}{b}").unwrap(), &font, style).unwrap();
        let layers = fraction_layers(&boxed);
        assert_eq!(
            layers.len(),
            3,
            "ruled fraction must contain numerator/bar/denominator"
        );

        let numerator = &layers[0];
        let bar = &layers[1];
        let denominator = &layers[2];
        let half = bar.height.checked_div(&Dim::from_i64(2)).unwrap();
        let axis = font_units(&font, constants.axis_height().value);
        let denominator_shift_down = -denominator.shift.clone();
        let actual_num_gap = numerator
            .shift
            .checked_sub(&numerator.depth)
            .unwrap()
            .checked_sub(&bar.shift.checked_add(&bar.height).unwrap())
            .unwrap();
        let actual_den_gap = bar
            .shift
            .checked_sub(&denominator.shift.checked_add(&denominator.height).unwrap())
            .unwrap();

        assert_eq!(
            bar.height,
            font_units(&font, constants.fraction_rule_thickness().value)
        );
        assert_eq!(bar.shift.checked_add(&half).unwrap(), axis);
        assert!(numerator.shift >= font_units(&font, num_shift));
        assert!(denominator_shift_down >= font_units(&font, den_shift));
        assert!(actual_num_gap >= font_units(&font, num_gap));
        assert!(actual_den_gap >= font_units(&font, den_gap));
    }
}

#[test]
fn ruleless_fraction_uses_math_stack_spacing() {
    let font = common::stix_two_math().unwrap();
    let constants = font
        .face()
        .tables()
        .math
        .expect("MATH table")
        .constants
        .expect("MATH constants");

    for (source, style, top, bottom, gap) in [
        (
            r"\binom{a}{b}",
            MathStyle::Text,
            constants.stack_top_shift_up().value,
            constants.stack_bottom_shift_down().value,
            constants.stack_gap_min().value,
        ),
        (
            r"\dbinom{a}{b}",
            MathStyle::Display,
            constants.stack_top_display_style_shift_up().value,
            constants.stack_bottom_display_style_shift_down().value,
            constants.stack_display_style_gap_min().value,
        ),
    ] {
        let boxed = layout(&parse(source).unwrap(), &font, style).unwrap();
        let layers = fraction_layers(&boxed);
        assert_eq!(layers.len(), 2, "ruleless fraction must not contain a bar");

        let numerator = &layers[0];
        let denominator = &layers[1];
        let denominator_shift_down = -denominator.shift.clone();
        let actual_gap = numerator
            .shift
            .checked_sub(&numerator.depth)
            .unwrap()
            .checked_add(&denominator_shift_down)
            .unwrap()
            .checked_sub(&denominator.height)
            .unwrap();

        assert!(numerator.shift >= font_units(&font, top));
        assert!(denominator_shift_down >= font_units(&font, bottom));
        assert!(actual_gap >= font_units(&font, gap));
    }
}

#[test]
fn public_fraction_spec_rejects_negative_explicit_rule_thickness() {
    let font = common::stix_two_math().unwrap();
    let mut spec = FractionSpec::ordinary(
        MathNode::Atom('a', texpose::AtomKind::Ord),
        MathNode::Atom('b', texpose::AtomKind::Ord),
    );
    spec.rule = FractionRule::Exact(Length::TexPt(Dim::from_i64(-1)));

    let err = layout(&MathNode::Fraction(spec), &font, MathStyle::Text).unwrap_err();
    assert!(matches!(err, Error::Malformed { .. }));
}

#[test]
fn infix_fraction_stays_at_group_level_across_declarations() {
    let colored = parse(r"{a \color{red} b \over c}").unwrap();
    let MathNode::Fraction(colored) = colored else {
        panic!("top-level over must remain the group-level fraction");
    };
    assert_eq!(
        colored.numerator.gold(),
        r#"(row (atom Ord "a") (color #ff0000 (atom Ord "b")))"#
    );
    assert_eq!(
        colored.denominator.gold(),
        r#"(color #ff0000 (atom Ord "c"))"#
    );

    let styled = parse(r"{a \bf b \over c}").unwrap();
    let MathNode::Fraction(styled) = styled else {
        panic!("plain-TeX font declaration must not hide group-level over");
    };
    assert_eq!(
        styled.numerator.gold(),
        r#"(row (atom Ord "a") (mathalpha bf "b"))"#
    );
    assert_eq!(styled.denominator.gold(), r#"(mathalpha bf "c")"#);
}

#[test]
fn explicit_rule_thickness_controls_layout_and_zero_is_ruleless() {
    let font = common::stix_two_math().unwrap();

    let exact = layout(
        &parse(r"\genfrac{}{}{2pt}{}{a}{b}").unwrap(),
        &font,
        MathStyle::Text,
    )
    .unwrap();
    let exact_rule = fraction_layers(&exact)
        .iter()
        .find(|layer| matches!(&layer.content, BoxContent::Rule))
        .expect("positive explicit thickness must produce a rule");
    assert_eq!(exact_rule.height, Dim::ratio(1, 5).unwrap());

    let numerator = MathNode::Atom('a', texpose::AtomKind::Ord);
    let denominator = MathNode::Atom('b', texpose::AtomKind::Ord);
    let mut zero = FractionSpec::ordinary(numerator.clone(), denominator.clone());
    zero.rule = FractionRule::Exact(Length::TexPt(Dim::zero()));
    let mut none = FractionSpec::ordinary(numerator, denominator);
    none.rule = FractionRule::None;

    let zero_box = layout(&MathNode::Fraction(zero), &font, MathStyle::Text).unwrap();
    let none_box = layout(&MathNode::Fraction(none), &font, MathStyle::Text).unwrap();
    assert_eq!(zero_box, none_box);
}

#[test]
fn nested_fraction_keeps_math_script_scale_across_root_em_sizes_and_profiles() {
    let ast = parse(r"\frac{1+\frac{a}{b}}{1+\frac{c}{d}}").expect("nested fraction");

    for (name, font) in profile_fonts() {
        let expected_script = MathParams::from_font(&font)
            .expect("validated MATH constants")
            .scale(MathStyle::Script);
        for size in [6_i64, 10, 20, 40] {
            let boxed =
                layout_with_em_size_pt(&ast, &font, MathStyle::Display, &Dim::from_i64(size))
                    .unwrap_or_else(|error| panic!("{name} {size}pt nested fraction: {error}"));
            let mut scales = Vec::new();
            glyph_scales(&boxed, &mut scales);

            assert!(
                scales.iter().any(|scale| scale == &Dim::one()),
                "{name} {size}pt must retain text-style glyphs"
            );
            assert!(
                scales.iter().any(|scale| scale == &expected_script),
                "{name} {size}pt must use the font MATH script scale"
            );
            assert!(
                scales
                    .iter()
                    .all(|scale| scale == &Dim::one() || scale == &expected_script),
                "{name} {size}pt introduced a physical-size-dependent glyph scale"
            );
        }
    }
}

#[test]
fn fraction_degrades_missing_component_glyph_without_losing_fraction_structure() {
    let font = common::stix_two_math().expect("STIX Two Math");
    let node = MathNode::Fraction(FractionSpec::ordinary(
        MathNode::Atom(MISSING, texpose::AtomKind::Ord),
        MathNode::Atom('1', texpose::AtomKind::Ord),
    ));
    let output = layout_with_diagnostics(&node, &font, MathStyle::Display)
        .expect("missing component glyph must use the normal deterministic fallback");

    assert_eq!(
        output.diagnostics,
        vec![LayoutDiagnostic::MissingGlyph { ch: MISSING }]
    );
    assert_eq!(fraction_layers(&output.math_box).len(), 3);
    assert_eq!(count_rules(&output.math_box.content), 1);
}

#[test]
fn fraction_components_materialize_terminal_math_italic_across_profiles() {
    for (name, font) in profile_fonts() {
        let numerator = layout(&parse("b").unwrap(), &font, MathStyle::Text).unwrap();
        let denominator = layout(&parse("b").unwrap(), &font, MathStyle::TextCramped).unwrap();
        let expected = numerator
            .width
            .checked_add(&numerator.italic)
            .unwrap()
            .max_ref(&denominator.width.checked_add(&denominator.italic).unwrap());

        let fraction = layout(&parse(r"\frac{b}{b}").unwrap(), &font, MathStyle::Display).unwrap();
        let rule = fraction_layers(&fraction)
            .iter()
            .find(|layer| matches!(layer.content, BoxContent::Rule))
            .unwrap_or_else(|| panic!("{name}: ruled fraction must contain a bar"));

        assert_eq!(
            rule.width, expected,
            "{name}: fraction clean-box width must include terminal math italic"
        );
    }
}

#[test]
fn cfrac_alignment_changes_only_the_numerator_horizontal_origin() {
    let font = common::stix_two_math().unwrap();
    let centered = layout(
        &parse(r"\cfrac{i}{MMMM}").unwrap(),
        &font,
        MathStyle::Display,
    )
    .unwrap();
    let left = layout(
        &parse(r"\cfrac[l]{i}{MMMM}").unwrap(),
        &font,
        MathStyle::Display,
    )
    .unwrap();
    let right = layout(
        &parse(r"\cfrac[r]{i}{MMMM}").unwrap(),
        &font,
        MathStyle::Display,
    )
    .unwrap();

    let center_layers = fraction_layers(&centered);
    let left_layers = fraction_layers(&left);
    let right_layers = fraction_layers(&right);
    let center_x = first_glyph_x(&center_layers[0]).expect("centered numerator glyph");
    let left_x = first_glyph_x(&left_layers[0]).expect("left numerator glyph");
    let right_x = first_glyph_x(&right_layers[0]).expect("right numerator glyph");

    assert_eq!(left_x, Dim::zero());
    assert!(left_x < center_x && center_x < right_x);
    assert_eq!(centered.width, left.width);
    assert_eq!(centered.width, right.width);
    assert_eq!(center_layers[1].width, left_layers[1].width);
    assert_eq!(center_layers[1].width, right_layers[1].width);
}
