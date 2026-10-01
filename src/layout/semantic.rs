//! TeX-style semantic math sequence used between syntax and geometry.

use crate::atoms::symbol_atom_kind;
use crate::layout::style::MathStyle;
use crate::parser::{AccentKind, AtomKind, MathNode, MatrixStyle, SpaceKind};

/// One item in the normalized semantic math sequence.
///
/// This is intentionally private to layout: it is a proof boundary between the
/// public syntax AST and geometry, not a second public AST.
#[derive(Clone, Copy, Debug)]
pub(super) enum SemanticItem<'a> {
    /// Current math style. Style changes are non-spacing sequence state.
    Style(MathStyle),
    /// A mathematical noad with its normalized TeX class.
    Noad { node: &'a MathNode, class: AtomKind },
    /// Explicit user-requested math glue/kern.
    Glue(&'a SpaceKind),
    /// Non-spacing control syntax that still needs semantic execution.
    Control(&'a MathNode),
}

/// Build and binary-normalize the semantic sequence for one syntax row.
pub(super) fn normalize_row<'a>(items: &'a [MathNode], style: MathStyle) -> Vec<SemanticItem<'a>> {
    let mut out = Vec::with_capacity(items.len() + 1);
    out.push(SemanticItem::Style(style));

    for node in items {
        match node {
            MathNode::Space(kind) => out.push(SemanticItem::Glue(kind)),
            _ => match noad_class(node) {
                Some(class) => out.push(SemanticItem::Noad { node, class }),
                None => out.push(SemanticItem::Control(node)),
            },
        }
    }

    reclassify_bins(&mut out);
    out
}

/// Class a syntax node as a TeX math noad without constructing geometry.
///
/// `None` means the node is non-spacing control material in a row.
pub(super) fn noad_class(node: &MathNode) -> Option<AtomKind> {
    match node {
        MathNode::Atom(_, class) => Some(*class),
        MathNode::Symbol(name) => Some(symbol_atom_kind(name)),
        MathNode::Operator(_, _)
        | MathNode::Sum(_, _)
        | MathNode::Product(_, _)
        | MathNode::Integral(_, _, _)
        | MathNode::Limit(_) => Some(AtomKind::Op),
        MathNode::Fraction(_) | MathNode::Radical(_, _) => Some(AtomKind::Ord),
        MathNode::Matrix(matrix_style, _, _) => Some(match matrix_style {
            MatrixStyle::Matrix | MatrixStyle::Array | MatrixStyle::Aligned => AtomKind::Ord,
            _ => AtomKind::Inner,
        }),
        MathNode::Delimited(_, _, _) | MathNode::ColorBox(_, _) | MathNode::FColorBox(_, _, _) => {
            Some(AtomKind::Inner)
        }
        MathNode::Substack(_) => Some(AtomKind::Ord),
        MathNode::SizedDelim(_, _, class) => Some(*class),
        MathNode::Superscript(base, _)
        | MathNode::Subscript(base, _)
        | MathNode::SubSup(base, _, _) => noad_class(base),
        MathNode::OverUnder(base, _, _) => noad_class(base),
        MathNode::Accent(base, AccentKind::Not) => noad_class(base),
        MathNode::Accent(_, _) | MathNode::CancelTo(_, _) => Some(AtomKind::Ord),
        MathNode::Text(_, _) | MathNode::Ref(_) | MathNode::Tag { .. } | MathNode::Intertext(_) => {
            Some(AtomKind::Ord)
        }
        MathNode::Color(_, body) | MathNode::TextColor(_, body) | MathNode::Phantom(_, body) => {
            noad_class(body)
        }
        MathNode::Row(nodes) if nodes.len() == 1 => noad_class(&nodes[0]),
        MathNode::Row(_) | MathNode::Strut(_, _) | MathNode::Rule(_, _) => Some(AtomKind::Ord),
        MathNode::Space(_) | MathNode::Label(_) | MathNode::NoNumber | MathNode::Hline => None,
    }
}

fn reclassify_bins(items: &mut [SemanticItem<'_>]) {
    let noads: Vec<usize> = items
        .iter()
        .enumerate()
        .filter_map(|(index, item)| matches!(item, SemanticItem::Noad { .. }).then_some(index))
        .collect();

    let mut previous = None;
    for (position, &index) in noads.iter().enumerate() {
        let next = noads
            .get(position + 1)
            .and_then(|next_index| noad_class_at(items, *next_index));
        let current = noad_class_at(items, index).expect("index came from a noad item");
        let normalized = convert_bin(previous, current, next);
        if let SemanticItem::Noad { class, .. } = &mut items[index] {
            *class = normalized;
        }
        previous = Some(normalized);
    }
}

fn noad_class_at(items: &[SemanticItem<'_>], index: usize) -> Option<AtomKind> {
    match items.get(index) {
        Some(SemanticItem::Noad { class, .. }) => Some(*class),
        _ => None,
    }
}

/// TeX Appendix G binary-operator reclassification.
fn convert_bin(prev: Option<AtomKind>, current: AtomKind, next: Option<AtomKind>) -> AtomKind {
    if current != AtomKind::Bin {
        return current;
    }

    let previous_forbids_bin = matches!(
        prev,
        None | Some(
            AtomKind::Bin | AtomKind::Op | AtomKind::Rel | AtomKind::Open | AtomKind::Punct
        )
    );
    let next_forbids_bin = matches!(
        next,
        None | Some(AtomKind::Rel | AtomKind::Close | AtomKind::Punct)
    );

    if previous_forbids_bin || next_forbids_bin {
        AtomKind::Ord
    } else {
        AtomKind::Bin
    }
}

#[cfg(test)]
mod tests {
    use super::{normalize_row, SemanticItem};
    use crate::layout::MathStyle;
    use crate::parser::{parse, AtomKind, MathNode};

    fn classes(input: &str) -> Vec<AtomKind> {
        let ast = parse(input).unwrap();
        classes_from_node(&ast)
    }

    fn classes_from_node(ast: &MathNode) -> Vec<AtomKind> {
        let items = match ast {
            MathNode::Row(items) => items.as_slice(),
            node => core::slice::from_ref(node),
        };
        normalize_row(items, MathStyle::Text)
            .into_iter()
            .filter_map(|item| match item {
                SemanticItem::Noad { class, .. } => Some(class),
                _ => None,
            })
            .collect()
    }

    #[test]
    fn binary_reclassification_uses_relevant_noads_not_adjacent_syntax() {
        assert_eq!(
            classes("a+b"),
            vec![AtomKind::Ord, AtomKind::Bin, AtomKind::Ord]
        );
        assert_eq!(
            classes(r"a\, + b"),
            vec![AtomKind::Ord, AtomKind::Bin, AtomKind::Ord]
        );
        assert_eq!(
            classes(r"a\label{x}+b"),
            vec![AtomKind::Ord, AtomKind::Bin, AtomKind::Ord]
        );
        assert_eq!(
            classes("(+a)"),
            vec![
                AtomKind::Open,
                AtomKind::Ord,
                AtomKind::Ord,
                AtomKind::Close
            ]
        );
        assert_eq!(
            classes("a+)"),
            vec![AtomKind::Ord, AtomKind::Ord, AtomKind::Close]
        );
        assert_eq!(
            classes("a,+b"),
            vec![AtomKind::Ord, AtomKind::Punct, AtomKind::Ord, AtomKind::Ord]
        );
    }

    #[test]
    fn binary_reclassification_uses_already_normalized_previous_noad() {
        assert_eq!(
            classes("++a"),
            vec![AtomKind::Ord, AtomKind::Bin, AtomKind::Ord]
        );
    }

    #[test]
    fn fraction_uses_ord_spacing_class() {
        assert_eq!(classes(r"\frac{1}{x}"), vec![AtomKind::Ord]);
    }

    #[test]
    fn radical_and_undelimited_amsmath_stacks_use_ord_spacing_class() {
        assert_eq!(classes(r"\sqrt{x}"), vec![AtomKind::Ord]);
        assert_eq!(
            classes(r"a\begin{matrix}x\\y\end{matrix}b"),
            vec![AtomKind::Ord, AtomKind::Ord, AtomKind::Ord]
        );
        assert_eq!(
            classes(r"a\begin{aligned}x&=y\\z&=w\end{aligned}b"),
            vec![AtomKind::Ord, AtomKind::Ord, AtomKind::Ord]
        );
        assert_eq!(
            classes(r"a\begin{pmatrix}x\\y\end{pmatrix}b"),
            vec![AtomKind::Ord, AtomKind::Inner, AtomKind::Ord]
        );
    }

    #[test]
    fn over_under_preserves_the_base_spacing_class() {
        assert_eq!(
            classes(r"a\overset{!}{+}b"),
            vec![AtomKind::Ord, AtomKind::Bin, AtomKind::Ord]
        );
        assert_eq!(
            classes(r"a\overset{!}{=}b"),
            vec![AtomKind::Ord, AtomKind::Rel, AtomKind::Ord]
        );
        assert_eq!(
            classes(r"a\xrightarrow{x}b"),
            vec![AtomKind::Ord, AtomKind::Rel, AtomKind::Ord]
        );
    }

    #[test]
    fn relation_after_binary_forces_binary_to_ord() {
        let ast = MathNode::Row(vec![
            MathNode::Atom('a', AtomKind::Ord),
            MathNode::Atom('+', AtomKind::Bin),
            MathNode::Atom('=', AtomKind::Rel),
            MathNode::Atom('b', AtomKind::Ord),
        ]);
        assert_eq!(
            classes_from_node(&ast),
            vec![AtomKind::Ord, AtomKind::Ord, AtomKind::Rel, AtomKind::Ord]
        );
    }

    #[test]
    fn semantic_sequence_keeps_style_glue_and_control_distinct() {
        let nodes = vec![
            MathNode::Atom('a', AtomKind::Ord),
            MathNode::Space(crate::parser::SpaceKind::Thin),
            MathNode::Label("x".into()),
        ];
        let sequence = normalize_row(&nodes, MathStyle::ScriptCramped);
        assert!(matches!(
            sequence[0],
            SemanticItem::Style(MathStyle::ScriptCramped)
        ));
        assert!(matches!(
            sequence[1],
            SemanticItem::Noad {
                class: AtomKind::Ord,
                ..
            }
        ));
        assert!(matches!(sequence[2], SemanticItem::Glue(_)));
        assert!(matches!(sequence[3], SemanticItem::Control(_)));
    }
}
