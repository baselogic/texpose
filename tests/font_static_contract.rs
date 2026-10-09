#[path = "support/hash.rs"]
mod test_hash;

use texpose::{FontError, MathFont};

const STIX: &[u8] = include_bytes!("../fonts/stix-two-math/STIXTwoMath-Regular.otf");
const FIRA: &[u8] = include_bytes!("../fonts/fira-math/FiraMath-Regular.otf");
const DEJAVU_TTF: &[u8] = include_bytes!("../fonts/dejavu-math/DejaVuMathTeXGyre.ttf");
const STIX_SOURCE: &str = include_str!("../fonts/stix-two-math/SOURCE.md");
const LIBERTINUS_SOURCE: &str = include_str!("../fonts/libertinus-math/SOURCE.md");
const FIRA_SOURCE: &str = include_str!("../fonts/fira-math/SOURCE.md");
const DEJAVU_SOURCE: &str = include_str!("../fonts/dejavu-math/SOURCE.md");
const DEJAVU_LICENSE: &str = include_str!("../fonts/dejavu-math/LICENSE");

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

fn align4(bytes: &mut Vec<u8>) {
    while bytes.len() % 4 != 0 {
        bytes.push(0);
    }
}

fn append_collection_face(collection: &mut Vec<u8>, font: &[u8]) -> u32 {
    align4(collection);
    let face_offset = u32::try_from(collection.len()).expect("test collection fits u32");
    let num_tables = usize::from(read_u16(font, 4));
    let directory_len = 12 + num_tables * 16;
    assert!(
        font.len() >= directory_len,
        "fixture has a complete sfnt directory"
    );

    let directory_start = collection.len();
    collection.extend_from_slice(&font[..directory_len]);

    for table_index in 0..num_tables {
        let source_record = 12 + table_index * 16;
        let source_offset = usize::try_from(read_u32(font, source_record + 8))
            .expect("source table offset fits usize");
        let table_len = usize::try_from(read_u32(font, source_record + 12))
            .expect("source table length fits usize");
        let source_end = source_offset
            .checked_add(table_len)
            .expect("source table range does not overflow");
        assert!(source_end <= font.len(), "fixture table is in bounds");

        align4(collection);
        let target_offset = u32::try_from(collection.len()).expect("test collection fits u32");
        collection.extend_from_slice(&font[source_offset..source_end]);
        let target_record = directory_start + source_record;
        write_u32(collection, target_record + 8, target_offset);
    }

    face_offset
}

fn make_collection(faces: &[&[u8]]) -> Vec<u8> {
    let header_len = 12 + faces.len() * 4;
    let mut collection = vec![0; header_len];
    collection[..4].copy_from_slice(b"ttcf");
    write_u32(&mut collection, 4, 0x0001_0000);
    write_u32(
        &mut collection,
        8,
        u32::try_from(faces.len()).expect("test face count fits u32"),
    );

    for (index, face) in faces.iter().enumerate() {
        let offset = append_collection_face(&mut collection, face);
        write_u32(&mut collection, 12 + index * 4, offset);
    }

    collection
}

fn with_fvar_axis_count(font: &[u8], axis_count: u16) -> Vec<u8> {
    let mut bytes = font.to_vec();
    let num_tables = usize::from(read_u16(&bytes, 4));
    let record = (0..num_tables)
        .map(|index| 12 + index * 16)
        .find(|offset| &bytes[*offset..*offset + 4] == b"name")
        .expect("fixture has a replaceable name table");
    let table_offset = usize::try_from(read_u32(&bytes, record + 8)).expect("offset fits usize");
    let table_len = usize::try_from(read_u32(&bytes, record + 12)).expect("length fits usize");
    assert!(table_len >= 36, "replacement table has room for fvar data");
    assert!(
        table_offset + table_len <= bytes.len(),
        "replacement table is in bounds"
    );

    bytes[record..record + 4].copy_from_slice(b"fvar");

    // SFNT table records are sorted by tag. `ttf-parser` relies on that
    // ordering when resolving raw tables, so keep the synthetic fixture
    // structurally valid after replacing `name` with `fvar`.
    let mut records = (0..num_tables)
        .map(|index| {
            let start = 12 + index * 16;
            let mut record = [0_u8; 16];
            record.copy_from_slice(&bytes[start..start + 16]);
            record
        })
        .collect::<Vec<_>>();
    records.sort_unstable_by_key(|record| [record[0], record[1], record[2], record[3]]);
    for (index, record) in records.iter().enumerate() {
        let start = 12 + index * 16;
        bytes[start..start + 16].copy_from_slice(record);
    }
    let fvar = &mut bytes[table_offset..table_offset + table_len];
    fvar[..36].fill(0);
    write_u16(fvar, 0, 1);
    write_u16(fvar, 2, 0);
    write_u16(fvar, 4, 16);
    write_u16(fvar, 6, 2);
    write_u16(fvar, 8, axis_count);
    write_u16(fvar, 10, 20);
    write_u16(fvar, 12, 0);
    let instance_size = axis_count
        .checked_mul(4)
        .and_then(|size| size.checked_add(4))
        .expect("test axis count keeps instance size in u16");
    write_u16(fvar, 14, instance_size);

    if axis_count > 0 {
        fvar[16..20].copy_from_slice(b"wght");
        write_u32(fvar, 20, 0);
        write_u32(fvar, 24, 0);
        write_u32(fvar, 28, 0x0001_0000);
        write_u16(fvar, 32, 0);
        write_u16(fvar, 34, 256);
    }

    bytes
}

fn leak(bytes: Vec<u8>) -> &'static [u8] {
    Box::leak(bytes.into_boxed_slice())
}

fn parsed_face(font: &MathFont) -> ttf_parser::Face<'_> {
    ttf_parser::Face::parse(font.bytes(), font.face_index())
        .expect("MathFont retains a validated face identity")
}

fn glyph_advance(font: &MathFont, ch: char) -> u16 {
    let face = parsed_face(font);
    let glyph = face.glyph_index(ch).expect("fixture glyph");
    face.glyph_hor_advance(glyph)
        .expect("fixture horizontal advance")
}

#[test]
fn verification_profiles_pin_face_index_and_dejavu_provenance() {
    for (name, source) in [
        ("stix-two-math", STIX_SOURCE),
        ("libertinus-math", LIBERTINUS_SOURCE),
        ("fira-math", FIRA_SOURCE),
        ("dejavu-math", DEJAVU_SOURCE),
    ] {
        assert!(
            source.contains("Face index: `0`"),
            "{name}: SOURCE.md must pin face index 0"
        );
    }

    let sha256 = test_hash::sha256_hex(DEJAVU_TTF);
    assert!(
        DEJAVU_SOURCE.contains(&format!("SHA-256: `{sha256}`")),
        "dejavu-math: SOURCE.md SHA-256 does not match fixture bytes"
    );
    assert!(
        DEJAVU_LICENSE.contains("Bitstream Vera Fonts Copyright"),
        "dejavu-math: packaged license text is missing"
    );
}

#[test]
fn static_otf_and_ttf_faces_load_with_explicit_profile_index() {
    let otf = MathFont::from_bytes_at_index(STIX, 0).expect("static OTF");
    let ttf = MathFont::from_bytes_at_index(DEJAVU_TTF, 0).expect("static TTF");

    assert_eq!(otf.face_index(), 0);
    assert_eq!(ttf.face_index(), 0);
    assert!(parsed_face(&otf).tables().math.is_some());
    assert!(parsed_face(&ttf).tables().math.is_some());
    assert_eq!(&STIX[..4], b"OTTO");
    assert_eq!(&DEJAVU_TTF[..4], &[0, 1, 0, 0]);
    assert!(matches!(
        MathFont::from_bytes_at_index(STIX, 1),
        Err(FontError::FaceIndexOutOfBounds)
    ));
}

#[test]
fn collections_require_and_honor_an_explicit_face_index() {
    let collection = leak(make_collection(&[STIX, FIRA]));
    assert_eq!(ttf_parser::fonts_in_collection(collection), Some(2));

    assert!(matches!(
        MathFont::from_bytes(collection),
        Err(FontError::CollectionFaceIndexRequired)
    ));

    let direct_stix = MathFont::from_bytes_at_index(STIX, 0).expect("direct STIX");
    let direct_fira = MathFont::from_bytes_at_index(FIRA, 0).expect("direct Fira");
    let collection_stix = MathFont::from_bytes_at_index(collection, 0).expect("collection STIX");
    let collection_fira = MathFont::from_bytes_at_index(collection, 1).expect("collection Fira");

    assert_eq!(collection_stix.face_index(), 0);
    assert_eq!(collection_fira.face_index(), 1);
    assert_eq!(
        glyph_advance(&collection_stix, 'x'),
        glyph_advance(&direct_stix, 'x')
    );
    assert_eq!(
        glyph_advance(&collection_fira, 'x'),
        glyph_advance(&direct_fira, 'x')
    );
    assert!(matches!(
        MathFont::from_bytes_at_index(collection, 2),
        Err(FontError::FaceIndexOutOfBounds)
    ));
}

#[test]
fn truetype_collection_faces_use_the_same_explicit_index_contract() {
    let collection = leak(make_collection(&[DEJAVU_TTF, DEJAVU_TTF]));
    assert_eq!(ttf_parser::fonts_in_collection(collection), Some(2));
    assert!(matches!(
        MathFont::from_bytes(collection),
        Err(FontError::CollectionFaceIndexRequired)
    ));

    let face = MathFont::from_bytes_at_index(collection, 1).expect("second TTC face");
    assert_eq!(face.face_index(), 1);
    assert!(parsed_face(&face).tables().math.is_some());
    assert!(parsed_face(&face).glyph_index('x').is_some());
}

#[test]
fn functional_fvar_axes_are_rejected_without_enabling_variable_font_support() {
    let variable = leak(with_fvar_axis_count(STIX, 1));
    assert!(matches!(
        MathFont::from_bytes(variable),
        Err(FontError::VariableFontUnsupported)
    ));

    let zero_axis_fvar = leak(with_fvar_axis_count(STIX, 0));
    let static_face = MathFont::from_bytes(zero_axis_fvar)
        .expect("OpenType specifies axisCount=0 fvar data as non-variable");
    assert_eq!(static_face.face_index(), 0);

    let variable_fira = with_fvar_axis_count(FIRA, 1);
    let mixed_collection = leak(make_collection(&[STIX, &variable_fira]));
    MathFont::from_bytes_at_index(mixed_collection, 0).expect("static collection face");
    assert!(matches!(
        MathFont::from_bytes_at_index(mixed_collection, 1),
        Err(FontError::VariableFontUnsupported)
    ));
}
