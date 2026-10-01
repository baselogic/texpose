use texpose::{layout, parse, BoxContent, MathFont, MathNode, MathStyle, ParseErrorKind};

fn literal_glyphs(node: &texpose::MathBox) -> Vec<char> {
    match &node.content {
        BoxContent::Glyph { ch, .. } => vec![*ch],
        BoxContent::HList(children)
        | BoxContent::VList(children)
        | BoxContent::Overlap(children) => children.iter().flat_map(literal_glyphs).collect(),
        BoxContent::Color(_, inner)
        | BoxContent::BackColor(_, inner)
        | BoxContent::Frame { inner, .. } => literal_glyphs(inner),
        _ => Vec::new(),
    }
}

#[test]
fn math_alphabet_and_literal_text_are_distinct_semantic_constructs() {
    assert_eq!(
        parse(r"\mathrm{ab}").unwrap().gold(),
        r#"(mathalpha rm "ab")"#
    );
    assert_eq!(parse(r"\text{ab}").unwrap().gold(), r#"(literal "ab")"#);
    assert_eq!(
        parse(r"\mathrm{\text{ab}c}").unwrap().gold(),
        r#"(row (literal "ab") (mathalpha rm "c"))"#
    );

    let ast = parse(r"\text{ab}").unwrap();
    assert!(matches!(ast, MathNode::LiteralText(ref text) if text == "ab"));
}

#[test]
fn literal_text_uses_direct_scalar_glyphs_and_face_space_advance() {
    let font = MathFont::stix_two_math().expect("STIX Two Math");
    let ast = parse(r"\text{A A}").expect("literal text");
    let laid = layout(&ast, &font, MathStyle::Text).expect("literal text layout");

    let a = font.glyph('A').expect("A glyph");
    let space = font.glyph(' ').expect("U+0020 glyph");
    let expected = a
        .advance
        .checked_add(&space.advance)
        .and_then(|width| width.checked_add(&a.advance))
        .expect("literal text width");

    assert_eq!(laid.width, expected);
    assert_eq!(literal_glyphs(&laid), vec!['A', ' ', 'A']);
}

#[test]
fn unsupported_text_mode_semantics_fail_typed_instead_of_becoming_literal_bytes() {
    assert_eq!(
        parse(r"\text{\% \_ \{x\}}").unwrap().gold(),
        r#"(literal "% _ {x}")"#
    );

    for source in [
        r"\textbf{abc}",
        r"\text{a\alpha b}",
        r"\text{a{b}}",
        r"\text{a\\b}",
    ] {
        let error = parse(source).expect_err(source);
        assert_eq!(error.kind(), ParseErrorKind::UnsupportedCommand, "{source}");
    }
}
