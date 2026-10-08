use std::sync::Arc;

use texpose::{
    layout, parse, AtomKind, Error, LayoutDiagnostic, MathFont, MathNode, MathOp, MathStyle,
    NumericError,
};

const STIX: &[u8] = include_bytes!("../data/fonts/stix-two-math/STIXTwoMath-Regular.otf");

fn font() -> MathFont {
    MathFont::from_shared_bytes(Arc::from(STIX), 0).expect("STIX Two Math")
}

#[test]
fn pmb_keeps_its_syntactic_argument_and_atom_class() {
    for (input, expected) in [
        (r"\pmb{x}", AtomKind::Ord),
        (r"\pmb{+}", AtomKind::Bin),
        (r"\pmb{=}", AtomKind::Rel),
    ] {
        let MathNode::Pmb(inner) = parse(input).expect("parse pmb") else {
            panic!("{input} must parse into Pmb, not MathAlphabet")
        };
        let MathNode::Atom(_, class) = *inner else {
            panic!("{input} must retain an atom nucleus")
        };
        assert_eq!(class, expected, "{input}");
    }
}

#[test]
fn pmb_triples_paint_without_changing_logical_geometry() {
    let font = font();
    for input in ["x", "+", "=", r"\infty", r"\frac{a}{b}"] {
        let plain = layout(&parse(input).unwrap(), &font, MathStyle::Text).unwrap();
        let source = format!(r"\pmb{{{input}}}");
        let bold = layout(&parse(&source).unwrap(), &font, MathStyle::Text).unwrap();
        assert_eq!(bold.width().to_bits(), plain.width().to_bits(), "{source}");
        assert_eq!(
            bold.height().to_bits(),
            plain.height().to_bits(),
            "{source}"
        );
        assert_eq!(bold.depth().to_bits(), plain.depth().to_bits(), "{source}");
        assert_eq!(bold.diagnostics(), plain.diagnostics(), "{source}");
        assert_eq!(bold.ops().len(), 3 * plain.ops().len(), "{source}");
        assert_eq!(
            bold.ops()[0],
            plain.ops()[0],
            "first impression must use the original glyph"
        );
    }
}

#[test]
fn pmb_preserves_relation_and_binary_spacing() {
    let font = font();
    for (plain, decorated) in [
        ("a=b", r"a\pmb{=}b"),
        ("a+b", r"a\pmb{+}b"),
        ("a=b", r"a\pmb{{=}}b"),
    ] {
        let base = layout(&parse(plain).unwrap(), &font, MathStyle::Text).unwrap();
        let bold = layout(&parse(decorated).unwrap(), &font, MathStyle::Text).unwrap();
        assert_eq!(
            bold.width().to_bits(),
            base.width().to_bits(),
            "{decorated}"
        );
        assert_eq!(bold.ops().len(), base.ops().len() + 2);
        let MathOp::Glyph {
            x: x0, glyph_id, ..
        } = bold.ops()[1]
        else {
            panic!("missing decorated nucleus")
        };
        let MathOp::Glyph {
            x: x1,
            glyph_id: g1,
            ..
        } = bold.ops()[2]
        else {
            panic!("missing second imprint")
        };
        let MathOp::Glyph {
            x: x2,
            glyph_id: g2,
            ..
        } = bold.ops()[3]
        else {
            panic!("missing third imprint")
        };
        assert_eq!(glyph_id, g1);
        assert_eq!(glyph_id, g2);
        assert!(x0 < x1 && x1 < x2, "imprints must differ in x");
        assert_eq!(
            bold.ops()[4],
            base.ops()[2],
            "subsequent nucleus must not move"
        );
    }
}

#[test]
fn pmb_nested_painting_is_bounded_and_diagnostics_are_not_repeated() {
    let font = font();
    let once = layout(&parse(r"\pmb{x}").unwrap(), &font, MathStyle::Text).unwrap();
    let twice = layout(&parse(r"\pmb{\pmb{x}}").unwrap(), &font, MathStyle::Text).unwrap();
    assert_eq!(once.width().to_bits(), twice.width().to_bits());
    assert_eq!(twice.ops().len(), 9);

    let missing = '\u{10FFFF}';
    let input = format!(r"\pmb{{{missing}}}");
    let output = layout(&parse(&input).unwrap(), &font, MathStyle::Text).unwrap();
    assert_eq!(output.diagnostics().len(), 1);
    assert!(matches!(
        output.diagnostics()[0],
        LayoutDiagnostic::MissingGlyph { .. }
    ));

    let mut nested = "x".to_owned();
    for _ in 0..12 {
        nested = format!(r"\pmb{{{nested}}}");
    }
    let err = match layout(&parse(&nested).unwrap(), &font, MathStyle::Text) {
        Err(err) => err,
        Ok(_) => panic!("nested paint expansion must be bounded"),
    };
    assert!(matches!(err, Error::Numeric(NumericError::OutOfRange)));
}
