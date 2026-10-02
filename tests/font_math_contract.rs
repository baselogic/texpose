use texpose::{FontError, MathFont, MathParams};

const STIX: &[u8] = include_bytes!("fixtures/fonts/stix-two-math/STIXTwoMath-Regular.otf");

fn read_u16(bytes: &[u8], offset: usize) -> u16 {
    u16::from_be_bytes([bytes[offset], bytes[offset + 1]])
}

fn write_u16(bytes: &mut [u8], offset: usize, value: u16) {
    bytes[offset..offset + 2].copy_from_slice(&value.to_be_bytes());
}

fn write_i16(bytes: &mut [u8], offset: usize, value: i16) {
    bytes[offset..offset + 2].copy_from_slice(&value.to_be_bytes());
}

fn write_u32(bytes: &mut [u8], offset: usize, value: u32) {
    bytes[offset..offset + 4].copy_from_slice(&value.to_be_bytes());
}

fn align4(bytes: &mut Vec<u8>) {
    while bytes.len() % 4 != 0 {
        bytes.push(0);
    }
}

fn table_record(font: &[u8], tag: &[u8; 4]) -> usize {
    let count = usize::from(read_u16(font, 4));
    (0..count)
        .map(|index| 12 + index * 16)
        .find(|offset| &font[*offset..*offset + 4] == tag)
        .unwrap_or_else(|| panic!("fixture has {tag:?} table"))
}

fn sort_table_records(font: &mut [u8]) {
    let count = usize::from(read_u16(font, 4));
    let mut records = (0..count)
        .map(|index| {
            let start = 12 + index * 16;
            let mut record = [0_u8; 16];
            record.copy_from_slice(&font[start..start + 16]);
            record
        })
        .collect::<Vec<_>>();
    records.sort_unstable_by_key(|record| [record[0], record[1], record[2], record[3]]);
    for (index, record) in records.iter().enumerate() {
        let start = 12 + index * 16;
        font[start..start + 16].copy_from_slice(record);
    }
}

fn without_math_table(font: &[u8]) -> Vec<u8> {
    let mut bytes = font.to_vec();
    let record = table_record(&bytes, b"MATH");
    bytes[record..record + 4].copy_from_slice(b"MATI");
    sort_table_records(&mut bytes);
    bytes
}

fn replace_math_table(font: &[u8], math: &[u8]) -> Vec<u8> {
    let mut bytes = font.to_vec();
    let record = table_record(&bytes, b"MATH");
    align4(&mut bytes);
    let offset = u32::try_from(bytes.len()).expect("test font fits u32");
    bytes.extend_from_slice(math);
    write_u32(&mut bytes, record + 8, offset);
    write_u32(
        &mut bytes,
        record + 12,
        u32::try_from(math.len()).expect("test MATH length fits u32"),
    );
    bytes
}

fn math_header(constants_offset: u16) -> Vec<u8> {
    let mut math = vec![0; 10];
    write_u16(&mut math, 0, 1);
    write_u16(&mut math, 2, 0);
    write_u16(&mut math, 4, constants_offset);
    math
}

fn minimal_math(axis_height: i16, with_device: bool) -> Vec<u8> {
    const HEADER_LEN: usize = 10;
    const CONSTANTS_LEN: usize = 214;
    const AXIS_HEIGHT_OFFSET: usize = 12;

    let mut math = math_header(HEADER_LEN as u16);
    math.resize(HEADER_LEN + CONSTANTS_LEN, 0);
    let axis = HEADER_LEN + AXIS_HEIGHT_OFFSET;
    write_i16(&mut math, axis, axis_height);

    if with_device {
        let device_offset = u16::try_from(CONSTANTS_LEN).expect("constants offset fits u16");
        write_u16(&mut math, axis + 2, device_offset);
        // Device format 1, eight PPEM entries (10..=17), one packed u16.
        // The first packed correction is +1. TeXpose deliberately ignores it.
        math.extend_from_slice(&[0, 10, 0, 17, 0, 1, 0x40, 0]);
    }

    math
}

#[test]
fn construction_preserves_absent_and_malformed_math_states() {
    assert!(matches!(
        MathFont::from_bytes(b"not an OpenType font"),
        Err(FontError::InvalidFace)
    ));
    assert!(matches!(
        MathFont::from_bytes_at_index(STIX, 1),
        Err(FontError::FaceIndexOutOfBounds)
    ));

    let missing = without_math_table(STIX);
    assert!(matches!(
        MathFont::from_bytes(&missing),
        Err(FontError::MissingMathTable)
    ));

    let malformed = replace_math_table(STIX, &[0, 1, 0, 0, 0, 10, 0, 0]);
    assert!(matches!(
        MathFont::from_bytes(&malformed),
        Err(FontError::MalformedMathTable)
    ));

    let mut wrong_version = math_header(10);
    wrong_version.resize(224, 0);
    write_u16(&mut wrong_version, 0, 2);
    let wrong_version = replace_math_table(STIX, &wrong_version);
    assert!(matches!(
        MathFont::from_bytes(&wrong_version),
        Err(FontError::MalformedMathTable)
    ));
}

#[test]
fn construction_distinguishes_missing_and_malformed_math_constants() {
    let missing_constants = replace_math_table(STIX, &math_header(0));
    assert!(matches!(
        MathFont::from_bytes(&missing_constants),
        Err(FontError::MissingMathConstants)
    ));

    let malformed_constants = replace_math_table(STIX, &math_header(10));
    assert!(matches!(
        MathFont::from_bytes(&malformed_constants),
        Err(FontError::MalformedMathConstants)
    ));

    let valid = replace_math_table(STIX, &minimal_math(123, false));
    MathFont::from_bytes(&valid).expect("complete MathConstants payload");
}

#[test]
fn math_value_device_corrections_are_parseable_but_do_not_change_layout_values() {
    let design_only = replace_math_table(STIX, &minimal_math(123, false));
    let with_device = replace_math_table(STIX, &minimal_math(123, true));

    let design_font = MathFont::from_bytes(&design_only).expect("design-only MATH");
    let device_font = MathFont::from_bytes(&with_device).expect("MATH with Device correction");

    let parsed = device_font.face();
    let axis = parsed
        .tables()
        .math
        .and_then(|math| math.constants)
        .expect("validated constants")
        .axis_height();
    assert_eq!(axis.value, 123);
    assert!(
        axis.device.is_some(),
        "synthetic Device table must be parseable"
    );

    let design = MathParams::from_font(&design_font).expect("design params");
    let device = MathParams::from_font(&device_font).expect("device params");
    assert_eq!(device.axis_height, design.axis_height);
}
