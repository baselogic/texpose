//! Lexical normalization before tokenization.
//!
//! Semantic TeX/LaTeX constructs are parsed structurally. This stage deliberately
//! preserves the original source bytes so later parser diagnostics can refer to
//! source positions without reconstructing a rewritten input.

/// Return the original math source unchanged.
///
/// Semantic rewrites do not belong here: `\dfrac`, `\tfrac`, `\cfrac`,
/// generalized fractions, plain-TeX font switches, and `\mbox` are all handled
/// by the parser.
///
/// # Examples
///
/// ```
/// use texpose::preprocess;
///
/// let source = r"{a \over b} + \tfrac{c}{d}";
/// assert_eq!(preprocess(source), source);
/// ```
#[must_use]
pub fn preprocess(raw_input: &str) -> String {
    raw_input.to_string()
}

#[cfg(test)]
mod tests {
    use super::preprocess;

    #[test]
    fn semantic_source_is_not_rewritten() {
        for source in [
            r"{a \over b}",
            r"{n \choose k}",
            r"\tfrac{a}{b}",
            r"\dfrac{a}{b}",
            r"\cfrac[l]{a}{b}",
            r"{\rm x}",
            r"\mbox{Diagonal}",
        ] {
            assert_eq!(preprocess(source), source);
        }
    }

    #[test]
    fn whitespace_and_comments_are_preserved_for_source_provenance() {
        let source = "  x % comment\n + y  ";
        assert_eq!(preprocess(source), source);
    }
}
