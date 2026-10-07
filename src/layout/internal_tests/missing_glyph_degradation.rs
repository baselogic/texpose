use super::common;

use crate::test_support::{
    layout, layout_with_diagnostics, AccentKind, AtomKind, BoxContent, Error, FontError,
    LayoutDiagnostic, MathFont, MathNode, MathParams, MathStyle,
};

const MISSING: char = '\u{10FFFF}';

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

fn write_u16(bytes: &mut [u8], offset: usize, value: u16) {
    bytes[offset..offset + 2].copy_from_slice(&value.to_be_bytes());
}

fn write_u32(bytes: &mut [u8], offset: usize, value: u32) {
    bytes[offset..offset + 4].copy_from_slice(&value.to_be_bytes());
}

fn push_u16(bytes: &mut Vec<u8>, value: u16) {
    bytes.extend_from_slice(&value.to_be_bytes());
}

fn push_u32(bytes: &mut Vec<u8>, value: u32) {
    bytes.extend_from_slice(&value.to_be_bytes());
}

fn table_record(font: &[u8], tag: &[u8; 4]) -> usize {
    let count = usize::from(read_u16(font, 4));
    (0..count)
        .map(|index| 12 + index * 16)
        .find(|offset| &font[*offset..*offset + 4] == tag)
        .expect("fixture contains requested OpenType table")
}

fn replace_table(font: &[u8], tag: &[u8; 4], table: &[u8]) -> Vec<u8> {
    let mut bytes = font.to_vec();
    while bytes.len() % 4 != 0 {
        bytes.push(0);
    }
    let offset = bytes.len();
    bytes.extend_from_slice(table);

    let record = table_record(&bytes, tag);
    write_u32(
        &mut bytes,
        record + 8,
        u32::try_from(offset).expect("test font offset fits u32"),
    );
    write_u32(
        &mut bytes,
        record + 12,
        u32::try_from(table.len()).expect("test table length fits u32"),
    );
    bytes
}

fn cmap_format12(mappings: &[(char, u16)]) -> Vec<u8> {
    let mut mappings = mappings.to_vec();
    mappings.sort_unstable_by_key(|(ch, _)| u32::from(*ch));

    let mut subtable = Vec::new();
    push_u16(&mut subtable, 12);
    push_u16(&mut subtable, 0);
    push_u32(
        &mut subtable,
        u32::try_from(16 + mappings.len() * 12).expect("test cmap length fits u32"),
    );
    push_u32(&mut subtable, 0);
    push_u32(
        &mut subtable,
        u32::try_from(mappings.len()).expect("test cmap group count fits u32"),
    );
    for (ch, glyph_id) in mappings {
        let scalar = u32::from(ch);
        push_u32(&mut subtable, scalar);
        push_u32(&mut subtable, scalar);
        push_u32(&mut subtable, u32::from(glyph_id));
    }

    let mut cmap = Vec::new();
    push_u16(&mut cmap, 0);
    push_u16(&mut cmap, 1);
    push_u16(&mut cmap, 3);
    push_u16(&mut cmap, 10);
    push_u32(&mut cmap, 12);
    cmap.extend_from_slice(&subtable);
    cmap
}

fn zero_notdef_advance(font: &mut [u8]) {
    let hmtx_record = table_record(font, b"hmtx");
    let hmtx_offset = usize::try_from(read_u32(font, hmtx_record + 8)).expect("hmtx offset fits");
    write_u16(font, hmtx_offset, 0);
}

#[test]
fn missing_required_scalar_uses_notdef_metrics_and_emits_typed_diagnostic() {
    let font = common::stix_two_math().expect("STIX Two Math");
    assert_eq!(
        font.glyph(MISSING),
        Err(Error::Font(FontError::MissingGlyph { ch: MISSING })),
        "fixture must not map the sentinel through cmap"
    );
    let notdef = font
        .glyph_id(MISSING, 0)
        .expect("fixture exposes glyph 0 horizontal metrics");
    assert_ne!(
        notdef.advance_fu, 0,
        "fixture must exercise the glyph-0 degradation branch"
    );

    let node = MathNode::Atom(MISSING, AtomKind::Ord);
    let output = layout_with_diagnostics(&node, &font, MathStyle::Text)
        .expect("missing required scalar is recoverable");

    assert_eq!(
        output.diagnostics,
        vec![LayoutDiagnostic::MissingGlyph { ch: MISSING }]
    );
    assert_eq!(output.math_box.width, notdef.advance);
    assert_eq!(output.math_box.height, notdef.height);
    assert_eq!(output.math_box.depth, notdef.depth);
    assert_eq!(output.math_box.italic, texpose::Dim::zero());
    assert!(matches!(
        output.math_box.content,
        BoxContent::Glyph {
            ch: MISSING,
            glyph_id: 0,
            ..
        }
    ));

    assert_eq!(
        layout(&node, &font, MathStyle::Text).expect("compatibility layout also degrades"),
        output.math_box,
        "compatibility wrapper must differ only by discarding diagnostics"
    );
}

#[test]
fn unusable_notdef_advance_uses_one_current_em_placeholder() {
    let mut bytes = common::STIX_TWO_MATH_OTF.to_vec();
    zero_notdef_advance(&mut bytes);
    let font = MathFont::from_bytes(&bytes).expect("zero-advance synthetic font stays parseable");
    let notdef = font
        .glyph_id(MISSING, 0)
        .expect("glyph 0 still has an hmtx record");
    assert!(notdef.advance.is_zero());

    let style = MathStyle::Script;
    let expected_em = MathParams::from_font(&font)
        .expect("validated MATH constants")
        .em(style)
        .expect("script em fits exact dimension range");
    let output = layout_with_diagnostics(&MathNode::Atom(MISSING, AtomKind::Ord), &font, style)
        .expect("missing glyph fallback remains recoverable");

    assert_eq!(
        output.diagnostics,
        vec![LayoutDiagnostic::MissingGlyph { ch: MISSING }]
    );
    assert_eq!(output.math_box.width, expected_em);
    assert_eq!(output.math_box.height, expected_em);
    assert!(output.math_box.depth.is_zero());
    assert!(output.math_box.italic.is_zero());
    assert!(matches!(output.math_box.content, BoxContent::Empty));
}

#[test]
fn construction_candidate_misses_keep_their_fallback_without_missing_glyph_diagnostic() {
    let base = MathFont::from_bytes(common::STIX_TWO_MATH_OTF).expect("STIX fixture");
    let x = base.glyph('x').expect("fixture x glyph").glyph_id;
    let slash = base.glyph('/').expect("fixture slash glyph").glyph_id;
    let cmap = cmap_format12(&[('x', x), ('/', slash)]);
    let bytes = replace_table(common::STIX_TWO_MATH_OTF, b"cmap", &cmap);
    let font = MathFont::from_bytes(&bytes).expect("restricted cmap font");
    assert_eq!(
        font.glyph('\u{0338}'),
        Err(Error::Font(FontError::MissingGlyph { ch: '\u{0338}' }))
    );

    let node = MathNode::Accent(Box::new(MathNode::LiteralText("x".into())), AccentKind::Not);
    let output = layout_with_diagnostics(&node, &font, MathStyle::Text)
        .expect("not overlay falls back from U+0338 to slash");

    assert!(
        output.diagnostics.is_empty(),
        "internal construction candidates are not required-scalar diagnostics"
    );
}
