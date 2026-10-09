//! The 68 dvipsnames CMYK definitions checked against LaTeX's drivers.dtx.
//!
//! Source: the `%<*dvipsnames>` section generating dvipsnam.def:
//! https://github.com/latex3/latex2e/blob/develop/required/graphics/drivers.dtx
//! Reference constants adapted from LaTeX-Rust 2.1.1, commit e377ecb,
//! tests/dvipsnames_upstream.rs (LPPL data attribution in NOTICE).

use texpose::{named_color, parse_color_spec, Dim};

const TABLE: &str = include_str!("../data/dvipsnames.tsv");

// (name, cyan, magenta, yellow, black) from drivers.dtx, in dvips order.
const UPSTREAM: &[(&str, &str, &str, &str, &str)] = &[
    ("GreenYellow", "0.15", "0", "0.69", "0"),
    ("Yellow", "0", "0", "1", "0"),
    ("Goldenrod", "0", "0.10", "0.84", "0"),
    ("Dandelion", "0", "0.29", "0.84", "0"),
    ("Apricot", "0", "0.32", "0.52", "0"),
    ("Peach", "0", "0.50", "0.70", "0"),
    ("Melon", "0", "0.46", "0.50", "0"),
    ("YellowOrange", "0", "0.42", "1", "0"),
    ("Orange", "0", "0.61", "0.87", "0"),
    ("BurntOrange", "0", "0.51", "1", "0"),
    ("Bittersweet", "0", "0.75", "1", "0.24"),
    ("RedOrange", "0", "0.77", "0.87", "0"),
    ("Mahogany", "0", "0.85", "0.87", "0.35"),
    ("Maroon", "0", "0.87", "0.68", "0.32"),
    ("BrickRed", "0", "0.89", "0.94", "0.28"),
    ("Red", "0", "1", "1", "0"),
    ("OrangeRed", "0", "1", "0.50", "0"),
    ("RubineRed", "0", "1", "0.13", "0"),
    ("WildStrawberry", "0", "0.96", "0.39", "0"),
    ("Salmon", "0", "0.53", "0.38", "0"),
    ("CarnationPink", "0", "0.63", "0", "0"),
    ("Magenta", "0", "1", "0", "0"),
    ("VioletRed", "0", "0.81", "0", "0"),
    ("Rhodamine", "0", "0.82", "0", "0"),
    ("Mulberry", "0.34", "0.90", "0", "0.02"),
    ("RedViolet", "0.07", "0.90", "0", "0.34"),
    ("Fuchsia", "0.47", "0.91", "0", "0.08"),
    ("Lavender", "0", "0.48", "0", "0"),
    ("Thistle", "0.12", "0.59", "0", "0"),
    ("Orchid", "0.32", "0.64", "0", "0"),
    ("DarkOrchid", "0.40", "0.80", "0.20", "0"),
    ("Purple", "0.45", "0.86", "0", "0"),
    ("Plum", "0.50", "1", "0", "0"),
    ("Violet", "0.79", "0.88", "0", "0"),
    ("RoyalPurple", "0.75", "0.90", "0", "0"),
    ("BlueViolet", "0.86", "0.91", "0", "0.04"),
    ("Periwinkle", "0.57", "0.55", "0", "0"),
    ("CadetBlue", "0.62", "0.57", "0.23", "0"),
    ("CornflowerBlue", "0.65", "0.13", "0", "0"),
    ("MidnightBlue", "0.98", "0.13", "0", "0.43"),
    ("NavyBlue", "0.94", "0.54", "0", "0"),
    ("RoyalBlue", "1", "0.50", "0", "0"),
    ("Blue", "1", "1", "0", "0"),
    ("Cerulean", "0.94", "0.11", "0", "0"),
    ("Cyan", "1", "0", "0", "0"),
    ("ProcessBlue", "0.96", "0", "0", "0"),
    ("SkyBlue", "0.62", "0", "0.12", "0"),
    ("Turquoise", "0.85", "0", "0.20", "0"),
    ("TealBlue", "0.86", "0", "0.34", "0.02"),
    ("Aquamarine", "0.82", "0", "0.30", "0"),
    ("BlueGreen", "0.85", "0", "0.33", "0"),
    ("Emerald", "1", "0", "0.50", "0"),
    ("JungleGreen", "0.99", "0", "0.52", "0"),
    ("SeaGreen", "0.69", "0", "0.50", "0"),
    ("Green", "1", "0", "1", "0"),
    ("ForestGreen", "0.91", "0", "0.88", "0.12"),
    ("PineGreen", "0.92", "0", "0.59", "0.25"),
    ("LimeGreen", "0.50", "0", "1", "0"),
    ("YellowGreen", "0.44", "0", "0.74", "0"),
    ("SpringGreen", "0.26", "0", "0.76", "0"),
    ("OliveGreen", "0.64", "0", "0.95", "0.40"),
    ("RawSienna", "0", "0.72", "1", "0.45"),
    ("Sepia", "0", "0.83", "1", "0.70"),
    ("Brown", "0", "0.81", "1", "0.60"),
    ("Tan", "0.14", "0.42", "0.56", "0"),
    ("Gray", "0", "0", "0", "0.50"),
    ("Black", "0", "0", "0", "1"),
    ("White", "0", "0", "0", "0"),
];

#[test]
fn dvipsnames_match_drivers_dtx_and_named_colors() {
    assert_eq!(UPSTREAM.len(), 68, "LaTeX dvipsnames color count");
    assert_eq!(
        named_color("TealBlue").expect("TealBlue").css_hex(),
        "#23faa5",
        "CMYK correction must be observable at the public color API"
    );

    let mut lines = TABLE.lines().filter(|line| {
        let trimmed = line.trim();
        !trimmed.is_empty() && !trimmed.starts_with('#')
    });
    assert_eq!(lines.next(), Some("name\tc\tm\ty\tk"));

    let mut actual = std::collections::BTreeMap::new();
    let mut original_order = Vec::new();
    for line in lines {
        let parts: Vec<_> = line.split('\t').collect();
        assert_eq!(parts.len(), 5, "five TSV columns: {line}");
        let channels = [
            Dim::parse(parts[1]).expect("cyan"),
            Dim::parse(parts[2]).expect("magenta"),
            Dim::parse(parts[3]).expect("yellow"),
            Dim::parse(parts[4]).expect("black"),
        ];
        original_order.push(parts[0]);
        assert!(
            actual.insert(parts[0], channels).is_none(),
            "duplicate name: {line}"
        );
    }
    assert_eq!(actual.len(), UPSTREAM.len(), "exactly 68 unique colors");
    assert!(
        original_order.windows(2).all(|pair| pair[0] < pair[1]),
        "table must be strictly alphabetically sorted"
    );

    for &(name, c, m, y, k) in UPSTREAM {
        let row = actual
            .get(name)
            .unwrap_or_else(|| panic!("missing dvips name: {name}"));
        for (index, expected) in [c, m, y, k].iter().enumerate() {
            let channel = Dim::parse(expected).expect("valid upstream CMYK component");
            assert!(row[index].eq_dim(&channel), "{name} channel {index}");
        }
        let expected_rgb = parse_color_spec("cmyk", &format!("{c},{m},{y},{k}"), None)
            .expect("upstream CMYK conversion");
        assert_eq!(
            named_color(name).expect("named dvips color"),
            expected_rgb,
            "{name} RGB"
        );
    }
}
