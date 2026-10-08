use std::sync::Arc;

use texpose::{
    layout, layout_with_em_size_pt, layout_with_numbering, parse, Color, Dim, LayoutDiagnostic,
    MathFont, MathOp, MathStyle, NumberFormat, NumberStyle, NumberingConfig, NumberingState,
};

const STIX: &[u8] = include_bytes!("fixtures/fonts/stix-two-math/STIXTwoMath-Regular.otf");
const MISSING: char = '\u{10FFFF}';

fn font() -> MathFont {
    MathFont::from_shared_bytes(Arc::from(STIX), 0).expect("STIX Two Math")
}

fn assert_finite_op(op: &MathOp) {
    match *op {
        MathOp::Glyph {
            x, baseline, scale, ..
        } => assert!(x.is_finite() && baseline.is_finite() && scale.is_finite() && scale > 0.0),
        MathOp::Rule {
            x,
            y,
            width,
            height,
            ..
        }
        | MathOp::Background {
            x,
            y,
            width,
            height,
            ..
        } => assert!(
            x.is_finite()
                && y.is_finite()
                && width.is_finite()
                && height.is_finite()
                && width > 0.0
                && height > 0.0
        ),
        MathOp::Line {
            x1,
            y1,
            x2,
            y2,
            thickness,
            ..
        } => assert!(
            x1.is_finite()
                && y1.is_finite()
                && x2.is_finite()
                && y2.is_finite()
                && thickness.is_finite()
                && thickness > 0.0
        ),
        MathOp::Frame {
            x,
            y,
            width,
            height,
            thickness,
            ..
        } => assert!(
            x.is_finite()
                && y.is_finite()
                && width.is_finite()
                && height.is_finite()
                && thickness.is_finite()
                && width > 0.0
                && height > 0.0
                && thickness > 0.0
        ),
        _ => {}
    }
}

#[test]
fn public_layout_is_flat_finite_and_bound_to_the_exact_font() {
    let font = font();
    let bytes = font.shared_bytes();
    let ast = parse(r"\frac{x_i^2+1}{\sqrt{y}}").unwrap();
    let layout = layout(&ast, &font, MathStyle::Display).unwrap();

    assert!(layout.width().is_finite() && layout.width() > 0.0);
    assert!(layout.height().is_finite() && layout.height() >= 0.0);
    assert!(layout.depth().is_finite() && layout.depth() >= 0.0);
    assert!(!layout.ops().is_empty());
    assert!(layout
        .ops()
        .iter()
        .any(|op| matches!(op, MathOp::Rule { .. })));
    for op in layout.ops() {
        assert_finite_op(op);
    }

    let layout_bytes = layout.font().shared_bytes();
    assert!(Arc::ptr_eq(&bytes, &layout_bytes));
    assert_eq!(layout.font().face_index(), font.face_index());

    let face = ttf_parser::Face::parse(layout.font().bytes(), layout.font().face_index())
        .expect("layout retains a reparsable validated face");
    assert_eq!(layout.font().units_per_em(), face.units_per_em());
    let glyph_count = face.number_of_glyphs();
    for op in layout.ops() {
        if let MathOp::Glyph { glyph_id, .. } = *op {
            assert!(glyph_id < glyph_count);
        }
    }
}

#[test]
fn paint_order_carries_background_foreground_and_frame_colors() {
    let font = font();
    let ast = parse(r"\fcolorbox{red}{yellow}{\textcolor{blue}{x}}").unwrap();
    let layout = layout(&ast, &font, MathStyle::Text).unwrap();

    let background = layout
        .ops()
        .iter()
        .position(|op| {
            matches!(
                op,
                MathOp::Background { color, .. } if *color == Color::rgb(255, 255, 0)
            )
        })
        .expect("yellow background");
    let glyph = layout
        .ops()
        .iter()
        .position(|op| {
            matches!(
                op,
                MathOp::Glyph { color, .. } if *color == Color::rgb(0, 0, 255)
            )
        })
        .expect("blue glyph");
    let frame = layout
        .ops()
        .iter()
        .position(|op| {
            matches!(
                op,
                MathOp::Frame { color, .. } if *color == Color::rgb(255, 0, 0)
            )
        })
        .expect("red frame");

    assert!(background < glyph && glyph < frame);
}

#[test]
fn cancellation_is_exposed_as_absolute_line_geometry() {
    let font = font();
    let ast = parse(r"\cancel{x}").unwrap();
    let layout = layout(&ast, &font, MathStyle::Text).unwrap();
    assert!(layout
        .ops()
        .iter()
        .any(|op| matches!(op, MathOp::Line { .. })));
}

#[test]
fn recoverable_diagnostics_are_never_discarded_by_public_layout() {
    let font = font();
    let ast = parse(&MISSING.to_string()).unwrap();
    let layout = layout(&ast, &font, MathStyle::Text).unwrap();
    let diagnostics = layout.diagnostics();
    assert_eq!(diagnostics.len(), 1);
    match &diagnostics[0] {
        LayoutDiagnostic::MissingGlyph { ch, .. } => assert_eq!(*ch, MISSING),
        _ => panic!("unexpected layout diagnostic"),
    }
}

#[test]
fn physical_sizes_are_normalized_before_the_single_float_boundary() {
    let font = font();
    let ast = parse(r"\hspace{1pt}\rule{0pt}{2pt}").unwrap();
    let layout = layout_with_em_size_pt(&ast, &font, MathStyle::Text, &Dim::from_i64(10)).unwrap();

    assert_eq!(layout.width().to_bits(), 0.1_f32.to_bits());
    assert_eq!(layout.height().to_bits(), 0.2_f32.to_bits());
    assert_eq!(layout.depth().to_bits(), 0.0_f32.to_bits());
}

#[test]
fn public_layout_configuration_is_read_only_and_builder_driven() {
    let color = Color::rgb(12, 34, 56);
    assert_eq!(color.r(), 12);
    assert_eq!(color.g(), 34);
    assert_eq!(color.b(), 56);

    let config = NumberingConfig::new()
        .with_style(NumberStyle::Roman)
        .with_start(4)
        .with_format(NumberFormat::Bracketed);
    assert_eq!(config.style(), NumberStyle::Roman);
    assert_eq!(config.start(), 4);
    assert_eq!(config.format(), NumberFormat::Bracketed);

    let font = font();
    let ast = parse(r"\begin{equation}\label{eq:public}x\end{equation}").unwrap();
    let mut state = NumberingState::new(config);
    layout_with_numbering(&ast, &font, MathStyle::Display, &mut state).unwrap();
    assert_eq!(state.label("eq:public"), Some("[iv]"));
}

#[test]
fn aligned_probes_do_not_duplicate_missing_glyph_diagnostics() {
    let font = font();
    let source = format!(r"\begin{{aligned}}x&{}\end{{aligned}}", MISSING);
    let ast = parse(&source).expect("aligned expression parses");
    let result = layout(&ast, &font, MathStyle::Display).expect("aligned expression lays out");
    let missing = result
        .diagnostics()
        .iter()
        .filter(|diagnostic| {
            matches!(diagnostic, LayoutDiagnostic::MissingGlyph { ch, .. } if *ch == MISSING)
        })
        .count();
    assert_eq!(
        missing, 1,
        "aligned geometry probes must not duplicate diagnostics"
    );
}

#[test]
fn aligned_ordinary_leading_atom_matches_explicit_empty_ord() {
    let font = font();
    let implicit = parse(r"\begin{aligned}a&x+1\\b&y+2\end{aligned}").unwrap();
    let explicit = parse(r"\begin{aligned}a&{}x+1\\b&{}y+2\end{aligned}").unwrap();
    let bare = layout(&implicit, &font, MathStyle::Display).unwrap();
    let prefixed = layout(&explicit, &font, MathStyle::Display).unwrap();
    assert_eq!(bare.width().to_bits(), prefixed.width().to_bits());
    assert_eq!(bare.height().to_bits(), prefixed.height().to_bits());
    assert_eq!(bare.depth().to_bits(), prefixed.depth().to_bits());
    assert_eq!(bare.diagnostics().len(), prefixed.diagnostics().len());
}
