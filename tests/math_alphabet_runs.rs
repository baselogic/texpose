use texpose::{parse, AtomKind, MathNode, TextStyle};

#[test]
fn adjacent_math_alphabet_characters_form_one_semantic_run() {
    let parsed = parse(r"\mathbf{abcdefghijklmnopqrstuvwxyz}").expect("parse styled run");
    assert_eq!(
        parsed,
        MathNode::MathAlphabet("abcdefghijklmnopqrstuvwxyz".into(), TextStyle::Bf)
    );

    let greek = parse(r"\mathit{ab\alpha cd}").expect("parse Greek run");
    assert_eq!(greek, MathNode::MathAlphabet("abαcd".into(), TextStyle::It),);
}

#[test]
fn nonstylable_boundaries_and_literal_text_preserve_the_ast() {
    let parsed = parse(r"\mathbf{ab+cd\text{hi}ef}").expect("parse mixed row");
    assert_eq!(
        parsed,
        MathNode::Row(vec![
            MathNode::MathAlphabet("ab".into(), TextStyle::Bf),
            MathNode::Atom('+', AtomKind::Bin),
            MathNode::MathAlphabet("cd".into(), TextStyle::Bf),
            MathNode::LiteralText("hi".into()),
            MathNode::MathAlphabet("ef".into(), TextStyle::Bf),
        ])
    );
}

#[test]
fn nested_styled_constructs_keep_their_structural_boundaries() {
    let parsed = parse(r"\mathbf{x_{ij}^{kl}}").expect("parse styled scripts");
    assert_eq!(
        parsed,
        MathNode::SubSup(
            Box::new(MathNode::MathAlphabet("x".into(), TextStyle::Bf)),
            Box::new(MathNode::MathAlphabet("ij".into(), TextStyle::Bf)),
            Box::new(MathNode::MathAlphabet("kl".into(), TextStyle::Bf)),
        )
    );

    let fraction = parse(r"\mathbf{\frac{ab}{cd}}").expect("parse styled fraction");
    let MathNode::Fraction(spec) = fraction else {
        panic!("fraction shape must be preserved");
    };
    assert_eq!(
        *spec.numerator,
        MathNode::MathAlphabet("ab".into(), TextStyle::Bf)
    );
    assert_eq!(
        *spec.denominator,
        MathNode::MathAlphabet("cd".into(), TextStyle::Bf)
    );

    let radical = parse(r"\mathbf{\sqrt{abc}}").expect("parse styled radical");
    assert_eq!(
        radical,
        MathNode::Radical(
            None,
            Box::new(MathNode::MathAlphabet("abc".into(), TextStyle::Bf)),
        )
    );
}
