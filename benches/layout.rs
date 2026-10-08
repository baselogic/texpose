//! Core performance measurements (run with `cargo bench --bench layout`).
//!
//! `cargo bench` uses Cargo's optimized bench profile. This suite reports
//! distributions instead of enforcing machine-specific latency thresholds.
//!
//! I1 measures production boundaries that exist today: `MathFont` construction,
//! `ttf_parser::Face::parse`, parsing, and simple, complex, and
//! stress layout. I3 adds representative primitive-layout workloads for scripts,
//! fractions, radicals, large delimiters, glyph assemblies, large operators,
//! matrices, and aligned environments. Semantic normalization is included in
//! layout but cannot yet be isolated without exposing a private proof boundary
//! solely for benchmarking. Since Phase J, every measured public layout call also
//! includes exact positioned flattening and the final direct `Dim -> f32` display-
//! list emission, so these rows are end-to-end consumer-boundary measurements.

use std::hint::black_box;
use std::time::{Duration, Instant};

#[path = "../tests/common/mod.rs"]
mod common;

use texpose::{layout, parse, MathStyle};

const SIMPLE: &str = "x";
const COMPLEX: &str = r"\sum_{n=1}^{N}\frac{1}{n^{2}}=\frac{\pi^{2}}{6}";
const STRESS_CASE: &str = "hard-brutal-core";
const STRESS_TSV: &str = include_str!("../tests/fixtures/math_compare_stress.tsv");

const PRIMITIVE_CASES: [(&str, &str, MathStyle); 8] = [
    ("primitive scripts", r"x_{i_j}^{a^{b^c}}", MathStyle::Text),
    (
        "primitive fractions",
        r"\frac{\frac{a}{b}}{\frac{c}{d}}",
        MathStyle::Display,
    ),
    (
        "primitive radicals",
        r"\sqrt[3]{1+x^2+y^2}",
        MathStyle::Display,
    ),
    (
        "primitive delimiters",
        r"\left(\frac{a+b}{c+d}\right)",
        MathStyle::Display,
    ),
    (
        "primitive assembly",
        r"\left\{\rule{0pt}{5em}x\right\}",
        MathStyle::Display,
    ),
    (
        "primitive large operator",
        r"\sum_{i=1}^{n} i^2",
        MathStyle::Display,
    ),
    (
        "primitive matrix",
        r"\begin{matrix}a&b&c\\d&e&f\\g&h&i\end{matrix}",
        MathStyle::Display,
    ),
    (
        "primitive aligned",
        r"\begin{aligned}a&=b+c+d,\\&=e+f+g,\\&=\frac{x+y}{z}+\sqrt{w}.\end{aligned}",
        MathStyle::Display,
    ),
];

const SAMPLE_COUNT: usize = 31;
const TARGET_SAMPLE_TIME: Duration = Duration::from_millis(10);
const MAX_BATCH_ITERS: u64 = 1 << 24;

#[derive(Clone, Copy)]
struct Distribution {
    min_ns: f64,
    median_ns: f64,
    p95_ns: f64,
    max_ns: f64,
}

fn run_batch<T>(iters: u64, f: &mut impl FnMut() -> T) -> Duration {
    let start = Instant::now();
    for _ in 0..iters {
        black_box(f());
    }
    start.elapsed()
}

fn calibrated_iters<T>(f: &mut impl FnMut() -> T) -> u64 {
    let mut iters = 1_u64;
    loop {
        let elapsed = run_batch(iters, f);
        if elapsed >= TARGET_SAMPLE_TIME || iters >= MAX_BATCH_ITERS {
            return iters;
        }
        iters = (iters.saturating_mul(2)).min(MAX_BATCH_ITERS);
    }
}

fn measure<T>(name: &str, mut f: impl FnMut() -> T) {
    let iters = calibrated_iters(&mut f);
    black_box(run_batch(iters, &mut f));

    let mut samples = Vec::with_capacity(SAMPLE_COUNT);
    for _ in 0..SAMPLE_COUNT {
        let elapsed = run_batch(iters, &mut f);
        let ns_per_op = elapsed.as_secs_f64() * 1_000_000_000.0 / iters as f64;
        samples.push(ns_per_op);
    }
    samples.sort_by(|lhs, rhs| lhs.total_cmp(rhs));

    let p95_index = (SAMPLE_COUNT * 95 - 1) / 100;
    let distribution = Distribution {
        min_ns: samples[0],
        median_ns: samples[SAMPLE_COUNT / 2],
        p95_ns: samples[p95_index],
        max_ns: samples[SAMPLE_COUNT - 1],
    };

    println!(
        "{name:30} min={:>10.1} ns  median={:>10.1} ns  p95={:>10.1} ns  max={:>10.1} ns  batch={iters}",
        distribution.min_ns, distribution.median_ns, distribution.p95_ns, distribution.max_ns,
    );
}

fn stress_source(name: &str) -> &'static str {
    STRESS_TSV
        .lines()
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .find_map(|line| {
            let mut fields = line.splitn(5, '\t');
            let case_name = fields.next()?;
            let _family = fields.next()?;
            let _style = fields.next()?;
            let _size_pt = fields.next()?;
            let source = fields.next()?;
            (case_name == name).then_some(source)
        })
        .unwrap_or_else(|| panic!("missing stress benchmark case {name}"))
}

fn main() {
    if cfg!(debug_assertions) {
        eprintln!(
            "Phase I measurements require Cargo's optimized bench profile; run `cargo bench --bench layout`"
        );
        std::process::exit(2);
    }

    println!(
        "Phase I benchmark: samples={SAMPLE_COUNT}, target_sample_ms={}, font=STIX Two Math, stress={STRESS_CASE}",
        TARGET_SAMPLE_TIME.as_millis()
    );

    measure("MathFont construction", || {
        common::stix_two_math().expect("STIX Two Math")
    });

    measure("Face::parse", || {
        let face = texpose::ttf_parser::Face::parse(
            black_box(common::STIX_TWO_MATH_OTF),
            common::STIX_TWO_MATH_FACE_INDEX,
        )
        .expect("STIX Two Math face");
        black_box(face.number_of_glyphs())
    });

    measure("parse complex", || {
        parse(black_box(COMPLEX)).expect("parse complex")
    });

    let font = common::stix_two_math().expect("STIX Two Math");
    let simple = parse(SIMPLE).expect("parse simple");
    let complex = parse(COMPLEX).expect("parse complex");
    let stress = parse(stress_source(STRESS_CASE)).expect("parse stress");
    let primitive_asts: Vec<_> = PRIMITIVE_CASES
        .iter()
        .map(|&(name, source, style)| {
            (
                name,
                parse(source).unwrap_or_else(|error| panic!("parse {name}: {error}")),
                style,
            )
        })
        .collect();

    measure("layout simple", || {
        layout(black_box(&simple), black_box(&font), MathStyle::Text).expect("layout simple")
    });

    measure("layout complex", || {
        layout(black_box(&complex), black_box(&font), MathStyle::Display).expect("layout complex")
    });

    measure("layout stress", || {
        layout(black_box(&stress), black_box(&font), MathStyle::Display).expect("layout stress")
    });

    println!("I3 primitive layout workloads");
    for (name, ast, style) in &primitive_asts {
        measure(name, || {
            layout(black_box(ast), black_box(&font), *style)
                .unwrap_or_else(|error| panic!("layout {name}: {error}"))
        });
    }
}
