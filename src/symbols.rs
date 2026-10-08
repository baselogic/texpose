//! Math-mode symbol catalog from `data/symbols.tsv`.
//!
//! Source tables live in `documents/`. Flutter UI columns are not loaded.
//! Duplicate `\sqrt{}` rows were collapsed to one entry.

use std::collections::HashMap;
use std::sync::OnceLock;

const TSV: &str = include_str!("../data/symbols.tsv");

/// How a catalog entry is used on a math keyboard / in the parser.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SymbolKind {
    /// Single glyph (`\alpha`, `\times`).
    Symbol,
    /// Operator (`\sum`, `\int`, `+`).
    Operator,
    /// Structure that takes a body (`\frac`, `\sqrt`, matrices).
    Container,
    /// Accent or modifier (`\hat`, `\vec`).
    Modifier,
}

impl SymbolKind {
    fn parse(s: &str) -> Option<Self> {
        match s {
            "Symbol" => Some(Self::Symbol),
            "Operator" => Some(Self::Operator),
            "Container" => Some(Self::Container),
            "Modifier" => Some(Self::Modifier),
            _ => None,
        }
    }

    /// Catalog spelling.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Symbol => "Symbol",
            Self::Operator => "Operator",
            Self::Container => "Container",
            Self::Modifier => "Modifier",
        }
    }
}

/// One row of the shipped symbol table.
///
/// # Examples
///
/// ```
/// use texpose::lookup;
///
/// let e = lookup(r"\alpha").unwrap();
/// assert_eq!(e.glyph, "α");
/// assert_eq!(e.command_name(), "alpha");
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SymbolEntry {
    /// Typical mathematical character (may be a placeholder for containers).
    pub glyph: &'static str,
    /// High-level group (`Greek`, `Calculus`, …).
    pub category: &'static str,
    /// Keyboard / parser kind.
    pub kind: SymbolKind,
    /// Raw LaTeX from the table (`\alpha`, `\frac{}{}`, `+`).
    pub latex: &'static str,
    /// Human description from the table.
    pub description: &'static str,
}

impl SymbolEntry {
    /// Control-sequence name without `\`, or the raw character for `+` / `=`.
    #[must_use]
    pub fn command_name(&self) -> &str {
        command_name(self.latex)
    }
}

fn command_name(latex: &str) -> &str {
    let t = latex.trim();
    if let Some(rest) = t.strip_prefix('\\') {
        let n = rest.chars().take_while(|c| c.is_ascii_alphabetic()).count();
        if n > 0 {
            return &rest[..n];
        }
        if let Some(c) = rest.chars().next() {
            let end = c.len_utf8();
            return &rest[..end];
        }
    }
    t
}

fn parse_tsv(text: &'static str) -> Vec<SymbolEntry> {
    let mut rows = Vec::new();
    let mut lines = text.lines();
    let header = lines.next().expect("symbols.tsv header");
    assert_eq!(
        header, "glyph\tcategory\tkind\tlatex\tdescription",
        "symbols.tsv schema"
    );
    for line in lines {
        if line.is_empty() {
            continue;
        }
        let mut cols = line.split('\t');
        let glyph = cols.next().expect("glyph");
        let category = cols.next().expect("category");
        let kind_s = cols.next().expect("kind");
        let latex = cols.next().expect("latex");
        let description = cols.next().unwrap_or("");
        let kind = SymbolKind::parse(kind_s).expect("symbol kind");
        rows.push(SymbolEntry {
            glyph,
            category,
            kind,
            latex,
            description,
        });
    }
    rows
}

fn catalog() -> &'static [SymbolEntry] {
    static CAT: OnceLock<Vec<SymbolEntry>> = OnceLock::new();
    CAT.get_or_init(|| parse_tsv(TSV)).as_slice()
}

/// Preserve table-order precedence for non-unique command names.
struct CommandMatches {
    first: usize,
    bare: Option<usize>,
}

struct LookupIndex {
    exact: HashMap<&'static str, usize>,
    commands: HashMap<&'static str, CommandMatches>,
}

fn lookup_index() -> &'static LookupIndex {
    static INDEX: OnceLock<LookupIndex> = OnceLock::new();
    INDEX.get_or_init(|| {
        let mut exact = HashMap::new();
        let mut commands = HashMap::new();
        for (position, entry) in catalog().iter().enumerate() {
            exact.entry(entry.latex).or_insert(position);
            let name = entry.command_name();
            let matches = commands.entry(name).or_insert(CommandMatches {
                first: position,
                bare: None,
            });
            if matches.bare.is_none() && is_bare_latex(entry.latex, name) {
                matches.bare = Some(position);
            }
        }
        LookupIndex { exact, commands }
    })
}

/// All shipped symbols, in table order.
///
/// # Examples
///
/// ```
/// use texpose::symbols;
///
/// assert!(!symbols().is_empty());
/// ```
#[must_use]
pub fn symbols() -> &'static [SymbolEntry] {
    catalog()
}

/// Look up by raw table LaTeX (`\alpha`, `\frac{}{}`) or command name (`alpha`).
///
/// Bare commands (`\aleph`) win over composite rows (`\aleph_0`).
///
/// # Arguments
///
/// * `query` — control sequence with or without `\`, or a table `latex` cell.
///
/// # Returns
///
/// The catalog row, or `None` if `query` is not in the table.
///
/// # Examples
///
/// ```
/// use texpose::lookup;
///
/// assert_eq!(lookup(r"\alpha").unwrap().glyph, "α");
/// assert!(lookup(r"\notacommand").is_none());
/// ```
#[must_use]
pub fn lookup(query: &str) -> Option<&'static SymbolEntry> {
    let q = query.trim();
    let q_name = command_name(q);
    let index = lookup_index();
    let by_name = index.commands.get(q_name);
    let position = index
        .exact
        .get(q)
        .copied()
        .or_else(|| by_name.and_then(|entry| entry.bare))
        .or_else(|| {
            // The previous linear search matched either name in table order.
            // The earliest position wins even when both keys are present.
            index
                .commands
                .get(q)
                .map(|entry| entry.first)
                .into_iter()
                .chain(by_name.map(|entry| entry.first))
                .min()
        })?;
    catalog().get(position)
}

/// Single-character catalog glyph for `query`, if the row is a lone code point.
///
/// # Examples
///
/// ```
/// use texpose::glyph_char;
///
/// assert_eq!(glyph_char(r"\alpha"), Some('α'));
/// ```
#[must_use]
pub fn glyph_char(query: &str) -> Option<char> {
    let e = lookup(query)?;
    let mut chars = e.glyph.chars();
    match (chars.next(), chars.next()) {
        (Some(c), None) => Some(c),
        _ => None,
    }
}

fn is_bare_latex(latex: &str, name: &str) -> bool {
    let t = latex.trim();
    t == name || t.strip_prefix('\\').is_some_and(|rest| rest == name)
}

/// Number of entries in a category.
///
/// # Examples
///
/// ```
/// use texpose::category_count;
///
/// assert!(category_count("Greek") > 0);
/// ```
#[must_use]
pub fn category_count(category: &str) -> usize {
    catalog().iter().filter(|e| e.category == category).count()
}

#[cfg(test)]
mod tests {
    use super::{catalog, command_name, is_bare_latex, lookup, symbols, SymbolEntry};

    // Independent reference for the old table-order selection policy.
    fn linear_lookup(query: &str) -> Option<&'static SymbolEntry> {
        let q = query.trim();
        let name = command_name(q);
        catalog()
            .iter()
            .find(|entry| entry.latex == q)
            .or_else(|| {
                catalog()
                    .iter()
                    .find(|entry| entry.command_name() == name && is_bare_latex(entry.latex, name))
            })
            .or_else(|| {
                catalog()
                    .iter()
                    .find(|entry| entry.command_name() == q || entry.command_name() == name)
            })
    }

    #[test]
    fn index_preserves_exact_bare_and_alias_precedence_for_every_catalog_entry() {
        for entry in symbols() {
            for query in [
                entry.latex.to_string(),
                entry.command_name().to_string(),
                format!(r"\{}", entry.command_name()),
                format!(" \t{} \n", entry.latex),
            ] {
                let actual = lookup(&query);
                let expected = linear_lookup(&query);
                assert!(
                    match (actual, expected) {
                        (Some(actual), Some(expected)) => std::ptr::eq(actual, expected),
                        (None, None) => true,
                        _ => false,
                    },
                    "lookup chose a different catalog row for {query:?}"
                );
            }
        }
        for query in ["", " \t ", r"\not_a_symbol", "not_a_symbol"] {
            assert_eq!(lookup(query), linear_lookup(query), "{query:?}");
        }
    }
}
