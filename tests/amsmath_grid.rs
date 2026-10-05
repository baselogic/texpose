mod common;

use core::cmp::Ordering;

use texpose::{
    layout_with_diagnostics, layout_with_em_size_pt, parse, BoxContent, Dim, LayoutDiagnostic,
    MathBox, MathFont, MathParams, MathStyle,
};

const STIX: &[u8] = include_bytes!("fixtures/fonts/stix-two-math/STIXTwoMath-Regular.otf");
const LIBERTINUS: &[u8] =
    include_bytes!("fixtures/fonts/libertinus-math/LibertinusMath-Regular.otf");
const FIRA: &[u8] = include_bytes!("fixtures/fonts/fira-math/FiraMath-Regular.otf");
const MISSING: char = '\u{10FFFF}';
const TEX_ARRAY_COLSEP_TOTAL_PT: i64 = 10;

fn profile_fonts() -> [(&'static str, MathFont); 3] {
    [
        ("stix", MathFont::from_bytes(STIX).expect("STIX Two Math")),
        (
            "libertinus",
            MathFont::from_bytes(LIBERTINUS).expect("Libertinus Math"),
        ),
        ("fira", MathFont::from_bytes(FIRA).expect("Fira Math")),
    ]
}

fn add(a: &Dim, b: &Dim) -> Dim {
    a.checked_add(b).unwrap()
}
fn sub(a: &Dim, b: &Dim) -> Dim {
    a.checked_sub(b).unwrap()
}
fn mul(a: &Dim, b: &Dim) -> Dim {
    a.checked_mul(b).unwrap()
}
fn div(a: &Dim, b: &Dim) -> Dim {
    a.checked_div(b).unwrap()
}

fn hlist_children(bx: &MathBox) -> &[MathBox] {
    let BoxContent::HList(children) = &bx.content else {
        panic!("expected HList");
    };

    children
}

fn vlist_children(bx: &MathBox) -> &[MathBox] {
    let BoxContent::VList(children) = &bx.content else {
        panic!("expected VList");
    };

    children
}

fn environment_stack(tree: &MathBox) -> &MathBox {
    if matches!(&tree.content, BoxContent::VList(_)) {
        return tree;
    }

    hlist_children(tree)
        .iter()
        .find(|child| matches!(&child.content, BoxContent::VList(_)))
        .expect("environment stack")
}

fn assert_axis_centered(stack: &MathBox, axis: &Dim) {
    let height = add(&stack.height, &stack.shift).clamp_nonneg();
    let depth = sub(&stack.depth, &stack.shift).clamp_nonneg();
    let center = div(&sub(&height, &depth), &Dim::from_i64(2));

    assert!(
        center.eq_dim(axis),
        "stack center {} != axis {}",
        center.to_dec_string(),
        axis.to_dec_string()
    );
}

fn kern_width(bx: &MathBox) -> &Dim {
    let BoxContent::Kern(width) = &bx.content else {
        panic!("expected kern");
    };

    width
}

#[test]
fn matrix_uses_textstyle_physical_array_spacing_and_axis_center() {
    let font = common::stix_two_math().expect("STIX Two Math");

    let params = MathParams::from_font(&font).expect("MATH constants");

    let style = MathStyle::Display;

    let em_size_pt = Dim::from_i64(20);

    let axis = mul(&params.axis_height, &params.scale(style));

    let ast = parse(r"\begin{bmatrix}\frac{1}{2}&x\\y&z\end{bmatrix}").expect("bmatrix");

    let tree = layout_with_em_size_pt(&ast, &font, style, &em_size_pt).expect("bmatrix layout");

    let outer = hlist_children(&tree);

    assert_eq!(
        outer.len(),
        3,
        "expected left delimiter, stack and right delimiter"
    );

    let stack = &outer[1];

    assert_axis_centered(stack, &axis);

    for delimiter in [&outer[0], &outer[2]] {
        let center = div(&sub(&delimiter.height, &delimiter.depth), &Dim::from_i64(2));
        assert!(add(&center, &delimiter.shift).eq_dim(&axis));
    }

    let rows = vlist_children(stack);

    assert_eq!(rows.len(), 2,);

    let parts = hlist_children(&rows[0]);

    assert_eq!(
        parts.len(),
        4,
        "strut, first cell, intercolumn kern, second cell"
    );

    let fraction_ast = parse(r"\frac{1}{2}").expect("fraction");

    let text_fraction = layout_with_em_size_pt(&fraction_ast, &font, MathStyle::Text, &em_size_pt)
        .expect("text fraction");

    let display_fraction =
        layout_with_em_size_pt(&fraction_ast, &font, MathStyle::Display, &em_size_pt)
            .expect("display fraction");

    assert!(parts[1].height.eq_dim(&text_fraction.height));

    assert!(parts[1].depth.eq_dim(&text_fraction.depth));

    assert_ne!(parts[1].height, display_fraction.height);

    let expected_gap = div(&Dim::from_i64(10), &em_size_pt);

    assert!(kern_width(&parts[2]).eq_dim(&expected_gap));

    assert_ne!(
        expected_gap,
        Dim::one(),
        "20pt fixture must expose physical spacing"
    );
}

#[test]
fn cases_use_arraystretch_quad_gap_and_axis_center() {
    let font = common::stix_two_math().expect("STIX Two Math");

    let params = MathParams::from_font(&font).expect("MATH constants");

    let style = MathStyle::Display;

    let em_size_pt = Dim::from_i64(10);

    let axis = mul(&params.axis_height, &params.scale(style));

    let ast = parse(r"\begin{cases}x,&x<0\\y,&x\ge0\end{cases}").expect("cases");

    let tree = layout_with_em_size_pt(&ast, &font, style, &em_size_pt).expect("cases layout");

    let outer = hlist_children(&tree);

    assert_eq!(
        outer.len(),
        3,
        "expected left brace, cases stack and right null delimiter"
    );

    let stack = &outer[1];
    let expected_null = div(&Dim::ratio(6, 5).unwrap(), &em_size_pt);
    assert!(kern_width(&outer[2]).eq_dim(&expected_null));

    assert_axis_centered(stack, &axis);

    let rows = vlist_children(stack);

    assert_eq!(rows.len(), 2,);

    for row in rows {
        let parts = hlist_children(row);

        assert_eq!(parts.len(), 4, "strut, first cell, quad, second cell");

        let BoxContent::Rule = &parts[0].content else {
            panic!("expected cases strut");
        };

        assert!(parts[0].height.eq_dim(&Dim::ratio(84, 100).unwrap()));

        assert!(parts[0].depth.eq_dim(&Dim::ratio(36, 100).unwrap()));

        assert!(kern_width(&parts[2]).eq_dim(&Dim::one()));
    }
}

#[test]
fn cases_right_null_delimiter_remains_physical_across_root_em_sizes_and_profiles() {
    let ast = parse(r"\begin{cases}x,&x<0\\y,&x\ge0\end{cases}").expect("cases");

    for (name, font) in profile_fonts() {
        for size in [6_i64, 10, 20, 40] {
            let em_size_pt = Dim::from_i64(size);
            let tree = layout_with_em_size_pt(&ast, &font, MathStyle::Display, &em_size_pt)
                .unwrap_or_else(|error| panic!("{name} {size}pt cases: {error}"));
            let outer = hlist_children(&tree);
            assert_eq!(outer.len(), 3, "{name} {size}pt cases outer shape");

            let expected = div(&Dim::ratio(6, 5).unwrap(), &em_size_pt);
            assert!(
                kern_width(&outer[2]).eq_dim(&expected),
                "{name} {size}pt null delimiter {} != {}",
                kern_width(&outer[2]).to_dec_string(),
                expected.to_dec_string()
            );
        }
    }
}

#[test]
fn aligned_applies_empty_ord_right_field_preamble() {
    let font = common::stix_two_math().expect("STIX Two Math");

    let em_size_pt = Dim::from_i64(10);

    let ast = parse(r"\begin{aligned}a&=b\end{aligned}").expect("aligned");

    let tree =
        layout_with_em_size_pt(&ast, &font, MathStyle::Text, &em_size_pt).expect("aligned layout");

    let stack = environment_stack(&tree);

    let rows = vlist_children(stack);

    assert_eq!(rows.len(), 1,);

    let left = parse("a").expect("left field");

    let right = parse("=b").expect("right field");

    let prefixed = parse("{}=b").expect("empty-Ord right field");

    let left = layout_with_em_size_pt(&left, &font, MathStyle::Display, &em_size_pt)
        .expect("left display layout");

    let right = layout_with_em_size_pt(&right, &font, MathStyle::Display, &em_size_pt)
        .expect("right display layout");

    let prefixed = layout_with_em_size_pt(&prefixed, &font, MathStyle::Display, &em_size_pt)
        .expect("prefixed display layout");

    let leading = sub(&prefixed.width, &right.width).clamp_nonneg();

    assert!(!leading.is_zero(), "fixture must expose Ord-to-Rel spacing");

    let expected = add(&left.width, &prefixed.width);

    assert!(rows[0].width.eq_dim(&expected));
}

#[test]
fn aligned_uses_displaystyle_cells_and_physical_minalignsep() {
    let font = common::stix_two_math().expect("STIX Two Math");

    let em_size_pt = Dim::from_i64(20);

    let ast = parse(r"\begin{aligned}\frac{1}{2}&=x&y&=z\end{aligned}").expect("aligned");

    let tree =
        layout_with_em_size_pt(&ast, &font, MathStyle::Text, &em_size_pt).expect("aligned layout");

    let stack = environment_stack(&tree);

    let rows = vlist_children(stack);

    assert_eq!(rows.len(), 1,);

    let parts = hlist_children(&rows[0]);

    assert_eq!(parts.len(), 6, "strut, four fields and one pair separator");

    let fraction_ast = parse(r"\frac{1}{2}").expect("fraction");

    let display_fraction =
        layout_with_em_size_pt(&fraction_ast, &font, MathStyle::Display, &em_size_pt)
            .expect("display fraction");

    assert!(parts[1].height.eq_dim(&display_fraction.height));

    assert!(parts[1].depth.eq_dim(&display_fraction.depth));

    let expected_pair_gap = div(&Dim::from_i64(10), &em_size_pt);

    assert!(kern_width(&parts[3]).eq_dim(&expected_pair_gap));

    assert_ne!(
        expected_pair_gap,
        Dim::one(),
        "20pt fixture must expose physical spacing"
    );
}

#[test]
fn aligned_uses_jot_lineskip_and_centers_complete_stack() {
    let font = common::stix_two_math().expect("STIX Two Math");

    let params = MathParams::from_font(&font).expect("MATH constants");

    let style = MathStyle::Display;

    let em_size_pt = Dim::from_i64(20);

    let axis = mul(&params.axis_height, &params.scale(style));

    let ast = parse(r"\begin{aligned}a&=b\\c&=d\end{aligned}").expect("aligned");

    let tree = layout_with_em_size_pt(&ast, &font, style, &em_size_pt).expect("aligned layout");

    let stack = environment_stack(&tree);

    assert_axis_centered(stack, &axis);

    let children = vlist_children(stack);

    assert_eq!(children.len(), 3, "row, inter-row glue, row");

    let first = &children[0];

    let gap = &children[1];

    let second = &children[2];

    let BoxContent::Empty = &gap.content else {
        panic!("expected aligned inter-row glue");
    };

    let jot = div(&Dim::from_i64(3), &em_size_pt);
    let baseline_skip = add(&Dim::one(), &jot);
    let line_skip = add(&div(&Dim::from_i64(1), &em_size_pt), &jot);
    let candidate = sub(&sub(&baseline_skip, &first.depth), &second.height);

    let expected_gap = if candidate.cmp(&jot) == Ordering::Less {
        line_skip
    } else {
        candidate
    };

    assert!(gap.depth.eq_dim(&expected_gap,));
}

#[test]
fn matrix_physical_column_spacing_is_root_em_resolved_across_profiles() {
    let ast = parse(r"\begin{matrix}a&b\end{matrix}").expect("matrix");

    for (name, font) in profile_fonts() {
        for size in [6_i64, 10, 20, 40] {
            let tree =
                layout_with_em_size_pt(&ast, &font, MathStyle::Display, &Dim::from_i64(size))
                    .unwrap_or_else(|error| panic!("{name} {size}pt matrix: {error}"));
            let stack = environment_stack(&tree);
            let rows = vlist_children(stack);
            assert_eq!(rows.len(), 1, "{name} {size}pt row count");
            let parts = hlist_children(&rows[0]);
            assert_eq!(parts.len(), 4, "{name} {size}pt matrix row shape");

            let expected = div(
                &Dim::from_i64(TEX_ARRAY_COLSEP_TOTAL_PT),
                &Dim::from_i64(size),
            );
            assert!(
                kern_width(&parts[2]).eq_dim(&expected),
                "{name} {size}pt matrix column gap {} != {}",
                kern_width(&parts[2]).to_dec_string(),
                expected.to_dec_string()
            );
        }
    }
}

#[test]
fn amsmath_grid_cells_clean_terminal_math_italic_across_profiles() {
    let matrix_ast = parse(r"\begin{matrix}x&x\end{matrix}").expect("matrix");
    let aligned_ast = parse(r"\begin{aligned}x&x\end{aligned}").expect("aligned");
    let x_ast = parse("x").expect("x");
    let em_size_pt = Dim::from_i64(10);

    for (name, font) in profile_fonts() {
        let text_x = layout_with_em_size_pt(&x_ast, &font, MathStyle::Text, &em_size_pt)
            .unwrap_or_else(|error| panic!("{name} text x: {error}"));
        assert!(
            !text_x.italic.is_zero(),
            "{name}: fixture must expose terminal math italic"
        );
        let expected_text = add(&text_x.width, &text_x.italic);

        let matrix = layout_with_em_size_pt(&matrix_ast, &font, MathStyle::Display, &em_size_pt)
            .unwrap_or_else(|error| panic!("{name} matrix: {error}"));
        let matrix_rows = vlist_children(environment_stack(&matrix));
        let matrix_parts = hlist_children(&matrix_rows[0]);
        assert!(
            matrix_parts[1].width.eq_dim(&expected_text)
                && matrix_parts[3].width.eq_dim(&expected_text),
            "{name}: matrix cells must include terminal math italic exactly once"
        );

        let display_x = layout_with_em_size_pt(&x_ast, &font, MathStyle::Display, &em_size_pt)
            .unwrap_or_else(|error| panic!("{name} display x: {error}"));
        assert!(
            !display_x.italic.is_zero(),
            "{name}: display fixture must expose terminal math italic"
        );
        let expected_display = add(&display_x.width, &display_x.italic);

        let aligned = layout_with_em_size_pt(&aligned_ast, &font, MathStyle::Display, &em_size_pt)
            .unwrap_or_else(|error| panic!("{name} aligned: {error}"));
        let aligned_rows = vlist_children(environment_stack(&aligned));
        let aligned_parts = hlist_children(&aligned_rows[0]);
        assert_eq!(aligned_parts.len(), 3, "{name} aligned row shape");
        assert!(
            aligned_parts[1].width.eq_dim(&expected_display)
                && aligned_parts[2].width.eq_dim(&expected_display),
            "{name}: aligned fields must include terminal math italic exactly once"
        );
    }
}

#[test]
fn matrix_empty_cells_preserve_shared_column_widths_and_alignment_points() {
    let ast = parse(r"\begin{matrix}a&\\&bb\end{matrix}").expect("matrix with empty cells");

    for (name, font) in profile_fonts() {
        let tree = layout_with_em_size_pt(&ast, &font, MathStyle::Display, &Dim::from_i64(10))
            .unwrap_or_else(|error| panic!("{name} empty-cell matrix: {error}"));
        let rows = vlist_children(environment_stack(&tree));
        assert_eq!(rows.len(), 2, "{name} row count");
        assert!(
            rows[0].width.eq_dim(&rows[1].width),
            "{name} rows must share the measured column grid"
        );

        for row in rows {
            let parts = hlist_children(row);
            assert_eq!(
                parts.len(),
                4,
                "{name} row must retain two alignment fields"
            );
            assert!(matches!(&parts[0].content, BoxContent::Rule));
            assert!(matches!(&parts[2].content, BoxContent::Kern(_)));
        }
    }
}

#[test]
fn matrix_degrades_missing_cell_glyph_without_losing_grid_structure() {
    let font = common::stix_two_math().expect("STIX Two Math");
    let source = format!(r"\begin{{matrix}}a&{MISSING}\\b&c\end{{matrix}}");
    let ast = parse(&source).expect("matrix with missing scalar");
    let output = layout_with_diagnostics(&ast, &font, MathStyle::Display)
        .expect("missing matrix cell glyph must degrade deterministically");

    assert_eq!(
        output.diagnostics,
        vec![LayoutDiagnostic::MissingGlyph { ch: MISSING }]
    );
    let rows = vlist_children(environment_stack(&output.math_box));
    assert_eq!(rows.len(), 2);
    assert!(rows.iter().all(|row| hlist_children(row).len() == 4));
}
