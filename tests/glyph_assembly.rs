mod common;

use texpose::{
    layout_with_diagnostics, parse, BoxContent, LayoutDiagnostic, MathBox, MathFont, MathStyle,
};

fn glyph_count(tree: &MathBox, expected: char) -> usize {
    match &tree.content {
        BoxContent::Glyph { ch, .. } => usize::from(*ch == expected),
        BoxContent::HList(children)
        | BoxContent::VList(children)
        | BoxContent::Overlap(children) => children
            .iter()
            .map(|child| glyph_count(child, expected))
            .sum(),
        BoxContent::Color(_, inner)
        | BoxContent::BackColor(_, inner)
        | BoxContent::Frame { inner, .. } => glyph_count(inner, expected),
        _ => 0,
    }
}

fn glyph_id_for_char(tree: &MathBox, expected: char) -> Option<u16> {
    match &tree.content {
        BoxContent::Glyph { ch, glyph_id, .. } if *ch == expected => Some(*glyph_id),
        BoxContent::HList(children)
        | BoxContent::VList(children)
        | BoxContent::Overlap(children) => children
            .iter()
            .find_map(|child| glyph_id_for_char(child, expected)),
        BoxContent::Color(_, inner)
        | BoxContent::BackColor(_, inner)
        | BoxContent::Frame { inner, .. } => glyph_id_for_char(inner, expected),
        _ => None,
    }
}

fn set_math_min_connector_overlap(font: &[u8], value: u16) -> Vec<u8> {
    fn read_u16(bytes: &[u8], offset: usize) -> u16 {
        u16::from_be_bytes([bytes[offset], bytes[offset + 1]])
    }

    fn read_u32(bytes: &[u8], offset: usize) -> u32 {
        u32::from_be_bytes([
            bytes[offset],
            bytes[offset + 1],
            bytes[offset + 2],
            bytes[offset + 3],
        ])
    }

    let mut out = font.to_vec();
    let table_count = usize::from(read_u16(&out, 4));
    let mut math_offset = None;
    for index in 0..table_count {
        let record = 12 + index * 16;
        if &out[record..record + 4] == b"MATH" {
            math_offset = Some(usize::try_from(read_u32(&out, record + 8)).unwrap());
            break;
        }
    }
    let math_offset = math_offset.expect("fixture MATH table");
    let variants_offset = usize::from(read_u16(&out, math_offset + 8));
    assert_ne!(variants_offset, 0, "fixture MathVariants table");
    let field = math_offset + variants_offset;
    out[field..field + 2].copy_from_slice(&value.to_be_bytes());
    out
}

#[test]
fn vertical_delimiter_assembly_grows_beyond_prebuilt_variants() {
    let font = common::stix_two_math().expect("STIX Two Math");
    let ast = parse(r"\left|\frac{a+b}{c+d}\right|").expect("vertical assembly source");
    let output = layout_with_diagnostics(&ast, &font, MathStyle::Display)
        .expect("vertical delimiter assembly layout");

    assert!(
        output.diagnostics.is_empty(),
        "valid STIX assembly must not degrade: {:?}",
        output.diagnostics
    );
    assert!(
        glyph_count(&output.math_box, '|') > 2,
        "both delimiters must be constructed from multiple vertical MATH parts"
    );
}

#[test]
fn horizontal_overbrace_assembly_uses_repeated_parts_without_degradation() {
    let font = common::stix_two_math().expect("STIX Two Math");
    let ast = parse(r"\overbrace{abcdefghijklmnopqrstuvwxyz}").expect("horizontal assembly source");
    let output = layout_with_diagnostics(&ast, &font, MathStyle::Display)
        .expect("horizontal overbrace assembly layout");

    assert!(
        output.diagnostics.is_empty(),
        "valid STIX assembly must not degrade: {:?}",
        output.diagnostics
    );
    assert!(
        glyph_count(&output.math_box, '⏞') > 1,
        "wide overbrace must materialize multiple horizontal MATH parts"
    );
}

#[test]
fn stix_horizontal_arrows_respect_connector_overlap_and_degrade_invalid_recipes() {
    let font = common::stix_two_math().expect("STIX Two Math");
    let base = layout_with_diagnostics(
        &parse("AB").expect("plain arrow nucleus"),
        &font,
        MathStyle::Text,
    )
    .expect("plain arrow nucleus layout");
    assert!(base.diagnostics.is_empty());

    for (source, ch, degrades) in [
        (r"\overleftarrow{AB}", '←', false),
        (r"\underleftarrow{AB}", '←', false),
        (r"\overrightarrow{AB}", '→', true),
        (r"\underrightarrow{AB}", '→', true),
        (r"\overleftrightarrow{AB}", '↔', true),
        (r"\underleftrightarrow{AB}", '↔', true),
    ] {
        let output = layout_with_diagnostics(
            &parse(source).expect("stretchy arrow source"),
            &font,
            MathStyle::Text,
        )
        .expect("stretchy arrow layout");

        assert_eq!(
            output.math_box.width, base.math_box.width,
            "stretchy arrow must preserve the nucleus width: {source}"
        );
        if degrades {
            assert_eq!(
                output.diagnostics,
                vec![LayoutDiagnostic::ExtensibleFallback { ch }],
                "invalid STIX arrow assembly must degrade: {source}"
            );
            assert_eq!(
                glyph_count(&output.math_box, ch),
                1,
                "fallback must keep one prebuilt arrow glyph: {source}"
            );
        } else {
            assert!(
                output.diagnostics.is_empty(),
                "valid STIX arrow assembly must not degrade: {source}: {:?}",
                output.diagnostics
            );
            assert!(
                glyph_count(&output.math_box, ch) > 1,
                "valid STIX left-arrow assembly must materialize repeated parts: {source}"
            );
        }
    }
}

#[test]
fn invalid_connector_contract_falls_back_to_largest_prebuilt_and_diagnoses() {
    let bytes = set_math_min_connector_overlap(common::STIX_TWO_MATH_OTF, u16::MAX);
    let font =
        MathFont::from_bytes(&bytes).expect("MATH parser accepts structurally valid mutation");
    let ast = parse(r"\overbrace{abcdefghijklmnopqrstuvwxyz}").expect("fallback source");
    let output = layout_with_diagnostics(&ast, &font, MathStyle::Display)
        .expect("invalid assembly degrades instead of aborting");

    assert_eq!(
        output.diagnostics,
        vec![LayoutDiagnostic::ExtensibleFallback { ch: '⏞' }]
    );
    assert_eq!(
        glyph_count(&output.math_box, '⏞'),
        1,
        "unusable assembly must not materialize partial extender parts"
    );

    let base = font.glyph('⏞').expect("overbrace glyph");
    let expected_width = font
        .horizontal_variants(base.glyph_id)
        .into_iter()
        .filter_map(|glyph_id| {
            font.glyph_id('⏞', glyph_id)
                .ok()
                .map(|metrics| metrics.advance)
        })
        .max()
        .expect("at least base overbrace variant");
    let actual_id = glyph_id_for_char(&output.math_box, '⏞').expect("fallback overbrace glyph");
    let actual_width = font
        .glyph_id('⏞', actual_id)
        .expect("fallback glyph metrics")
        .advance;
    assert_eq!(actual_width, expected_width);
}
