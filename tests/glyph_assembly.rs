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

fn set_horizontal_assembly_italic_correction(font: &[u8], glyph_id: u16, value: i16) -> Vec<u8> {
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
    fn coverage_index(table: &[u8], offset: usize, glyph_id: u16) -> Option<usize> {
        let coverage = &table[offset..];
        match read_u16(coverage, 0) {
            1 => {
                let count = usize::from(read_u16(coverage, 2));
                (0..count).find(|index| read_u16(coverage, 4 + index * 2) == glyph_id)
            }
            2 => {
                let count = usize::from(read_u16(coverage, 2));
                for index in 0..count {
                    let record = 4 + index * 6;
                    let start = read_u16(coverage, record);
                    let end = read_u16(coverage, record + 2);
                    if (start..=end).contains(&glyph_id) {
                        let base = usize::from(read_u16(coverage, record + 4));
                        return Some(base + usize::from(glyph_id - start));
                    }
                }
                None
            }
            format => panic!("unsupported coverage format {format}"),
        }
    }

    let mut out = font.to_vec();
    let table_count = usize::from(read_u16(&out, 4));
    let math_offset = (0..table_count)
        .find_map(|index| {
            let record = 12 + index * 16;
            (&out[record..record + 4] == b"MATH")
                .then(|| usize::try_from(read_u32(&out, record + 8)).unwrap())
        })
        .expect("fixture MATH table");
    let variants_offset = usize::from(read_u16(&out, math_offset + 8));
    let variants_start = math_offset + variants_offset;
    let variants = &out[variants_start..];
    let horizontal_coverage = usize::from(read_u16(variants, 4));
    let vertical_count = usize::from(read_u16(variants, 6));
    let horizontal_count = usize::from(read_u16(variants, 8));
    let coverage_index = coverage_index(variants, horizontal_coverage, glyph_id)
        .expect("glyph must have a horizontal construction");
    assert!(coverage_index < horizontal_count);
    let construction_offsets = 10 + vertical_count * 2;
    let construction_offset = usize::from(read_u16(
        variants,
        construction_offsets + coverage_index * 2,
    ));
    let construction = &variants[construction_offset..];
    let assembly_offset = usize::from(read_u16(construction, 0));
    assert_ne!(assembly_offset, 0, "fixture horizontal assembly");
    let value_offset = variants_start + construction_offset + assembly_offset;
    out[value_offset..value_offset + 2].copy_from_slice(&value.to_be_bytes());
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
fn horizontal_assembly_italic_correction_survives_the_accent_wrapper() {
    let base_font = common::stix_two_math().expect("STIX Two Math");
    let overbrace = base_font.glyph('⏞').expect("overbrace glyph");
    let bytes = set_horizontal_assembly_italic_correction(
        common::STIX_TWO_MATH_OTF,
        overbrace.glyph_id,
        321,
    );
    let face = ttf_parser::Face::parse(&bytes, 0).expect("mutated fixture face");
    let expected =
        texpose::Dim::from_font_units(321, face.units_per_em()).expect("italic em value");
    let font = MathFont::from_bytes(&bytes).expect("mutated MATH font");
    let output = layout_with_diagnostics(
        &parse(r"\overbrace{abcdefghijklmnopqrstuvwxyz}").expect("assembly source"),
        &font,
        MathStyle::Display,
    )
    .expect("horizontal assembly layout");

    assert!(output.diagnostics.is_empty());
    assert_eq!(output.math_box.italic, expected);
}

#[test]
fn stretchy_arrows_select_valid_combining_math_constructions_before_spacing_fallbacks() {
    let font = common::stix_two_math().expect("STIX Two Math");
    let nucleus_source = "ABCDEFGHIJK";
    let base = layout_with_diagnostics(
        &parse(nucleus_source).expect("plain arrow nucleus"),
        &font,
        MathStyle::Text,
    )
    .expect("plain arrow nucleus layout");
    assert!(base.diagnostics.is_empty());

    for (source, construction_ch) in [
        (r"\overrightarrow{ABCDEFGHIJK}", '\u{20D7}'),
        (r"\underleftarrow{ABCDEFGHIJK}", '\u{20EE}'),
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
        assert!(
            output.diagnostics.is_empty(),
            "a valid combining-arrow construction must not degrade: {source}: {:?}",
            output.diagnostics
        );
        assert!(
            glyph_count(&output.math_box, construction_ch) > 1,
            "wide arrow must materialize the selected combining MATH assembly: {source}"
        );
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
