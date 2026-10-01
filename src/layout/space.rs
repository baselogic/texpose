//! TeXbook Table 18 inter-atom spacing (mu). Negative table entries vanish in script style.

use crate::dim::Dim;
use crate::error::NumericError;
use crate::layout::metrics::MathParams;
use crate::layout::style::MathStyle;
use crate::parser::AtomKind;

/// Space in mu: 0 none, 3 thin, 4 medium, 5 thick.
pub fn atom_space_mu(left: AtomKind, right: AtomKind, style: MathStyle) -> i64 {
    let l = idx(left);
    let r = idx(right);
    let raw = TABLE[l][r];
    if raw < 0 {
        if style.script_level() > 0 {
            0
        } else {
            i64::from(-raw)
        }
    } else {
        i64::from(raw)
    }
}

/// Kern width for a Table 18 space at `style`.
pub fn space_width(mu: i64, params: &MathParams, style: MathStyle) -> Result<Dim, NumericError> {
    if mu == 0 {
        return Ok(Dim::zero());
    }
    params.mu(style)?.checked_mul(&Dim::from_i64(mu))
}

fn idx(k: AtomKind) -> usize {
    match k {
        AtomKind::Ord => 0,
        AtomKind::Op => 1,
        AtomKind::Bin => 2,
        AtomKind::Rel => 3,
        AtomKind::Open => 4,
        AtomKind::Close => 5,
        AtomKind::Punct => 6,
        AtomKind::Inner => 7,
    }
}

// TeXbook p. 170. Values 0, 3, 4, 5 mu. Negative = not in script style.
const TABLE: [[i8; 8]; 8] = [
    //        Ord Op Bin Rel Open Close Punct Inner
    /* Ord */
    [0, 3, -4, -5, 0, 0, 0, -3],
    /* Op */ [3, 3, 0, -5, 0, 0, 0, -3],
    /* Bin */ [-4, -4, 0, 0, -4, 0, 0, -4],
    /* Rel */ [-5, -5, 0, 0, 5, 0, 0, -5],
    /* Open */ [0, 0, 0, 0, 0, 0, 0, 0],
    /* Close */ [0, 3, -4, -5, 0, 0, 0, -3],
    /* Punct */ [-3, -3, 0, -3, -3, -3, -3, -3],
    /* Inner */ [-3, 3, -4, -5, -3, 0, -3, -3],
];

#[cfg(test)]
mod tests {
    use super::{atom_space_mu, AtomKind, MathStyle};

    const KINDS: [AtomKind; 8] = [
        AtomKind::Ord,
        AtomKind::Op,
        AtomKind::Bin,
        AtomKind::Rel,
        AtomKind::Open,
        AtomKind::Close,
        AtomKind::Punct,
        AtomKind::Inner,
    ];

    #[test]
    fn table_18_nonzero_text() {
        let want: [[i64; 8]; 8] = [
            [0, 3, 4, 5, 0, 0, 0, 3],
            [3, 3, 0, 5, 0, 0, 0, 3],
            [4, 4, 0, 0, 4, 0, 0, 4],
            [5, 5, 0, 0, 5, 0, 0, 5],
            [0, 0, 0, 0, 0, 0, 0, 0],
            [0, 3, 4, 5, 0, 0, 0, 3],
            [3, 3, 0, 3, 3, 3, 3, 3],
            [3, 3, 4, 5, 3, 0, 3, 3],
        ];
        for (i, l) in KINDS.iter().enumerate() {
            for (j, r) in KINDS.iter().enumerate() {
                assert_eq!(
                    atom_space_mu(*l, *r, MathStyle::Text),
                    want[i][j],
                    "{l:?} {r:?}"
                );
            }
        }
    }

    #[test]
    fn table_18_script_drops_negative() {
        assert_eq!(
            atom_space_mu(AtomKind::Ord, AtomKind::Bin, MathStyle::Script),
            0
        );
        assert_eq!(
            atom_space_mu(AtomKind::Ord, AtomKind::Rel, MathStyle::Script),
            0
        );
        assert_eq!(
            atom_space_mu(AtomKind::Op, AtomKind::Ord, MathStyle::Script),
            3
        );
        assert_eq!(
            atom_space_mu(AtomKind::Open, AtomKind::Bin, MathStyle::Text),
            0
        );
    }

    #[test]
    fn table_18_cramped_variants_match_their_script_level() {
        for left in KINDS {
            for right in KINDS {
                assert_eq!(
                    atom_space_mu(left, right, MathStyle::Display),
                    atom_space_mu(left, right, MathStyle::DisplayCramped)
                );
                assert_eq!(
                    atom_space_mu(left, right, MathStyle::Text),
                    atom_space_mu(left, right, MathStyle::TextCramped)
                );
                assert_eq!(
                    atom_space_mu(left, right, MathStyle::Script),
                    atom_space_mu(left, right, MathStyle::ScriptCramped)
                );
                assert_eq!(
                    atom_space_mu(left, right, MathStyle::ScriptScript),
                    atom_space_mu(left, right, MathStyle::ScriptScriptCramped)
                );
            }
        }
    }
}
