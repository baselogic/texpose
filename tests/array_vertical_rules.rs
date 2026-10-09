use std::sync::Arc;

use texpose::{layout_with_em_size_pt, parse, Dim, MathFont, MathOp, MathStyle};

const STIX: &[u8] = include_bytes!("../fonts/stix-two-math/STIXTwoMath-Regular.otf");

fn rule_rectangles(source: &str, size: i64) -> Vec<(f32, f32, f32, f32)> {
    let font = MathFont::from_shared_bytes(Arc::from(STIX), 0).expect("STIX Two Math");
    let ast = parse(source).expect("valid array");
    let laid = layout_with_em_size_pt(&ast, &font, MathStyle::Text, &Dim::from_i64(size))
        .expect("array layout");
    laid.ops()
        .iter()
        .filter_map(|op| match *op {
            MathOp::Rule {
                x,
                y,
                width,
                height,
                ..
            } => Some((x, y, width, height)),
            _ => None,
        })
        .collect()
}

fn near(actual: f32, expected: f32) {
    assert!(
        (actual - expected).abs() <= 0.000_002,
        "{actual} != {expected}"
    );
}

#[test]
fn array_uses_physical_struts_without_spacer_rows() {
    let source = r"\begin{array}{|c|}\rule{0pt}{0pt}\\\rule{0pt}{0pt}\end{array}";
    for em_pt in [6, 10, 20, 40] {
        let mut rules = rule_rectangles(source, em_pt);
        assert_eq!(rules.len(), 4, "two vertical borders per data row");
        rules.sort_by(|a, b| a.0.total_cmp(&b.0).then(b.1.total_cmp(&a.1)));
        assert!(
            rules[0].0 < rules[2].0,
            "left and right borders must be distinct"
        );
        for pair in rules.chunks_exact(2) {
            let first = pair[0];
            let second = pair[1];
            near(first.2, second.2);
            near(first.0, second.0);
            near(first.3, 12.0 / em_pt as f32);
            near(second.3, 12.0 / em_pt as f32);
            near(first.1, second.1 + second.3);
        }
    }
}

#[test]
fn all_array_vertical_borders_join_on_adjacent_data_rows() {
    let mut rules = rule_rectangles(r"\begin{array}{|c|c|}a&b\\c&d\end{array}", 10);
    assert_eq!(rules.len(), 6, "three rules per row, two rows");
    rules.sort_by(|a, b| a.0.total_cmp(&b.0).then(b.1.total_cmp(&a.1)));
    for pair in rules.chunks_exact(2) {
        near(pair[0].0, pair[1].0);
        near(pair[0].2, pair[1].2);
        near(pair[0].1, pair[1].1 + pair[1].3);
    }
}

#[test]
fn array_hline_fills_gap_between_vertical_rule_segments() {
    let rules = rule_rectangles(r"\begin{array}{|c|c|}a&b\\\hline c&d\end{array}", 10);
    assert_eq!(rules.len(), 7, "six vertical segments and one hline");
    let (line, vertical): (Vec<_>, Vec<_>) = rules.into_iter().partition(|rule| rule.2 > 0.2);
    assert_eq!(line.len(), 1);
    assert_eq!(vertical.len(), 6);
    let mut vertical = vertical;
    vertical.sort_by(|a, b| a.0.total_cmp(&b.0).then(b.1.total_cmp(&a.1)));
    let hline = line[0];
    for pair in vertical.chunks_exact(2) {
        near(pair[0].0, pair[1].0);
        near(pair[0].1, hline.1 + hline.3);
        near(pair[1].1 + pair[1].3, hline.1);
    }
}

#[test]
fn tall_array_cell_expands_its_vertical_borders_without_extra_spacing() {
    let mut rules = rule_rectangles(
        r"\begin{array}{|c|}\rule{0pt}{20pt}\\\rule{0pt}{0pt}\end{array}",
        10,
    );
    assert_eq!(rules.len(), 4, "two vertical borders per data row");
    rules.sort_by(|a, b| a.0.total_cmp(&b.0).then(b.1.total_cmp(&a.1)));
    assert!(
        rules[0].0 < rules[2].0,
        "left and right borders must be distinct"
    );
    for pair in rules.chunks_exact(2) {
        near(pair[0].0, pair[1].0);
        near(pair[0].2, pair[1].2);
        near(pair[0].3, 2.36); // 20pt high + 3.6pt row strut depth
        near(pair[1].3, 1.2);
        near(pair[0].1, pair[1].1 + pair[1].3);
    }
}
