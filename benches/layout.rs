//! Parse, font-construction, and layout timings (run with `cargo bench`).
//!
//! Existing parser/layout targets remain regression checks. Font construction is
//! reported without a target until the dedicated font-access baseline is measured.

use std::time::{Duration, Instant};

use texpose::{layout, parse, MathFont, MathStyle};

const FRAC: &str = r"\frac{1}{2}";
const DISPLAY: &str = r"\sum_{n=1}^{N}\frac{1}{n^{2}}=\frac{\pi^{2}}{6}";

fn avg(iters: u32, warmup: u32, mut f: impl FnMut()) -> Duration {
    for _ in 0..warmup {
        f();
    }
    let t0 = Instant::now();
    for _ in 0..iters {
        f();
    }
    t0.elapsed() / iters
}

fn report(name: &str, got: Duration, target: Duration) {
    let ok = got <= target;
    println!(
        "{name:42} {got:>10?}  target {target:?}  {}",
        if ok { "OK" } else { "MISS" }
    );
    assert!(ok, "{name} {got:?} exceeds target {target:?}");
}

fn observe(name: &str, got: Duration) {
    println!("{name:42} {got:>10?}");
}

fn main() {
    let font_construct = avg(200, 8, || {
        let _ = MathFont::stix_two_math().expect("STIX Two Math");
    });
    observe("construct STIX Two Math", font_construct);

    let font = MathFont::stix_two_math().expect("STIX Two Math");

    let parse_frac = avg(8_000, 64, || {
        let _ = parse(FRAC).expect("parse frac");
    });
    report("parse \\frac{1}{2}", parse_frac, Duration::from_micros(50));

    let parse_disp = avg(2_000, 32, || {
        let _ = parse(DISPLAY).expect("parse display");
    });
    report(
        "parse full display equation",
        parse_disp,
        Duration::from_micros(200),
    );

    let ast_frac = parse(FRAC).expect("parse");
    let ast_disp = parse(DISPLAY).expect("parse");

    let lay_frac = avg(2_000, 32, || {
        let _ = layout(&ast_frac, &font, MathStyle::Text).expect("layout frac");
    });
    report("layout \\frac{1}{2}", lay_frac, Duration::from_micros(100));

    let lay_disp = avg(400, 16, || {
        let _ = layout(&ast_disp, &font, MathStyle::Display).expect("layout display");
    });
    report(
        "layout full display equation",
        lay_disp,
        Duration::from_micros(500),
    );
}
