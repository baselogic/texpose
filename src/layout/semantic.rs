//! TeX-style semantic math normalization between syntax and geometry.

use crate::atoms::symbol_atom_kind;
use crate::layout::style::MathStyle;
use crate::parser::{
    AccentKind, AtomKind, IntegralKind, LimitMode, MathNode, MathStyleDeclaration, MatrixStyle,
    SpaceKind,
};

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

/// Resolved placement of operator scripts.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum LimitPlacement {
    /// Scripts are attached to the side of the operator nucleus.
    Side,
    /// Upper/lower material is placed above/below the operator nucleus.
    Limits,
}

/// Script fields after TeX style normalization.
#[derive(Clone, Copy, Debug)]
pub(super) struct ScriptSemantics<'a> {
    pub(super) base: &'a MathNode,
    pub(super) sub: Option<&'a MathNode>,
    pub(super) sup: Option<&'a MathNode>,
    pub(super) style: MathStyle,
    pub(super) sub_style: MathStyle,
    pub(super) sup_style: MathStyle,
}

/// Geometry-level operator nucleus after source variants have been unified.
#[derive(Clone, Copy, Debug)]
pub(super) enum OperatorNucleus<'a> {
    Named(&'a str),
    Integral(IntegralKind),
}

/// Operator semantics after script styles and limit policy have been resolved.
#[derive(Clone, Copy, Debug)]
pub(super) struct OperatorSemantics<'a> {
    pub(super) nucleus: OperatorNucleus<'a>,
    pub(super) lower: Option<&'a MathNode>,
    pub(super) upper: Option<&'a MathNode>,
    pub(super) style: MathStyle,
    pub(super) lower_style: MathStyle,
    pub(super) upper_style: MathStyle,
    pub(super) placement: LimitPlacement,
}

fn overunder_class(base: &MathNode) -> AtomKind {
    match noad_class(base) {
        Some(AtomKind::Bin) => AtomKind::Bin,
        Some(AtomKind::Rel) => AtomKind::Rel,
        _ => AtomKind::Op,
    }
}

/// Build and binary-normalize the semantic sequence for one syntax row.
pub(super) fn normalize_row<'a>(items: &'a [MathNode], style: MathStyle) -> Vec<SemanticItem<'a>> {
    normalize_row_with_initial_class(items, style, None)
}

/// Normalize a right alignment field with its implicit leading empty Ord.
/// The artificial noad contributes no geometry; it only changes TeX binary
/// reclassification and introduces spacing before the first effective noad.
pub(super) fn normalize_row_with_leading_ord<'a>(
    items: &'a [MathNode],
    style: MathStyle,
) -> Vec<SemanticItem<'a>> {
    normalize_row_with_initial_class(items, style, Some(AtomKind::Ord))
}

fn normalize_row_with_initial_class<'a>(
    items: &'a [MathNode],
    style: MathStyle,
    preceding: Option<AtomKind>,
) -> Vec<SemanticItem<'a>> {
    let mut out = Vec::with_capacity(items.len() + 1);
    let mut current_style = style;
    out.push(SemanticItem::Style(current_style));

    for node in items {
        match node {
            MathNode::Style(declaration) => {
                current_style = declared_style(*declaration);
                out.push(SemanticItem::Style(current_style));
            }
            MathNode::Space(kind) => out.push(SemanticItem::Glue(kind)),
            _ => match noad_class(node) {
                Some(class) => out.push(SemanticItem::Noad { node, class }),
                None => out.push(SemanticItem::Control(node)),
            },
        }
    }

    reclassify_bins(&mut out, preceding);
    out
}

/// Normalize a non-operator scripted noad.
pub(super) fn script_semantics(node: &MathNode, style: MathStyle) -> Option<ScriptSemantics<'_>> {
    let (base, sub, sup) = peel_scripts(node);
    if sub.is_none() && sup.is_none() {
        return None;
    }
    if operator_semantics(node, style).is_some() {
        return None;
    }
    let (sub_style, sup_style) = script_styles(style);
    Some(ScriptSemantics {
        base,
        sub,
        sup,
        style,
        sub_style,
        sup_style,
    })
}

/// Normalize operator kind, scripts, script styles, and effective limit placement.
pub(super) fn operator_semantics(
    node: &MathNode,
    style: MathStyle,
) -> Option<OperatorSemantics<'_>> {
    let (script_base, outer_lower, outer_upper) = peel_scripts(node);
    let (core, explicit_mode) = match script_base {
        MathNode::Limits(core, mode) => (core.as_ref(), Some(*mode)),
        other => (other, None),
    };

    let (nucleus, inner_lower, inner_upper, limits_in_display) = operator_core(core)?;
    let lower = outer_lower.or(inner_lower);
    let upper = outer_upper.or(inner_upper);
    let (lower_style, upper_style) = script_styles(style);
    let placement = match explicit_mode {
        Some(LimitMode::Limits) => LimitPlacement::Limits,
        Some(LimitMode::NoLimits) => LimitPlacement::Side,
        None if limits_in_display && style.is_display() => LimitPlacement::Limits,
        None => LimitPlacement::Side,
    };

    Some(OperatorSemantics {
        nucleus,
        lower,
        upper,
        style,
        lower_style,
        upper_style,
        placement,
    })
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
        MathNode::Limits(body, _) => noad_class(body),
        MathNode::Fraction(_) | MathNode::Radical(_, _) => Some(AtomKind::Ord),
        MathNode::Matrix(matrix_style, _, _) => Some(match matrix_style {
            MatrixStyle::Matrix | MatrixStyle::Array | MatrixStyle::Aligned => AtomKind::Ord,
            _ => AtomKind::Inner,
        }),
        MathNode::Delimited(_, _, _) => Some(AtomKind::Inner),
        MathNode::ColorBox(_, _) | MathNode::FColorBox(_, _, _) => Some(AtomKind::Ord),
        MathNode::Substack(_) => Some(AtomKind::Ord),
        MathNode::SizedDelim(_, _, class) => Some(*class),
        MathNode::Superscript(base, _)
        | MathNode::Subscript(base, _)
        | MathNode::SubSup(base, _, _) => noad_class(base),
        MathNode::OverUnder(base, _, _) => Some(overunder_class(base)),
        MathNode::StackRel(_, _) => Some(AtomKind::Rel),
        MathNode::Accent(base, AccentKind::Not) => noad_class(base),
        MathNode::Accent(_, _) | MathNode::CancelTo(_, _) => Some(AtomKind::Ord),
        MathNode::MathAlphabet(_, _)
        | MathNode::LiteralText(_)
        | MathNode::Ref(_)
        | MathNode::Tag { .. }
        | MathNode::Intertext(_) => Some(AtomKind::Ord),
        MathNode::Color(_, body) | MathNode::TextColor(_, body) | MathNode::Pmb(body) => {
            noad_class(body)
        }
        MathNode::Phantom(_, _) => Some(AtomKind::Ord),
        MathNode::Row(nodes) if nodes.len() == 1 => noad_class(&nodes[0]),
        MathNode::Row(_) | MathNode::Strut(_, _) | MathNode::Rule(_, _) => Some(AtomKind::Ord),
        MathNode::Style(_)
        | MathNode::Space(_)
        | MathNode::Label(_)
        | MathNode::NoNumber
        | MathNode::Hline => None,
    }
}

fn declared_style(declaration: MathStyleDeclaration) -> MathStyle {
    match declaration {
        MathStyleDeclaration::Display => MathStyle::Display,
        MathStyleDeclaration::Text => MathStyle::Text,
        MathStyleDeclaration::Script => MathStyle::Script,
        MathStyleDeclaration::ScriptScript => MathStyle::ScriptScript,
    }
}

pub(super) fn script_styles(style: MathStyle) -> (MathStyle, MathStyle) {
    let sup_style = style.into_script();
    let sub_style = sup_style.cramp();
    (sub_style, sup_style)
}

fn peel_scripts(node: &MathNode) -> (&MathNode, Option<&MathNode>, Option<&MathNode>) {
    match node {
        MathNode::Superscript(base, sup) => (base, None, Some(sup)),
        MathNode::Subscript(base, sub) => (base, Some(sub), None),
        MathNode::SubSup(base, sub, sup) => (base, Some(sub), Some(sup)),
        other => (other, None, None),
    }
}

fn operator_core(
    node: &MathNode,
) -> Option<(
    OperatorNucleus<'_>,
    Option<&MathNode>,
    Option<&MathNode>,
    bool,
)> {
    match node {
        MathNode::Operator(name, limits_in_display) => {
            Some((OperatorNucleus::Named(name), None, None, *limits_in_display))
        }
        MathNode::Sum(lower, upper) => Some((
            OperatorNucleus::Named("sum"),
            lower.as_deref(),
            upper.as_deref(),
            true,
        )),
        MathNode::Product(lower, upper) => Some((
            OperatorNucleus::Named("prod"),
            lower.as_deref(),
            upper.as_deref(),
            true,
        )),
        MathNode::Integral(kind, lower, upper) => Some((
            OperatorNucleus::Integral(*kind),
            lower.as_deref(),
            upper.as_deref(),
            false,
        )),
        MathNode::Limit(lower) => {
            Some((OperatorNucleus::Named("lim"), lower.as_deref(), None, true))
        }
        _ => None,
    }
}

fn reclassify_bins(items: &mut [SemanticItem<'_>], preceding: Option<AtomKind>) {
    let noads: Vec<usize> = items
        .iter()
        .enumerate()
        .filter_map(|(index, item)| matches!(item, SemanticItem::Noad { .. }).then_some(index))
        .collect();

    let mut previous = preceding;
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
    use super::{
        normalize_row, operator_semantics, script_semantics, LimitPlacement, SemanticItem,
    };
    use crate::layout::MathStyle;
    use crate::parser::{parse, AtomKind, MathNode};

    fn classes(input: &str) -> Vec<AtomKind> {
        let ast = parse(input).unwrap();
        classes_from_node(&ast)
    }

    #[test]
    fn implicit_leading_ord_matches_an_explicit_empty_ord_in_binary_normalization() {
        use super::normalize_row_with_leading_ord;

        fn noad_classes(items: &[SemanticItem<'_>]) -> Vec<AtomKind> {
            items
                .iter()
                .filter_map(|item| match item {
                    SemanticItem::Noad { class, .. } => Some(*class),
                    _ => None,
                })
                .collect()
        }

        for source in ["+b", "=b", "a+b", r"\displaystyle +b", r"\,=b", "{}+b"] {
            let ast = parse(source).expect("valid alignment cell");
            let items = match &ast {
                MathNode::Row(items) => items.as_slice(),
                node => core::slice::from_ref(node),
            };
            let implicit = normalize_row_with_leading_ord(items, MathStyle::Display);
            let mut explicit = Vec::with_capacity(items.len() + 1);
            explicit.push(MathNode::Row(Vec::new()));
            explicit.extend(items.iter().cloned());
            let explicit = normalize_row(&explicit, MathStyle::Display);
            let implicit_classes = noad_classes(&implicit);
            let explicit_classes = noad_classes(&explicit);
            assert_eq!(
                implicit_classes.as_slice(),
                &explicit_classes[1..],
                "{source}"
            );
        }

        let ast = parse("+b").expect("leading binary");
        let MathNode::Row(items) = ast else {
            panic!("expected two noads");
        };
        assert_eq!(
            noad_classes(&normalize_row(&items, MathStyle::Display))[0],
            AtomKind::Ord
        );
        assert_eq!(
            noad_classes(&normalize_row_with_leading_ord(&items, MathStyle::Display))[0],
            AtomKind::Bin
        );
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
    fn over_under_preserves_only_bin_and_relation_classes() {
        assert_eq!(
            classes(r"a\overset{!}{+}b"),
            vec![AtomKind::Ord, AtomKind::Bin, AtomKind::Ord]
        );
        assert_eq!(
            classes(r"a\overset{!}{=}b"),
            vec![AtomKind::Ord, AtomKind::Rel, AtomKind::Ord]
        );
        assert_eq!(
            classes(r"a\overset{!}{x}b"),
            vec![AtomKind::Ord, AtomKind::Op, AtomKind::Ord]
        );
        assert_eq!(
            classes(r"a\xrightarrow{x}b"),
            vec![AtomKind::Ord, AtomKind::Rel, AtomKind::Ord]
        );
        assert_eq!(
            classes(r"a\stackrel{!}{x}b"),
            vec![AtomKind::Ord, AtomKind::Rel, AtomKind::Ord]
        );
        assert_eq!(
            classes(r"a\phantom{=}b"),
            vec![AtomKind::Ord, AtomKind::Ord, AtomKind::Ord]
        );
    }

    #[test]
    fn color_boxes_are_ordinary_while_foreground_color_preserves_class() {
        assert_eq!(
            classes(r"a\textcolor{red}{=}b"),
            vec![AtomKind::Ord, AtomKind::Rel, AtomKind::Ord]
        );
        assert_eq!(
            classes(r"a\colorbox{red}{=}b"),
            vec![AtomKind::Ord, AtomKind::Ord, AtomKind::Ord]
        );
        assert_eq!(
            classes(r"a\fcolorbox{blue}{red}{=}b"),
            vec![AtomKind::Ord, AtomKind::Ord, AtomKind::Ord]
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

    #[test]
    fn explicit_style_declarations_update_the_semantic_row_state() {
        let ast = parse(r"a\scriptstyle b\displaystyle c").unwrap();
        let MathNode::Row(items) = ast else {
            panic!("style declarations must remain in the row");
        };
        let styles = normalize_row(&items, MathStyle::Text)
            .into_iter()
            .filter_map(|item| match item {
                SemanticItem::Style(style) => Some(style),
                _ => None,
            })
            .collect::<Vec<_>>();
        assert_eq!(
            styles,
            vec![MathStyle::Text, MathStyle::Script, MathStyle::Display]
        );
    }

    #[test]
    fn script_semantics_use_tex_sup_and_sub_styles() {
        let ast = parse("x_2^3").unwrap();
        let semantics = script_semantics(&ast, MathStyle::Text).expect("scripted ordinary noad");
        assert_eq!(semantics.style, MathStyle::Text);
        assert_eq!(semantics.sup_style, MathStyle::Script);
        assert_eq!(semantics.sub_style, MathStyle::ScriptCramped);

        let cramped =
            script_semantics(&ast, MathStyle::TextCramped).expect("scripted cramped ordinary noad");
        assert_eq!(cramped.sup_style, MathStyle::ScriptCramped);
        assert_eq!(cramped.sub_style, MathStyle::ScriptCramped);

        let nested =
            script_semantics(&ast, MathStyle::Script).expect("scripted scriptstyle ordinary noad");
        assert_eq!(nested.sup_style, MathStyle::ScriptScript);
        assert_eq!(nested.sub_style, MathStyle::ScriptScriptCramped);
    }

    #[test]
    fn operator_semantics_resolve_default_and_explicit_limit_policy() {
        let display_sum = parse(r"\sum_1^n").unwrap();
        assert_eq!(
            operator_semantics(&display_sum, MathStyle::Display)
                .expect("sum semantics")
                .placement,
            LimitPlacement::Limits
        );
        assert_eq!(
            operator_semantics(&display_sum, MathStyle::Text)
                .expect("inline sum semantics")
                .placement,
            LimitPlacement::Side
        );

        let forced_sum = parse(r"\sum\limits_1^n").unwrap();
        assert_eq!(
            operator_semantics(&forced_sum, MathStyle::Text)
                .expect("forced sum semantics")
                .placement,
            LimitPlacement::Limits
        );

        let side_sum = parse(r"\sum\nolimits_1^n").unwrap();
        assert_eq!(
            operator_semantics(&side_sum, MathStyle::Display)
                .expect("nolimits sum semantics")
                .placement,
            LimitPlacement::Side
        );

        let default_integral = parse(r"\int_0^1").unwrap();
        assert_eq!(
            operator_semantics(&default_integral, MathStyle::Display)
                .expect("integral semantics")
                .placement,
            LimitPlacement::Side
        );

        let forced_integral = parse(r"\int\limits_0^1").unwrap();
        assert_eq!(
            operator_semantics(&forced_integral, MathStyle::Text)
                .expect("forced integral semantics")
                .placement,
            LimitPlacement::Limits
        );
    }
}
