use crate::layout::display::{dim_to_f32, flatten_exact, ExactMathOp};
use crate::test_support::{
    layout_with_em_size_pt, parse, styled_char, BoxContent, Dim, MathBox, MathFont, MathStyle,
    TextStyle,
};

#[derive(Clone, Copy)]
struct MathComparisonCase<'a> {
    name: &'a str,
    family: &'a str,
    source: &'a str,
    display: bool,
    size_pt: i64,
}

const MATH_COMPARISON_CASES: &[MathComparisonCase<'static>] = &[
    MathComparisonCase {
        name: "text-simple",
        family: "basic",
        source: r"x+y",
        display: false,
        size_pt: 10,
    },
    MathComparisonCase {
        name: "text-scripts",
        family: "scripts",
        source: r"x_i^2",
        display: false,
        size_pt: 10,
    },
    MathComparisonCase {
        name: "text-superscript",
        family: "scripts",
        source: r"i^2",
        display: false,
        size_pt: 10,
    },
    MathComparisonCase {
        name: "text-fraction",
        family: "fractions",
        source: r"\frac{1}{2}",
        display: false,
        size_pt: 10,
    },
    MathComparisonCase {
        name: "display-simple",
        family: "basic",
        source: r"x+y",
        display: true,
        size_pt: 10,
    },
    MathComparisonCase {
        name: "display-fraction",
        family: "fractions",
        source: r"\frac{a+b}{c+d}",
        display: true,
        size_pt: 10,
    },
    MathComparisonCase {
        name: "display-nested-fraction",
        family: "fractions",
        source: r"\frac{1+\frac{a}{b}}{1+\frac{c}{d}}",
        display: true,
        size_pt: 10,
    },
    MathComparisonCase {
        name: "display-delimited",
        family: "delimiters",
        source: r"\left(\frac{a+b}{c+d}\right)",
        display: true,
        size_pt: 10,
    },
    MathComparisonCase {
        name: "display-sum-limits",
        family: "operators",
        source: r"\sum_{i=1}^{n}",
        display: true,
        size_pt: 10,
    },
    MathComparisonCase {
        name: "display-sum",
        family: "operators",
        source: r"\sum_{i=1}^{n} i^2",
        display: true,
        size_pt: 10,
    },
    MathComparisonCase {
        name: "radical-plain",
        family: "radicals",
        source: r"\sqrt{x}",
        display: true,
        size_pt: 10,
    },
    MathComparisonCase {
        name: "radical-tall",
        family: "radicals",
        source: r"\sqrt{x^2+y^2}",
        display: true,
        size_pt: 10,
    },
    MathComparisonCase {
        name: "radical-fraction",
        family: "radicals",
        source: r"\sqrt{\frac{a+b}{c+d}}",
        display: true,
        size_pt: 10,
    },
    MathComparisonCase {
        name: "radical-index",
        family: "radical-degree",
        source: r"\sqrt[17]{x}",
        display: true,
        size_pt: 10,
    },
    MathComparisonCase {
        name: "radical-index-compound",
        family: "radical-degree",
        source: r"\sqrt[\frac{1+\alpha}{2}]{x}",
        display: true,
        size_pt: 10,
    },
    MathComparisonCase {
        name: "radical-index-tall",
        family: "radical-degree",
        source: r"\sqrt[17]{\frac{a+b}{c+d}}",
        display: true,
        size_pt: 10,
    },
    MathComparisonCase {
        name: "accent-hat-j",
        family: "accents",
        source: r"\hat J",
        display: false,
        size_pt: 10,
    },
    MathComparisonCase {
        name: "accent-widehat-j",
        family: "accents",
        source: r"\widehat J",
        display: false,
        size_pt: 10,
    },
    MathComparisonCase {
        name: "accent-widehat-xyz",
        family: "accents",
        source: r"\widehat{XYZ}",
        display: false,
        size_pt: 10,
    },
    MathComparisonCase {
        name: "accent-widetilde-xyz",
        family: "accents",
        source: r"\widetilde{XYZ}",
        display: false,
        size_pt: 10,
    },
    MathComparisonCase {
        name: "accent-widehat-script",
        family: "accents",
        source: r"\widehat{x}_i^2",
        display: false,
        size_pt: 10,
    },
    MathComparisonCase {
        name: "display-matrix",
        family: "matrices",
        source: r"\begin{pmatrix}\frac{1}{2}&x\\y&z\end{pmatrix}",
        display: true,
        size_pt: 10,
    },
    MathComparisonCase {
        name: "display-cases",
        family: "cases",
        source: r"\begin{cases}x,&x<0\\y,&x\ge0\end{cases}",
        display: true,
        size_pt: 10,
    },
    MathComparisonCase {
        name: "display-aligned",
        family: "aligned",
        source: r"\begin{aligned}a&=b\\c&=\frac{1}{2}\end{aligned}",
        display: true,
        size_pt: 10,
    },
    MathComparisonCase {
        name: "display-substack",
        family: "substack",
        source: r"\substack{1\le i\le n\\i\ne j}",
        display: true,
        size_pt: 10,
    },
];

const MATH_COMPARISON_STRESS_TSV: &str =
    include_str!("../../../tests/fixtures/math_compare_stress.tsv");

fn parse_stress_cases(tsv: &str) -> Vec<MathComparisonCase<'_>> {
    use std::collections::BTreeSet;

    let mut cases = Vec::new();
    let mut names = BTreeSet::new();
    for (line_index, raw) in tsv.lines().enumerate() {
        let line = raw.trim_end_matches('\r');
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let mut fields = line.split('\t');
        let name = fields.next().expect("stress case name");
        let family = fields.next().expect("stress case family");
        let style = fields.next().expect("stress case style");
        let size_pt = fields
            .next()
            .expect("stress case size")
            .parse::<i64>()
            .unwrap_or_else(|_| panic!("invalid stress size at line {}", line_index + 1));
        let source = fields
            .next()
            .unwrap_or_else(|| panic!("missing stress source at line {}", line_index + 1));
        assert!(
            fields.next().is_none(),
            "unexpected stress field at line {}",
            line_index + 1
        );
        assert!(
            [name, family].iter().all(|value| {
                value
                    .bytes()
                    .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_'))
            }),
            "unsupported stress case name at line {}",
            line_index + 1
        );
        assert!(
            !name.is_empty() && !family.is_empty() && !source.is_empty(),
            "empty stress field at line {}",
            line_index + 1
        );
        assert!(
            (1..=4096).contains(&size_pt),
            "stress size out of bounds at line {}",
            line_index + 1
        );
        let display = match style {
            "text" => false,
            "display" => true,
            _ => panic!("invalid stress style at line {}", line_index + 1),
        };
        assert!(
            names.insert(name),
            "duplicate stress case name '{name}' at line {}",
            line_index + 1
        );
        cases.push(MathComparisonCase {
            name,
            family,
            source,
            display,
            size_pt,
        });
    }
    cases
}

fn stress_math_comparison_cases() -> Vec<MathComparisonCase<'static>> {
    let cases = parse_stress_cases(MATH_COMPARISON_STRESS_TSV);
    assert_eq!(
        cases.len(),
        73,
        "stress corpus census changed; review its coverage"
    );
    cases
}

struct Measurement<'a> {
    case: MathComparisonCase<'a>,
    names: Vec<&'a str>,
    families: std::collections::BTreeSet<&'a str>,
}

fn unique_measurements<'a>(cases: &[MathComparisonCase<'a>]) -> Vec<Measurement<'a>> {
    use std::collections::{BTreeMap, BTreeSet};

    let mut indices = BTreeMap::new();
    let mut names = BTreeSet::new();
    let mut measurements: Vec<Measurement<'a>> = Vec::new();
    for case in cases {
        assert!(
            names.insert(case.name),
            "duplicate corpus name: {}",
            case.name
        );
        let key = (case.source, case.display, case.size_pt);
        if let Some(&index) = indices.get(&key) {
            let measurement: &mut Measurement<'a> = &mut measurements[index];
            measurement.names.push(case.name);
            measurement.families.insert(case.family);
        } else {
            indices.insert(key, measurements.len());
            measurements.push(Measurement {
                case: *case,
                names: vec![case.name],
                families: BTreeSet::from([case.family]),
            });
        }
    }
    measurements
}

#[test]
fn math_comparison_stress_fixture_is_well_formed() {
    let cases = stress_math_comparison_cases();
    assert!(cases.iter().any(|case| case.size_pt == 6));
    assert!(cases.iter().any(|case| case.size_pt == 40));
    assert!(cases.iter().any(|case| case.family == "composite"));
    assert!(cases.iter().any(|case| case.family == "matrices"));
    let all: Vec<_> = MATH_COMPARISON_CASES
        .iter()
        .chain(&cases)
        .copied()
        .collect();
    let unique = unique_measurements(&all);
    assert_eq!(all.len(), 98);
    assert_eq!(unique.len(), 93);
    assert_eq!(
        unique.iter().map(|item| item.names.len()).sum::<usize>(),
        98
    );
    let fraction = unique
        .iter()
        .find(|item| item.case.name == "display-fraction")
        .unwrap();
    assert!(fraction.names.contains(&"size-frac-10pt"));
    assert_eq!(fraction.families.len(), 2);
}

#[test]
fn malformed_stress_records_are_rejected() {
    for tsv in [
        "case\tfamily\ttext\t10\tx\textra",
        "case\tbad family\ttext\t10\tx",
        "case\tfamily\ttext\t10\t",
        "case\tfamily\ttext\t0\tx",
        "case\tfamily\tinvalid\t10\tx",
        "case\tfamily\ttext\t10\tx\ncase\tfamily\ttext\t10\ty",
    ] {
        assert!(std::panic::catch_unwind(|| parse_stress_cases(tsv)).is_err());
    }
}

fn utf8_hex(text: &str) -> String {
    use std::fmt::Write as _;

    let capacity = text
        .len()
        .checked_mul(2)
        .expect("bounded math fixture hex capacity fits usize");
    let mut out = String::with_capacity(capacity);
    for byte in text.bytes() {
        write!(&mut out, "{byte:02x}").expect("writing to String cannot fail");
    }
    out
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct BoxStats {
    glyphs: usize,
    rules: usize,
    ops: usize,
}

impl BoxStats {
    fn add(&mut self, other: Self) {
        self.glyphs += other.glyphs;
        self.rules += other.rules;
        self.ops += other.ops;
    }
}

fn box_stats(math_box: &MathBox) -> BoxStats {
    let mut stats = BoxStats::default();
    match &math_box.content {
        BoxContent::Empty | BoxContent::Kern(_) => {}
        BoxContent::Rule => {
            if math_box.width > Dim::zero()
                && (math_box.height > Dim::zero() || math_box.depth > Dim::zero())
            {
                stats.rules = 1;
                stats.ops = 1;
            }
        }
        BoxContent::Glyph { .. } => {
            stats.glyphs = 1;
            stats.ops = 1;
        }
        BoxContent::HList(children)
        | BoxContent::VList(children)
        | BoxContent::Overlap(children) => {
            for child in children {
                stats.add(box_stats(child));
            }
        }
        BoxContent::Color(_, inner) => stats.add(box_stats(inner)),
        BoxContent::PaintCopies { inner, .. } => {
            for _ in 0..3 {
                stats.add(box_stats(inner));
            }
        }
        BoxContent::BackColor(_, inner) => {
            stats.ops = 1;
            stats.add(box_stats(inner));
        }
        BoxContent::Line { .. } => stats.ops = 1,
        BoxContent::Frame { inner, .. } => {
            stats.ops = 1;
            stats.add(box_stats(inner));
        }
    }
    stats
}

#[test]
fn structural_rule_census_ignores_nonpainting_zero_width_struts() {
    let strut = MathBox::rule(Dim::zero(), Dim::one(), Dim::one());
    let visible = MathBox::rule(Dim::one(), Dim::one(), Dim::zero());
    let tree = MathBox::hpack(vec![strut, visible]).expect("rule census fixture");

    let stats = box_stats(&tree);
    assert_eq!(stats.rules, 1);
    assert_eq!(stats.ops, 1);
}

fn dim_ratio_text(value: &Dim) -> String {
    let (num, den) = value.as_ratio();
    format!("{num}/{den}")
}

fn oracle_trace(math_box: &MathBox) -> Result<Vec<ExactMathOp>, texpose::NumericError> {
    Ok(flatten_exact(math_box)?
        .into_iter()
        .filter(|primitive| {
            matches!(
                primitive,
                ExactMathOp::Glyph { .. } | ExactMathOp::Rule { .. }
            )
        })
        .collect())
}

fn print_trace(case: &str, trace: &[ExactMathOp]) {
    for (index, primitive) in trace.iter().enumerate() {
        match primitive {
            ExactMathOp::Glyph {
                glyph_id,
                x,
                baseline,
                scale,
                ..
            } => println!(
                "TEXPOSE_MATH_TRACE case={} index={} kind=glyph glyph_id={} x={} baseline={} scale={}",
                case,
                index,
                glyph_id,
                dim_ratio_text(x),
                dim_ratio_text(baseline),
                dim_ratio_text(scale),
            ),
            ExactMathOp::Rule {
                x,
                y,
                width,
                height,
                ..
            } => println!(
                "TEXPOSE_MATH_TRACE case={} index={} kind=rule x={} bottom={} width={} height={}",
                case,
                index,
                dim_ratio_text(x),
                dim_ratio_text(y),
                dim_ratio_text(width),
                dim_ratio_text(height),
            ),
            ExactMathOp::Line { .. }
            | ExactMathOp::Frame { .. }
            | ExactMathOp::Background { .. } => {
                unreachable!("oracle trace projection retains only glyphs and rules")
            }
        }
    }
}

fn required_oracle_env(name: &str) -> String {
    std::env::var(name).unwrap_or_else(|_| panic!("missing oracle environment variable {name}"))
}

#[test]
#[ignore = "explicit LuaLaTeX differential math-layout probe"]
fn lualatex_math_comparison_probe() {
    // LuaLaTeX is an external differential oracle, not a production dependency.
    // The verifier owns one immutable font file and supplies that exact path to
    // both this probe and the reference engine.
    let path = required_oracle_env("TEXPOSE_MATH_COMPARE_FONT");
    let expected_hash = required_oracle_env("TEXPOSE_MATH_COMPARE_FONT_SHA256");
    let face_index = required_oracle_env("TEXPOSE_MATH_COMPARE_FACE_INDEX")
        .parse::<u32>()
        .expect("oracle face index must be u32");
    let profile = required_oracle_env("TEXPOSE_MATH_COMPARE_PROFILE");
    let revision = required_oracle_env("TEXPOSE_MATH_COMPARE_REVISION");
    let bytes = std::fs::read(&path).expect("verifier-owned oracle font must be readable");
    let actual_hash = crate::hash::sha256_hex(&bytes);
    assert_eq!(
        actual_hash, expected_hash,
        "oracle font hash changed before probe"
    );
    let font = MathFont::from_bytes_at_index(&bytes, face_index)
        .expect("verifier-owned oracle font face must construct");
    let control_char = styled_char('x', TextStyle::It);
    let control_glyph_id = font
        .glyph(control_char)
        .expect("oracle profile must contain the default italic x control glyph")
        .glyph_id;
    println!(
        "TEXPOSE_MATH_COMPARE_META profile={} revision={} font_sha256={} face_index={} control_glyph_id={}",
        profile, revision, actual_hash, face_index, control_glyph_id
    );

    let stress_enabled =
        std::env::var("TEXPOSE_MATH_COMPARE_STRESS").is_ok_and(|value| value == "1");
    let stress_cases = if stress_enabled {
        stress_math_comparison_cases()
    } else {
        Vec::new()
    };

    let all: Vec<_> = MATH_COMPARISON_CASES
        .iter()
        .chain(&stress_cases)
        .copied()
        .collect();
    let measurements = unique_measurements(&all);
    println!(
        "TEXPOSE_MATH_COMPARE_CASES names={}",
        measurements
            .iter()
            .map(|item| item.case.name)
            .collect::<Vec<_>>()
            .join(",")
    );
    println!(
        "TEXPOSE_MATH_COMPARE_ALIASES names={}",
        all.iter()
            .map(|case| case.name)
            .collect::<Vec<_>>()
            .join(",")
    );
    for measurement in &measurements {
        let case = &measurement.case;
        let style = if case.display {
            MathStyle::Display
        } else {
            MathStyle::Text
        };
        let ast = parse(case.source).unwrap();
        let layout =
            layout_with_em_size_pt(&ast, &font, style, &Dim::from_i64(case.size_pt)).unwrap();

        let width = dim_to_f32(&layout.width).expect("finite exact width");
        let ascent = dim_to_f32(&layout.height).expect("finite exact ascent");
        let descent = dim_to_f32(&layout.depth).expect("finite exact descent");
        assert!(width.is_finite() && width >= 0.0);
        assert!(ascent.is_finite() && ascent >= 0.0);
        assert!(descent.is_finite() && descent >= 0.0);

        let stats = box_stats(&layout);
        let trace =
            oracle_trace(&layout).expect("oracle trace coordinates must stay representable");
        let style_name = if case.display { "display" } else { "text" };

        println!(
            "TEXPOSE_MATH_COMPARE case={} family={} aliases={} style={} size_pt={} source_utf8_hex={} \
             width_em={:.9} ascent_em={:.9} descent_em={:.9} glyphs={} rules={} ops={} trace_primitives={}",
            case.name,
            measurement
                .families
                .iter()
                .copied()
                .collect::<Vec<_>>()
                .join(","),
            measurement.names.join(","),
            style_name,
            case.size_pt,
            utf8_hex(case.source),
            width,
            ascent,
            descent,
            stats.glyphs,
            stats.rules,
            stats.ops,
            trace.len(),
        );
        print_trace(case.name, &trace);
    }
}
