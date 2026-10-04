use texpose::{layout, parse, BoxContent, MathBox, MathFont, MathStyle};

const STIX: &[u8] = include_bytes!("fixtures/fonts/stix-two-math/STIXTwoMath-Regular.otf");

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

fn write_i16(bytes: &mut [u8], offset: usize, value: i16) {
    bytes[offset..offset + 2].copy_from_slice(&value.to_be_bytes());
}

fn write_u32(bytes: &mut [u8], offset: usize, value: u32) {
    bytes[offset..offset + 4].copy_from_slice(&value.to_be_bytes());
}

fn push_u16(bytes: &mut Vec<u8>, value: u16) {
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

fn set_flattened_accent_base_height(font: &mut [u8], value: i16) {
    let math_record = table_record(font, b"MATH");
    let math_offset = usize::try_from(read_u32(font, math_record + 8)).expect("MATH offset fits");
    let constants_offset = usize::from(read_u16(font, math_offset + 4));
    let flattened_value = math_offset + constants_offset + 20;
    write_i16(font, flattened_value, value);
}

fn single_format2_lookup(source: u16, target: u16) -> Vec<u8> {
    let mut subtable = Vec::new();
    push_u16(&mut subtable, 2); // SingleSubst format 2.
    push_u16(&mut subtable, 8); // Coverage follows substitute glyph array.
    push_u16(&mut subtable, 1);
    push_u16(&mut subtable, target);
    push_u16(&mut subtable, 1); // Coverage format 1.
    push_u16(&mut subtable, 1);
    push_u16(&mut subtable, source);

    lookup(1, &subtable)
}

fn chained_single_format2_lookup(first: u16, second: u16, third: u16) -> Vec<u8> {
    let mut mappings = [(first, second), (second, third)];
    mappings.sort_unstable_by_key(|(source, _)| *source);

    let mut subtable = Vec::new();
    push_u16(&mut subtable, 2); // SingleSubst format 2.
    push_u16(&mut subtable, 10); // Coverage follows substitute glyph array.
    push_u16(&mut subtable, 2);
    for (_, target) in mappings {
        push_u16(&mut subtable, target);
    }
    push_u16(&mut subtable, 1); // Coverage format 1.
    push_u16(&mut subtable, 2);
    for (source, _) in mappings {
        push_u16(&mut subtable, source);
    }

    lookup(1, &subtable)
}

fn single_format1_lookup(source: u16, target: u16) -> Vec<u8> {
    let delta_bits = target.wrapping_sub(source);
    let mut subtable = Vec::new();
    push_u16(&mut subtable, 1); // SingleSubst format 1.
    push_u16(&mut subtable, 6); // Coverage follows the fixed header.
    push_u16(&mut subtable, delta_bits);
    push_u16(&mut subtable, 1); // Coverage format 1.
    push_u16(&mut subtable, 1);
    push_u16(&mut subtable, source);

    lookup(1, &subtable)
}

fn alternate_lookup(source: u16, alternates: &[u16]) -> Vec<u8> {
    let mut subtable = Vec::new();
    push_u16(&mut subtable, 1); // AlternateSubst format 1.
    push_u16(&mut subtable, 8); // Coverage.
    push_u16(&mut subtable, 1); // AlternateSet count.
    push_u16(&mut subtable, 14); // AlternateSet follows Coverage.
    push_u16(&mut subtable, 1); // Coverage format 1.
    push_u16(&mut subtable, 1);
    push_u16(&mut subtable, source);
    push_u16(
        &mut subtable,
        u16::try_from(alternates.len()).expect("test alternate count fits u16"),
    );
    for &glyph in alternates {
        push_u16(&mut subtable, glyph);
    }

    lookup(3, &subtable)
}

fn lookup(kind: u16, subtable: &[u8]) -> Vec<u8> {
    let mut bytes = Vec::new();
    push_u16(&mut bytes, kind);
    push_u16(&mut bytes, 0); // lookupFlag
    push_u16(&mut bytes, 1); // subTableCount
    push_u16(&mut bytes, 8); // subtable offset
    bytes.extend_from_slice(subtable);
    bytes
}

fn build_gsub(
    script_tag: &[u8; 4],
    required_feature: Option<u16>,
    language_features: &[u16],
    features: &[([u8; 4], Vec<u16>)],
    lookups: &[Vec<u8>],
) -> Vec<u8> {
    assert!(
        features.windows(2).all(|pair| pair[0].0 <= pair[1].0),
        "feature records must be tag-sorted"
    );

    let mut script_list = Vec::new();
    push_u16(&mut script_list, 1);
    script_list.extend_from_slice(script_tag);
    push_u16(&mut script_list, 8);
    push_u16(&mut script_list, 4); // DefaultLangSys follows Script header.
    push_u16(&mut script_list, 0); // No language-specific systems.
    push_u16(&mut script_list, 0); // lookupOrderOffset
    push_u16(&mut script_list, required_feature.unwrap_or(0xFFFF));
    push_u16(
        &mut script_list,
        u16::try_from(language_features.len()).expect("test feature count fits u16"),
    );
    for &feature in language_features {
        push_u16(&mut script_list, feature);
    }

    let feature_header_len = 2 + features.len() * 6;
    let mut feature_list = Vec::with_capacity(feature_header_len);
    push_u16(
        &mut feature_list,
        u16::try_from(features.len()).expect("test feature count fits u16"),
    );
    let mut feature_tables = Vec::new();
    for (tag, lookup_indices) in features {
        feature_list.extend_from_slice(tag);
        push_u16(
            &mut feature_list,
            u16::try_from(feature_header_len + feature_tables.len())
                .expect("test feature offset fits u16"),
        );
        push_u16(&mut feature_tables, 0); // featureParamsOffset
        push_u16(
            &mut feature_tables,
            u16::try_from(lookup_indices.len()).expect("test lookup count fits u16"),
        );
        for &lookup_index in lookup_indices {
            push_u16(&mut feature_tables, lookup_index);
        }
    }
    feature_list.extend_from_slice(&feature_tables);

    let lookup_header_len = 2 + lookups.len() * 2;
    let mut lookup_list = Vec::with_capacity(lookup_header_len);
    push_u16(
        &mut lookup_list,
        u16::try_from(lookups.len()).expect("test lookup count fits u16"),
    );
    let mut lookup_tables = Vec::new();
    for item in lookups {
        push_u16(
            &mut lookup_list,
            u16::try_from(lookup_header_len + lookup_tables.len())
                .expect("test lookup offset fits u16"),
        );
        lookup_tables.extend_from_slice(item);
    }
    lookup_list.extend_from_slice(&lookup_tables);

    let script_offset = 10usize;
    let feature_offset = script_offset + script_list.len();
    let lookup_offset = feature_offset + feature_list.len();
    let mut gsub = Vec::new();
    push_u16(&mut gsub, 1);
    push_u16(&mut gsub, 0);
    push_u16(
        &mut gsub,
        u16::try_from(script_offset).expect("test ScriptList offset fits u16"),
    );
    push_u16(
        &mut gsub,
        u16::try_from(feature_offset).expect("test FeatureList offset fits u16"),
    );
    push_u16(
        &mut gsub,
        u16::try_from(lookup_offset).expect("test LookupList offset fits u16"),
    );
    gsub.extend_from_slice(&script_list);
    gsub.extend_from_slice(&feature_list);
    gsub.extend_from_slice(&lookup_list);
    gsub
}

fn font_with_gsub(gsub: &[u8]) -> MathFont {
    let bytes = replace_table(STIX, b"GSUB", gsub);
    MathFont::from_bytes(&bytes).expect("synthetic GSUB font")
}

fn glyph_id(source: &str, style: MathStyle, font: &MathFont) -> u16 {
    let ast = parse(source).expect("parse glyph case");
    let bx = layout(&ast, font, style).expect("layout glyph case");
    let BoxContent::Glyph { glyph_id, .. } = bx.content else {
        panic!("single glyph case must stay a glyph box");
    };
    glyph_id
}

fn first_glyph_id(bx: &MathBox) -> Option<u16> {
    match &bx.content {
        BoxContent::Glyph { glyph_id, .. } => Some(*glyph_id),
        BoxContent::HList(children)
        | BoxContent::VList(children)
        | BoxContent::Overlap(children) => children.iter().find_map(first_glyph_id),
        BoxContent::Color(_, inner)
        | BoxContent::BackColor(_, inner)
        | BoxContent::Frame { inner, .. } => first_glyph_id(inner),
        _ => None,
    }
}

fn accent_children(source: &str, style: MathStyle, font: &MathFont) -> (u16, u16) {
    let ast = parse(source).expect("parse accent case");
    let bx = layout(&ast, font, style).expect("layout accent case");
    let BoxContent::Overlap(children) = &bx.content else {
        panic!("accent result must be an overlap");
    };
    assert_eq!(children.len(), 2, "accent overlap has base and accent");
    (
        first_glyph_id(&children[0]).expect("accent base glyph"),
        first_glyph_id(&children[1]).expect("accent glyph"),
    )
}

#[test]
fn ssty_is_selected_only_from_math_default_language_and_supports_both_levels() {
    let base = MathFont::from_bytes(STIX).expect("STIX fixture");
    let two = base.glyph('2').expect("digit 2").glyph_id;
    let three = base.glyph('3').expect("digit 3").glyph_id;
    let four = base.glyph('4').expect("digit 4").glyph_id;

    let unreachable = build_gsub(
        b"math",
        None,
        &[0],
        &[(*b"liga", vec![0]), (*b"ssty", vec![1])],
        &[
            single_format2_lookup(two, four),
            alternate_lookup(two, &[three, four]),
        ],
    );
    let font = font_with_gsub(&unreachable);
    assert_eq!(glyph_id("2", MathStyle::Text, &font), two);
    assert_eq!(glyph_id("2", MathStyle::Script, &font), two);
    assert_eq!(glyph_id("2", MathStyle::ScriptScript, &font), two);

    let default_script_only = build_gsub(
        b"DFLT",
        None,
        &[0],
        &[(*b"ssty", vec![0])],
        &[alternate_lookup(two, &[three, four])],
    );
    let font = font_with_gsub(&default_script_only);
    assert_eq!(glyph_id("2", MathStyle::Script, &font), two);

    let reachable = build_gsub(
        b"math",
        Some(0),
        &[],
        &[(*b"ssty", vec![0])],
        &[alternate_lookup(two, &[three, four])],
    );
    let font = font_with_gsub(&reachable);
    assert_eq!(glyph_id("2", MathStyle::Text, &font), two);
    assert_eq!(glyph_id("2", MathStyle::Script, &font), three);
    assert_eq!(glyph_id("2", MathStyle::ScriptScript, &font), four);
}

#[test]
fn ssty_single_substitution_is_the_fallback_for_both_script_levels() {
    let base = MathFont::from_bytes(STIX).expect("STIX fixture");
    let two = base.glyph('2').expect("digit 2").glyph_id;
    let three = base.glyph('3').expect("digit 3").glyph_id;
    let gsub = build_gsub(
        b"math",
        None,
        &[0],
        &[(*b"ssty", vec![0])],
        &[single_format1_lookup(two, three)],
    );
    let font = font_with_gsub(&gsub);

    assert_eq!(glyph_id("2", MathStyle::Text, &font), two);
    assert_eq!(glyph_id("2", MathStyle::Script, &font), three);
    assert_eq!(glyph_id("2", MathStyle::ScriptScript, &font), three);
}

#[test]
fn dtls_and_ssty_share_lookup_list_order_only_for_an_accent_base() {
    let base = MathFont::from_bytes(STIX).expect("STIX fixture");
    let italic_i = base.glyph('\u{1D456}').expect("math italic i").glyph_id;
    let italic_j = base.glyph('\u{1D457}').expect("math italic j").glyph_id;
    let seven = base.glyph('7').expect("digit 7").glyph_id;

    let gsub = build_gsub(
        b"math",
        None,
        &[0, 1],
        &[(*b"dtls", vec![0]), (*b"ssty", vec![1])],
        &[
            single_format2_lookup(italic_i, italic_j),
            single_format2_lookup(italic_j, seven),
        ],
    );
    let font = font_with_gsub(&gsub);

    assert_eq!(glyph_id("i", MathStyle::Script, &font), italic_i);
    let (accent_base, _) = accent_children(r"\hat{i}", MathStyle::Script, &font);
    assert_eq!(accent_base, seven);
}

#[test]
fn lookup_shared_by_ssty_and_dtls_is_applied_once() {
    let base = MathFont::from_bytes(STIX).expect("STIX fixture");
    let italic_i = base.glyph('\u{1D456}').expect("math italic i").glyph_id;
    let italic_j = base.glyph('\u{1D457}').expect("math italic j").glyph_id;
    let seven = base.glyph('7').expect("digit 7").glyph_id;

    let gsub = build_gsub(
        b"math",
        None,
        &[0, 1],
        &[(*b"dtls", vec![0]), (*b"ssty", vec![0])],
        &[chained_single_format2_lookup(italic_i, italic_j, seven)],
    );
    let font = font_with_gsub(&gsub);

    let (accent_base, _) = accent_children(r"\hat{i}", MathStyle::Script, &font);
    assert_eq!(accent_base, italic_j);
}

#[test]
fn flac_is_single_substitution_selected_only_above_flattened_accent_base_height() {
    let base = MathFont::from_bytes(STIX).expect("STIX fixture");
    let accent_source = ['\u{0302}', 'ˆ']
        .into_iter()
        .find_map(|ch| base.glyph(ch).ok().map(|glyph| glyph.glyph_id))
        .expect("STIX hat accent glyph");
    let seven = base.glyph('7').expect("digit 7").glyph_id;
    let gsub = build_gsub(
        b"math",
        None,
        &[0],
        &[(*b"flac", vec![0])],
        &[single_format2_lookup(accent_source, seven)],
    );

    let mut flattened = replace_table(STIX, b"GSUB", &gsub);
    set_flattened_accent_base_height(&mut flattened, 0);
    let flattened = MathFont::from_bytes(&flattened).expect("flattening-enabled font");
    let (_, accent) = accent_children(r"\hat{A}", MathStyle::Text, &flattened);
    assert_eq!(accent, seven);

    let mut unflattened = replace_table(STIX, b"GSUB", &gsub);
    set_flattened_accent_base_height(&mut unflattened, i16::MAX);
    let unflattened = MathFont::from_bytes(&unflattened).expect("high-threshold font");
    let (_, accent) = accent_children(r"\hat{A}", MathStyle::Text, &unflattened);
    assert_eq!(accent, accent_source);
}
