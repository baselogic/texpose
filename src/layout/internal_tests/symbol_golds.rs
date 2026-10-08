//! Gold runner: `golds/symbols.toml` plus a catalog corpus.

use super::{common, gold_support};

use crate::test_support::{
    layout, layout_with_diagnostics, parse, styled_char, symbol_atom_kind, symbols, AtomKind,
    BoxContent, LayoutDiagnostic, MathBox, MathFont, MathNode, MathStyle, SymbolKind, TextStyle,
};

struct Rec {
    name: String,
    kind: String,
    input: String,
    expect: String,
    class: String,
    lhs: String,
}

fn load_golds() -> Vec<Rec> {
    gold_support::load("golds/symbols.toml")
        .into_iter()
        .map(|gold| {
            match gold.kind() {
                "glyph" => {
                    gold.expect_fields(&["name", "kind", "input", "expect"], &["class"]);
                }
                "not" => {
                    gold.expect_fields(&["name", "kind", "input", "expect", "class"], &[]);
                }
                "wider" => gold.expect_fields(&["name", "kind", "input", "lhs"], &[]),
                "missing_glyph" => gold.expect_fields(&["name", "kind", "input"], &[]),
                "err_parse" => {
                    gold.expect_fields(&["name", "kind", "input", "expect"], &[]);
                }
                other => panic!("{}: unknown kind {other}", gold.name()),
            }
            Rec {
                name: gold.name().to_string(),
                kind: gold.kind().to_string(),
                input: gold.required("input").to_string(),
                expect: gold.optional("expect").unwrap_or("").to_string(),
                class: gold.optional("class").unwrap_or("").to_string(),
                lhs: gold.optional("lhs").unwrap_or("").to_string(),
            }
        })
        .collect()
}

fn glyphs(b: &MathBox) -> Vec<char> {
    match &b.content {
        BoxContent::Glyph { ch, .. } => vec![*ch],
        BoxContent::HList(v) | BoxContent::VList(v) | BoxContent::Overlap(v) => {
            v.iter().flat_map(glyphs).collect()
        }
        BoxContent::Color(_, inner)
        | BoxContent::BackColor(_, inner)
        | BoxContent::Frame { inner, .. }
        | BoxContent::PaintCopies { inner, .. } => glyphs(inner),
        _ => vec![],
    }
}

fn class_name(k: AtomKind) -> &'static str {
    match k {
        AtomKind::Ord => "Ord",
        AtomKind::Op => "Op",
        AtomKind::Bin => "Bin",
        AtomKind::Rel => "Rel",
        AtomKind::Open => "Open",
        AtomKind::Close => "Close",
        AtomKind::Punct => "Punct",
        AtomKind::Inner => "Inner",
    }
}

fn parsed_class(ast: &MathNode) -> Option<AtomKind> {
    match ast {
        MathNode::Atom(_, kind) => Some(*kind),
        MathNode::Symbol(name) => Some(symbol_atom_kind(name)),
        MathNode::MathAlphabet(_, _) => Some(AtomKind::Ord),
        MathNode::Operator(_, _) => Some(AtomKind::Op),
        _ => None,
    }
}

fn lay(font: &MathFont, input: &str) -> MathBox {
    let ast = parse(input).unwrap_or_else(|e| panic!("parse {input}: {e}"));
    layout(&ast, font, MathStyle::Text).unwrap_or_else(|e| panic!("layout {input}: {e}"))
}

fn is_bare_latex(latex: &str) -> bool {
    let t = latex.trim();
    if !t.starts_with('\\') {
        return t.chars().count() == 1;
    }
    let rest = &t[1..];
    rest.chars().all(|c| c.is_ascii_alphabetic())
        || (rest.chars().count() == 1 && !rest.chars().next().unwrap().is_ascii_alphabetic())
}

#[test]
fn symbol_golds() {
    let recs = load_golds();
    assert!(!recs.is_empty(), "no symbol golds loaded");
    let font = common::stix_two_math().expect("STIX Two Math");
    for rec in recs {
        match rec.kind.as_str() {
            "glyph" => {
                let bx = lay(&font, &rec.input);
                let gs = glyphs(&bx);
                let expected: Vec<char> = rec.expect.chars().collect();
                assert_eq!(gs, expected, "{}: exact glyph sequence", rec.name);
                if !rec.class.is_empty() {
                    let ast = parse(&rec.input).unwrap_or_else(|e| panic!("{}: {e}", rec.name));
                    let class = parsed_class(&ast).unwrap_or_else(|| {
                        panic!(
                            "{}: class field is not supported for AST {}",
                            rec.name,
                            ast.gold()
                        )
                    });
                    assert_eq!(class_name(class), rec.class, "{}", rec.name);
                }
            }
            "not" => {
                let bx = lay(&font, &rec.input);
                let expected: Vec<char> = rec.expect.chars().collect();
                assert_eq!(
                    expected.len(),
                    2,
                    "{}: \\not fixture must name exactly two glyphs",
                    rec.name
                );
                assert_eq!(
                    glyphs(&bx),
                    expected,
                    "{}: exact \\not glyph sequence",
                    rec.name
                );

                let layers = match &bx.content {
                    BoxContent::Overlap(layers) => layers,
                    other => panic!("{}: \\not must be an overlap, got {other:?}", rec.name),
                };
                assert_eq!(layers.len(), 2, "{}: \\not overlap layer count", rec.name);
                assert_eq!(
                    glyphs(&layers[0]),
                    vec![expected[0]],
                    "{}: \\not base layer",
                    rec.name
                );
                assert_eq!(
                    glyphs(&layers[1]),
                    vec![expected[1]],
                    "{}: \\not slash layer",
                    rec.name
                );

                let base_input = rec
                    .input
                    .strip_prefix("\\not")
                    .filter(|rest| !rest.is_empty())
                    .unwrap_or_else(|| panic!("{}: invalid \\not fixture input", rec.name));
                let base_ast =
                    parse(base_input).unwrap_or_else(|e| panic!("{} base: {e}", rec.name));
                let base_class = parsed_class(&base_ast).unwrap_or_else(|| {
                    panic!(
                        "{}: class field is not supported for base AST {}",
                        rec.name,
                        base_ast.gold()
                    )
                });
                assert_eq!(class_name(base_class), rec.class, "{} base class", rec.name);

                let base = lay(&font, base_input);
                let negated_row = lay(&font, &format!("a {} b", rec.input));
                let base_row = lay(&font, &format!("a {base_input} b"));
                let negated_plus_base = negated_row
                    .width
                    .checked_add(&base.width)
                    .unwrap_or_else(|e| panic!("{}: width sum failed: {e}", rec.name));
                let base_plus_negated = base_row
                    .width
                    .checked_add(&bx.width)
                    .unwrap_or_else(|e| panic!("{}: width sum failed: {e}", rec.name));
                assert_eq!(
                    negated_plus_base, base_plus_negated,
                    "{}: \\not must preserve the base atom's surrounding spacing",
                    rec.name
                );
            }
            "wider" => {
                let a = lay(&font, &rec.lhs);
                let b = lay(&font, &rec.input);
                assert!(
                    b.width.cmp(&a.width) == std::cmp::Ordering::Greater,
                    "{}: {} width {} not greater than {} width {}",
                    rec.name,
                    rec.input,
                    b.width,
                    rec.lhs,
                    a.width
                );
            }
            "missing_glyph" => {
                let ch = rec.input.chars().next().expect("char");
                let ast = MathNode::Atom(ch, AtomKind::Ord);
                let output = layout_with_diagnostics(&ast, &font, MathStyle::Text)
                    .unwrap_or_else(|err| panic!("{}: {err}", rec.name));
                assert_eq!(
                    output.diagnostics,
                    vec![LayoutDiagnostic::MissingGlyph { ch }],
                    "{}",
                    rec.name
                );
            }
            "err_parse" => {
                let err = parse(&rec.input).expect_err(&rec.name);
                assert!(
                    matches!(err.kind(), texpose::ParseErrorKind::UnsupportedCommand),
                    "{}: {err}",
                    rec.name
                );
                assert_eq!(err.to_string(), rec.expect, "{}", rec.name);
            }
            other => panic!("{}: unknown kind {other}", rec.name),
        }
    }
}

#[test]
fn catalog_single_glyphs_layout() {
    let font = common::stix_two_math().expect("STIX Two Math");
    let mut failed = Vec::new();
    for e in symbols() {
        if !matches!(e.kind, SymbolKind::Symbol | SymbolKind::Operator) {
            continue;
        }
        if e.glyph.chars().count() != 1 {
            continue;
        }
        if !is_bare_latex(e.latex) {
            continue;
        }
        let name = e.command_name();
        if matches!(
            name,
            "mathbb" | "mathcal" | "mathfrak" | "text" | "mathring" | "lim"
        ) {
            continue;
        }
        let input = if e.latex.starts_with('\\') {
            format!("\\{name}")
        } else {
            e.latex.to_string()
        };
        let ast = match parse(&input) {
            Ok(a) => a,
            Err(err) => {
                failed.push(format!("{} parse {err}", e.latex));
                continue;
            }
        };
        let bx = match layout(&ast, &font, MathStyle::Text) {
            Ok(b) => b,
            Err(err) => {
                failed.push(format!("{} layout {err}", e.latex));
                continue;
            }
        };
        let gs = glyphs(&bx);
        let ch = math_italic(e.glyph.chars().next().unwrap());
        if !gs.contains(&ch) {
            failed.push(format!(
                "{} glyph {ch} not in {gs:?} (ast {})",
                e.latex,
                ast.gold()
            ));
        }
    }
    assert!(
        failed.is_empty(),
        "catalog glyph golds failed:\n{}",
        failed.join("\n")
    );
}

#[test]
fn font_style_letter_classes() {
    let font = common::stix_two_math().expect("STIX Two Math");
    let styles: &[(TextStyle, &str)] = &[
        (TextStyle::Rm, "mathrm"),
        (TextStyle::Bf, "mathbf"),
        (TextStyle::It, "mathit"),
        (TextStyle::Sf, "mathsf"),
        (TextStyle::Tt, "mathtt"),
        (TextStyle::Bb, "mathbb"),
        (TextStyle::Cal, "mathcal"),
        (TextStyle::Frak, "mathfrak"),
        (TextStyle::Scr, "mathscr"),
    ];
    let mut failed = Vec::new();
    for (ts, cmd) in styles {
        let letters: Vec<char> = match *ts {
            TextStyle::Bb | TextStyle::Cal | TextStyle::Scr => ('A'..='Z').collect(),
            TextStyle::Frak => ('A'..='Z').chain('a'..='z').collect(),
            TextStyle::It => ('A'..='Z').chain('a'..='z').collect(),
            _ => ('A'..='Z').chain('a'..='z').chain('0'..='9').collect(),
        };
        for c in letters {
            let input = format!("\\{cmd}{{{c}}}");
            let want = styled_char(c, *ts);
            let ast = match parse(&input) {
                Ok(a) => a,
                Err(err) => {
                    failed.push(format!("{input} parse {err}"));
                    continue;
                }
            };
            match layout(&ast, &font, MathStyle::Text) {
                Ok(bx) => {
                    let gs = glyphs(&bx);
                    if !gs.contains(&want) {
                        failed.push(format!("{input} want {want} got {gs:?}"));
                    }
                }
                Err(err) => failed.push(format!("{input} layout {err}")),
            }
        }
    }
    assert!(
        failed.is_empty(),
        "font-style letter golds failed:\n{}",
        failed.join("\n")
    );
}

/// Plain math draws lowercase Greek and `\partial` from the Mathematical Italic block.
fn math_italic(c: char) -> char {
    let lower_greek = ('\u{03B1}'..='\u{03C9}').contains(&c);
    if lower_greek || matches!(c, 'ϵ' | 'ϑ' | 'ϰ' | 'ϕ' | 'ϱ' | 'ϖ' | '∂') {
        styled_char(c, TextStyle::It)
    } else {
        c
    }
}
