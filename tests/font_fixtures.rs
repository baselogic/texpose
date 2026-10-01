use texpose::MathFont;

const FIXTURES: &[(&str, &[u8], &str, &str)] = &[
    (
        "stix-two-math",
        include_bytes!("fixtures/fonts/stix-two-math/STIXTwoMath-Regular.otf"),
        include_str!("fixtures/fonts/stix-two-math/SOURCE.md"),
        include_str!("fixtures/fonts/stix-two-math/OFL.txt"),
    ),
    (
        "libertinus-math",
        include_bytes!("fixtures/fonts/libertinus-math/LibertinusMath-Regular.otf"),
        include_str!("fixtures/fonts/libertinus-math/SOURCE.md"),
        include_str!("fixtures/fonts/libertinus-math/OFL.txt"),
    ),
    (
        "fira-math",
        include_bytes!("fixtures/fonts/fira-math/FiraMath-Regular.otf"),
        include_str!("fixtures/fonts/fira-math/SOURCE.md"),
        include_str!("fixtures/fonts/fira-math/OFL.txt"),
    ),
];

#[test]
fn committed_verification_fonts_match_their_provenance_and_load_as_math_faces() {
    for (name, bytes, source, license) in FIXTURES {
        let sha256 = MathFont::sha256_hex(bytes);
        assert!(
            source.contains(&format!("SHA-256: `{sha256}`")),
            "{name}: SOURCE.md SHA-256 does not match fixture bytes"
        );
        assert!(
            license.contains("SIL OPEN FONT LICENSE Version 1.1"),
            "{name}: OFL.txt does not contain the OFL 1.1 license"
        );

        let font = MathFont::from_bytes(bytes).unwrap_or_else(|error| panic!("{name}: {error}"));
        assert!(
            font.face().tables().math.is_some(),
            "{name}: missing MATH table"
        );
        assert!(font.glyph('x').is_ok(), "{name}: missing ordinary x glyph");
    }
}
