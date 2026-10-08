use texpose::{parse, styled_char, MathNode, ParseErrorKind, TextStyle};

#[test]
fn boldsymbol_stylable_atoms_and_scripts_remain_math_alphabets() {
    for source in [r"\boldsymbol{x}", r"\boldsymbol{\alpha}"] {
        let node = parse(source).expect("bold mathematical character parses");
        match node {
            MathNode::MathAlphabet(chars, TextStyle::Boldsymbol) => {
                assert_eq!(chars.chars().count(), 1);
                let ch = chars.chars().next().unwrap();
                assert_ne!(styled_char(ch, TextStyle::Boldsymbol), ch);
            }
            other => panic!("expected a bold math alphabet for {source}, got {other:?}"),
        }
    }

    let scripted = parse(r"\boldsymbol{x^2}").unwrap();
    let MathNode::Superscript(base, sup) = scripted else {
        panic!("boldsymbol must retain the script structure");
    };
    assert!(matches!(
        *base,
        MathNode::MathAlphabet(_, TextStyle::Boldsymbol)
    ));
    assert!(matches!(
        *sup,
        MathNode::MathAlphabet(_, TextStyle::Boldsymbol)
    ));
}

#[test]
fn boldsymbol_rejects_math_shapes_without_an_actual_bold_variant() {
    for source in [
        r"\boldsymbol{+}",
        r"\boldsymbol{=}",
        r"\boldsymbol{\infty}",
        r"\boldsymbol{\sin}",
        r"\boldsymbol{(x)}",
        r"\boldsymbol{x+1}",
        r"\boldsymbol{\frac{x}{y}}",
        r"\boldsymbol{\text{x}}",
        r"\boldsymbol{\pmb{x}}",
    ] {
        let error = parse(source).expect_err("unsupported bold math must be rejected");
        assert_eq!(error.kind(), ParseErrorKind::UnsupportedCommand, "{source}");
    }
}

#[test]
fn strict_boldsymbol_does_not_change_other_math_alphabet_or_pmb_contracts() {
    assert!(parse(r"\mathbf{+}").is_ok());
    assert!(matches!(parse(r"\pmb{+}").unwrap(), MathNode::Pmb(_)));
}
