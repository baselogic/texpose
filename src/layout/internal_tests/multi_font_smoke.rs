use crate::test_support::{
    layout_with_diagnostics, parse, BoxContent, Dim, LayoutOutput, MathBox, MathFont, MathStyle,
};

#[derive(Clone, Copy)]
struct FontProfile {
    name: &'static str,
    bytes: &'static [u8],
    face_index: u32,
}

#[derive(Clone, Copy)]
struct SmokeCase {
    family: &'static str,
    source: &'static str,
    style: MathStyle,
}

const PROFILES: &[FontProfile] = &[
    FontProfile {
        name: "stix",
        bytes: include_bytes!("../../../fonts/stix-two-math/STIXTwoMath-Regular.otf",),
        face_index: 0,
    },
    FontProfile {
        name: "libertinus",
        bytes: include_bytes!("../../../fonts/libertinus-math/LibertinusMath-Regular.otf"),
        face_index: 0,
    },
    FontProfile {
        name: "fira",
        bytes: include_bytes!("../../../fonts/fira-math/FiraMath-Regular.otf"),
        face_index: 0,
    },
    FontProfile {
        name: "dejavu",
        bytes: include_bytes!("../../../fonts/dejavu-math/DejaVuMathTeXGyre.ttf"),
        face_index: 0,
    },
];

const REQUIRED_PROFILES: &[&str] = &["dejavu", "fira", "libertinus", "stix"];

const CASES: &[SmokeCase] = &[
    SmokeCase {
        family: "ordinary-symbols",
        source: r"x+y=\alpha",
        style: MathStyle::Text,
    },
    SmokeCase {
        family: "math-alphabets",
        source: r"\mathrm{x}+\mathbf{x}+\mathit{x}",
        style: MathStyle::Text,
    },
    SmokeCase {
        family: "scripts",
        source: r"x_i^2+y_{j_k}",
        style: MathStyle::Text,
    },
    SmokeCase {
        family: "fractions",
        source: r"\frac{a+b}{c+d}",
        style: MathStyle::Display,
    },
    SmokeCase {
        family: "radicals",
        source: r"\sqrt[3]{x^2+y^2}",
        style: MathStyle::Display,
    },
    SmokeCase {
        family: "delimiters",
        source: r"\left(\frac{a+b}{c+d}\right)",
        style: MathStyle::Display,
    },
    SmokeCase {
        family: "operators",
        source: r"\sum_{i=1}^{n}i^2",
        style: MathStyle::Display,
    },
    SmokeCase {
        family: "integrals",
        source: r"\int_0^1 x^2\,dx",
        style: MathStyle::Display,
    },
    SmokeCase {
        family: "accents",
        source: r"\widehat{xyz}",
        style: MathStyle::Text,
    },
    SmokeCase {
        family: "matrices",
        source: r"\begin{pmatrix}a&b\\c&d\end{pmatrix}",
        style: MathStyle::Display,
    },
    SmokeCase {
        family: "aligned",
        source: r"\begin{aligned}a&=b+c\\d&=e-f\end{aligned}",
        style: MathStyle::Display,
    },
    SmokeCase {
        family: "spacing",
        source: r"a+b\,c\!d\quad e",
        style: MathStyle::Text,
    },
];

const REQUIRED_FAMILIES: &[&str] = &[
    "accents",
    "aligned",
    "delimiters",
    "fractions",
    "integrals",
    "math-alphabets",
    "matrices",
    "operators",
    "ordinary-symbols",
    "radicals",
    "scripts",
    "spacing",
];

fn assert_dim_valid(value: &Dim, profile: &str, family: &str, field: &str) {
    let (_, denominator) = value.as_ratio();
    assert!(
        denominator > 0,
        "{profile}/{family}: {field} has a non-positive denominator"
    );
}

fn assert_box_dims_valid(math_box: &MathBox, profile: &str, family: &str) {
    for (field, value) in [
        ("width", &math_box.width),
        ("height", &math_box.height),
        ("depth", &math_box.depth),
        ("italic", &math_box.italic),
        ("shift", &math_box.shift),
    ] {
        assert_dim_valid(value, profile, family, field);
    }

    match &math_box.content {
        BoxContent::Empty | BoxContent::Rule => {}
        BoxContent::Glyph { scale, .. } => {
            assert_dim_valid(scale, profile, family, "glyph scale");
            assert!(scale > &Dim::zero(), "{profile}/{family}: glyph scale <= 0");
        }
        BoxContent::HList(children)
        | BoxContent::VList(children)
        | BoxContent::Overlap(children) => {
            for child in children {
                assert_box_dims_valid(child, profile, family);
            }
        }
        BoxContent::Kern(width) => assert_dim_valid(width, profile, family, "kern"),
        BoxContent::Color(_, inner) | BoxContent::BackColor(_, inner) => {
            assert_box_dims_valid(inner, profile, family);
        }
        BoxContent::PaintCopies { inner, dx } => {
            assert_dim_valid(dx, profile, family, "paint dx");
            assert!(dx >= &Dim::zero(), "{profile}/{family}: paint dx < 0");
            assert_box_dims_valid(inner, profile, family);
        }
        BoxContent::Line {
            x1,
            y1,
            x2,
            y2,
            thickness,
        } => {
            for (field, value) in [
                ("line x1", x1),
                ("line y1", y1),
                ("line x2", x2),
                ("line y2", y2),
                ("line thickness", thickness),
            ] {
                assert_dim_valid(value, profile, family, field);
            }
            assert!(
                thickness >= &Dim::zero(),
                "{profile}/{family}: line thickness < 0"
            );
        }
        BoxContent::Frame {
            thickness, inner, ..
        } => {
            assert_dim_valid(thickness, profile, family, "frame thickness");
            assert!(
                thickness >= &Dim::zero(),
                "{profile}/{family}: frame thickness < 0"
            );
            assert_box_dims_valid(inner, profile, family);
        }
    }
}

fn glyph_ids(math_box: &MathBox, out: &mut Vec<u16>) {
    match &math_box.content {
        BoxContent::Glyph { glyph_id, .. } => out.push(*glyph_id),
        BoxContent::HList(children)
        | BoxContent::VList(children)
        | BoxContent::Overlap(children) => {
            for child in children {
                glyph_ids(child, out);
            }
        }
        BoxContent::Color(_, inner)
        | BoxContent::BackColor(_, inner)
        | BoxContent::Frame { inner, .. }
        | BoxContent::PaintCopies { inner, .. } => glyph_ids(inner, out),
        BoxContent::Empty | BoxContent::Rule | BoxContent::Kern(_) | BoxContent::Line { .. } => {}
    }
}

fn assert_output_valid(output: &LayoutOutput, profile: &str, family: &str) -> Vec<u16> {
    assert!(
        output.diagnostics.is_empty(),
        "{profile}/{family}: unexpected recoverable diagnostics: {:?}",
        output.diagnostics
    );

    assert!(
        output.math_box.width >= Dim::zero(),
        "{profile}/{family}: top-level width < 0"
    );
    assert!(
        output.math_box.height >= Dim::zero(),
        "{profile}/{family}: top-level height < 0"
    );
    assert!(
        output.math_box.depth >= Dim::zero(),
        "{profile}/{family}: top-level depth < 0"
    );
    assert_box_dims_valid(&output.math_box, profile, family);

    let mut ids = Vec::new();
    glyph_ids(&output.math_box, &mut ids);
    assert!(
        !ids.is_empty(),
        "{profile}/{family}: smoke case produced no glyphs"
    );
    ids
}

#[test]
fn smoke_matrix_census_is_complete() {
    let mut profiles = PROFILES
        .iter()
        .map(|profile| profile.name)
        .collect::<Vec<_>>();
    profiles.sort_unstable();
    assert_eq!(profiles, REQUIRED_PROFILES);

    let mut families = CASES.iter().map(|case| case.family).collect::<Vec<_>>();
    families.sort_unstable();
    assert_eq!(families, REQUIRED_FAMILIES);
}

#[test]
fn common_math_corpus_is_deterministic_across_verification_fonts() {
    for profile in PROFILES {
        let first_font = MathFont::from_bytes_at_index(profile.bytes, profile.face_index)
            .unwrap_or_else(|error| panic!("{}: font construction failed: {error}", profile.name));
        let second_font = MathFont::from_bytes_at_index(profile.bytes, profile.face_index)
            .unwrap_or_else(|error| {
                panic!("{}: second font construction failed: {error}", profile.name)
            });

        for case in CASES {
            let first_ast = parse(case.source).unwrap_or_else(|error| {
                panic!("{}/{}: parse failed: {error}", profile.name, case.family)
            });
            let second_ast = parse(case.source).unwrap_or_else(|error| {
                panic!(
                    "{}/{}: second parse failed: {error}",
                    profile.name, case.family
                )
            });

            let first = layout_with_diagnostics(&first_ast, &first_font, case.style)
                .unwrap_or_else(|error| {
                    panic!("{}/{}: layout failed: {error}", profile.name, case.family)
                });
            let second = layout_with_diagnostics(&second_ast, &second_font, case.style)
                .unwrap_or_else(|error| {
                    panic!(
                        "{}/{}: second layout failed: {error}",
                        profile.name, case.family
                    )
                });

            let first_ids = assert_output_valid(&first, profile.name, case.family);
            let second_ids = assert_output_valid(&second, profile.name, case.family);
            assert_eq!(
                first_ids, second_ids,
                "{}/{}: glyph IDs changed between independent layouts",
                profile.name, case.family
            );
            assert_eq!(
                first, second,
                "{}/{}: exact geometry changed between independent layouts",
                profile.name, case.family
            );
        }
    }
}

#[test]
fn same_style_control_does_not_change_normalized_row_geometry() {
    for profile in PROFILES {
        let font = MathFont::from_bytes_at_index(profile.bytes, profile.face_index)
            .unwrap_or_else(|error| panic!("{}: font construction failed: {error}", profile.name));
        let plain = parse("a+b").expect("semantic control fixture parses");
        let explicit = parse(r"a\textstyle+b").expect("explicit-style fixture parses");
        let plain =
            layout_with_diagnostics(&plain, &font, MathStyle::Text).unwrap_or_else(|error| {
                panic!("{}: plain semantic layout failed: {error}", profile.name)
            });
        let explicit =
            layout_with_diagnostics(&explicit, &font, MathStyle::Text).unwrap_or_else(|error| {
                panic!(
                    "{}: explicit-style semantic layout failed: {error}",
                    profile.name
                )
            });

        assert!(
            plain.diagnostics.is_empty(),
            "{}: plain semantic diagnostic",
            profile.name
        );
        assert!(
            explicit.diagnostics.is_empty(),
            "{}: explicit-style semantic diagnostic",
            profile.name
        );
        assert_eq!(
            plain, explicit,
            "{}: non-spacing same-style control changed normalized row geometry",
            profile.name
        );
    }
}
