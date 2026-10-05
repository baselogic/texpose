mod common;

use texpose::{layout, parse, styled_char, BoxContent, MathBox, MathStyle, TextStyle};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum VisibleHead {
    Glyph(char),
    Rule,
}

fn first_visible(boxed: &MathBox) -> Option<VisibleHead> {
    match &boxed.content {
        BoxContent::Glyph { ch, .. } => Some(VisibleHead::Glyph(*ch)),
        BoxContent::Rule => Some(VisibleHead::Rule),
        BoxContent::HList(children)
        | BoxContent::VList(children)
        | BoxContent::Overlap(children) => children.iter().find_map(first_visible),
        BoxContent::Color(_, inner)
        | BoxContent::BackColor(_, inner)
        | BoxContent::Frame { inner, .. } => first_visible(inner),
        BoxContent::Empty | BoxContent::Kern(_) | BoxContent::Line { .. } => None,
    }
}

fn overlap_heads(source: &str, style: MathStyle) -> Vec<VisibleHead> {
    let font = common::stix_two_math().expect("STIX Two Math fixture");
    let ast = parse(source).expect("parse stacked construct");
    let boxed = layout(&ast, &font, style).expect("layout stacked construct");
    let BoxContent::Overlap(children) = &boxed.content else {
        panic!("{source}: expected top-level overlap");
    };
    children
        .iter()
        .map(|child| first_visible(child).expect("stack branch must paint a primitive"))
        .collect()
}

#[test]
fn stacked_overlap_children_follow_tex_top_to_bottom_paint_order() {
    let italic_x = styled_char('x', TextStyle::It);
    let italic_i = styled_char('i', TextStyle::It);
    let italic_n = styled_char('n', TextStyle::It);

    assert_eq!(
        overlap_heads(r"\hat{x}", MathStyle::Text),
        vec![VisibleHead::Glyph('\u{0302}'), VisibleHead::Glyph(italic_x),],
        "top accents must paint before their nucleus"
    );
    assert_eq!(
        overlap_heads(r"\overline{x}", MathStyle::Text),
        vec![VisibleHead::Rule, VisibleHead::Glyph(italic_x)],
        "overline rule must precede its nucleus"
    );
    assert_eq!(
        overlap_heads(r"\underline{x}", MathStyle::Text),
        vec![VisibleHead::Glyph(italic_x), VisibleHead::Rule],
        "underline rule must follow its nucleus"
    );
    let under_accent = overlap_heads(r"\underleftarrow{x}", MathStyle::Text);
    assert_eq!(
        under_accent.len(),
        2,
        "under-accent overlap must contain base and accent branches"
    );
    assert_eq!(
        under_accent[0],
        VisibleHead::Glyph(italic_x),
        "under-accent base must paint before its accent"
    );
    assert_eq!(
        overlap_heads(r"\overset{n}{x}", MathStyle::Display),
        vec![VisibleHead::Glyph(italic_n), VisibleHead::Glyph(italic_x),],
        "overset annotation must precede its base"
    );
    assert_eq!(
        overlap_heads(r"\underset{i}{x}", MathStyle::Display),
        vec![VisibleHead::Glyph(italic_x), VisibleHead::Glyph(italic_i),],
        "underset annotation must follow its base"
    );
    assert_eq!(
        overlap_heads(r"\sum_{i=1}^{n}", MathStyle::Display),
        vec![
            VisibleHead::Glyph(italic_n),
            VisibleHead::Glyph('∑'),
            VisibleHead::Glyph(italic_i),
        ],
        "operator limits must paint upper, nucleus, then lower"
    );
}
