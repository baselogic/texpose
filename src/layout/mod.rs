//! TeX-faithful math box model. Dimensions are [`Dim`](crate::Dim).

mod assembly;
mod engine;
mod metrics;
mod numbering;
mod semantic;
mod space;
mod style;

pub use engine::{
    layout, layout_with_diagnostics, layout_with_em_size_pt,
    layout_with_em_size_pt_and_diagnostics, layout_with_max_depth,
    layout_with_max_depth_and_diagnostics, layout_with_numbering,
    layout_with_numbering_and_diagnostics, layout_with_numbering_and_em_size_pt,
    layout_with_numbering_and_em_size_pt_and_diagnostics,
};
pub use metrics::MathParams;
pub use numbering::{NumberFormat, NumberStyle, NumberingConfig, NumberingState};
pub use style::MathStyle;

use crate::color::Color;
use crate::dim::Dim;
use crate::error::{Error, NumericError};
use crate::font::MathFont;

/// Recoverable issue discovered while producing a mathematical layout.
///
/// Diagnostics do not invalidate the returned box tree. Callers that need to
/// surface graceful-degradation events should use a `*_with_diagnostics` layout
/// entry point.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LayoutDiagnostic {
    /// The selected face had no cmap entry for a required Unicode scalar.
    ///
    /// Layout continues with glyph id 0 when it has a usable horizontal
    /// advance, otherwise with the deterministic missing-glyph fallback box.
    MissingGlyph {
        /// Unicode scalar that could not be resolved through cmap.
        ch: char,
    },
    /// OpenType MATH assembly data could not be used safely for a requested
    /// extension, so layout retained the largest valid ready-made variant.
    ExtensibleFallback {
        /// Unicode scalar whose extensible construction degraded.
        ch: char,
    },
}

/// Box tree plus recoverable diagnostics from one layout operation.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LayoutOutput {
    /// Backend-neutral mathematical box tree.
    pub math_box: MathBox,
    /// Recoverable diagnostics in deterministic traversal order.
    pub diagnostics: Vec<LayoutDiagnostic>,
}

/// Validated physical size of the root math em, expressed in TeX points.
///
/// The value is strictly positive. Finiteness and representable bounds are
/// inherited from [`Dim`], which is always a finite in-range exact rational.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct RootEmSize {
    tex_points: Dim,
}

impl RootEmSize {
    /// Validate a physical root-em size expressed in TeX points.
    ///
    /// # Errors
    ///
    /// Returns [`Error::InvalidOption`] when `tex_points` is zero or negative.
    pub(crate) fn new(tex_points: Dim) -> Result<Self, Error> {
        if tex_points <= Dim::zero() {
            return Err(Error::InvalidOption {
                what: "root em size must be positive TeX pt".into(),
            });
        }
        Ok(Self { tex_points })
    }

    /// Exact physical size in TeX points.
    #[must_use]
    pub(crate) fn tex_points(&self) -> &Dim {
        &self.tex_points
    }
}

/// What a box contains.
///
/// Glyphs carry an OpenType id from the math face. Lists compose children.
/// Color wrappers do not change dimensions. [`BoxContent::Line`] and
/// [`BoxContent::Frame`] are decorations from cancel / boxed constructs.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum BoxContent {
    /// Empty box (strut or placeholder).
    Empty,
    /// Solid rule (fraction bar, vinculum). No glyph.
    Rule,
    /// A single character whose metrics came from the math font.
    ///
    /// Build with [`BoxContent::glyph`]; the variant is non-exhaustive.
    #[non_exhaustive]
    Glyph {
        /// Character.
        ch: char,
        /// OpenType glyph id.
        glyph_id: u16,
        /// Outline scale relative to the em: 1 at text size, smaller at script
        /// and scriptscript size. Box dimensions are already scaled; consumers
        /// apply this factor when placing the glyph outline.
        scale: Dim,
    },
    /// Horizontal list. Width is the sum of children.
    HList(Vec<MathBox>),
    /// Vertical list. Height/depth stack on the baseline of the first box.
    VList(Vec<MathBox>),
    /// Horizontal kern (atom spacing, `\hspace`).
    Kern(Dim),
    /// Foreground-color wrapper. Dimensions match the inner box.
    Color(Color, Box<MathBox>),
    /// Background color (`\colorbox`). Inner glyphs keep the default fill.
    BackColor(Color, Box<MathBox>),
    /// Children share the left edge; each child's [`MathBox::shift`] is its baseline.
    /// Child order is paint order.
    Overlap(Vec<MathBox>),
    /// Diagonal or free line in em, relative to the box left and baseline (`y` up).
    Line {
        /// Start x (em from left).
        x1: Dim,
        /// Start y (em above baseline).
        y1: Dim,
        /// End x.
        x2: Dim,
        /// End y.
        y2: Dim,
        /// Stroke thickness (em).
        thickness: Dim,
    },
    /// Stroked rectangle around the inner box. Inner is laid out at the same origin.
    Frame {
        /// Rule thickness (em).
        thickness: Dim,
        /// Border color. `None` inherits the current fill (`\boxed`).
        stroke: Option<Color>,
        /// Contents inside the frame.
        inner: Box<MathBox>,
    },
}

/// TeX-style box: width, height above baseline, depth below, italic correction.
///
/// # Examples
///
/// ```
/// use texpose::{Dim, MathBox};
///
/// let packed = MathBox::hpack(vec![
///     MathBox::rule(Dim::one(), Dim::zero(), Dim::zero()),
///     MathBox::rule(Dim::ratio(1, 2).unwrap(), Dim::zero(), Dim::zero()),
/// ]).unwrap();
/// assert_eq!(packed.width, Dim::ratio(3, 2).unwrap());
/// ```
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MathBox {
    /// Width.
    pub width: Dim,
    /// Height above the baseline.
    pub height: Dim,
    /// Depth below the baseline.
    pub depth: Dim,
    /// Italic correction.
    pub italic: Dim,
    /// Baseline raise relative to the parent list (positive is up).
    pub shift: Dim,
    /// Payload.
    pub content: BoxContent,
}

impl BoxContent {
    /// Glyph content drawn at `scale` times the em (1 for text size).
    #[must_use]
    pub fn glyph(ch: char, glyph_id: u16, scale: Dim) -> Self {
        Self::Glyph {
            ch,
            glyph_id,
            scale,
        }
    }
}

impl MathBox {
    /// Zero-size empty box.
    #[must_use]
    pub fn empty() -> Self {
        Self {
            width: Dim::zero(),
            height: Dim::zero(),
            depth: Dim::zero(),
            italic: Dim::zero(),
            shift: Dim::zero(),
            content: BoxContent::Empty,
        }
    }

    /// Rule with explicit dimensions (fraction bar, strut).
    #[must_use]
    pub fn rule(width: Dim, height: Dim, depth: Dim) -> Self {
        Self {
            width,
            height,
            depth,
            italic: Dim::zero(),
            shift: Dim::zero(),
            content: BoxContent::Rule,
        }
    }

    /// Horizontal kern of `width` (zero height and depth).
    #[must_use]
    pub fn kern(width: Dim) -> Self {
        Self {
            width: width.clone(),
            height: Dim::zero(),
            depth: Dim::zero(),
            italic: Dim::zero(),
            shift: Dim::zero(),
            content: BoxContent::Kern(width),
        }
    }

    /// Box from a font glyph. Errors if the face has no glyph for `ch`.
    pub fn from_glyph(font: &MathFont, ch: char) -> Result<Self, Error> {
        let g = font.glyph(ch)?;
        Ok(Self {
            width: g.advance,
            height: g.height,
            depth: g.depth,
            italic: font.italic_correction(g.glyph_id),
            shift: Dim::zero(),
            content: BoxContent::glyph(ch, g.glyph_id, Dim::one()),
        })
    }

    /// Pack boxes in a row. Width sums; height and depth are maxima after
    /// applying each child baseline shift.
    ///
    /// # Errors
    ///
    /// Returns [`NumericError::ArithmeticOverflow`] if the exact packed width
    /// is outside the supported [`Dim`] range.
    pub fn hpack(children: Vec<Self>) -> Result<Self, NumericError> {
        let mut width = Dim::zero();
        let mut height = Dim::zero();
        let mut depth = Dim::zero();
        for c in &children {
            width = width.checked_add(&c.width)?;
            height = height.max_ref(&c.height.checked_add(&c.shift)?.clamp_nonneg());
            depth = depth.max_ref(&c.depth.checked_sub(&c.shift)?.clamp_nonneg());
        }
        Ok(Self {
            width,
            height,
            depth,
            italic: Dim::zero(),
            shift: Dim::zero(),
            content: BoxContent::HList(children),
        })
    }

    /// Pack boxes in a column, first child on the baseline.
    ///
    /// Subsequent children sit below the previous (height + depth stacked).
    ///
    /// # Errors
    ///
    /// Returns [`NumericError::ArithmeticOverflow`] if the exact stacked depth
    /// is outside the supported [`Dim`] range.
    pub fn vpack(children: Vec<Self>) -> Result<Self, NumericError> {
        if children.is_empty() {
            return Ok(Self::empty());
        }
        let mut width = Dim::zero();
        let height = children[0].height.clone();
        let mut depth = children[0].depth.clone();
        for c in children.iter().skip(1) {
            width = width.max_ref(&c.width);
            depth = depth.checked_add(&c.height)?;
            depth = depth.checked_add(&c.depth)?;
        }
        width = width.max_ref(&children[0].width);
        Ok(Self {
            width,
            height,
            depth,
            italic: Dim::zero(),
            shift: Dim::zero(),
            content: BoxContent::VList(children),
        })
    }

    /// Raise this box's baseline by `shift` (positive is up).
    #[must_use]
    pub fn with_shift(mut self, shift: Dim) -> Self {
        self.shift = shift;
        self
    }

    /// Gold-stable width/height/depth decimal string.
    #[must_use]
    pub fn dim_gold(&self) -> String {
        format!(
            "w={} h={} d={}",
            self.width.to_dec_string(),
            self.height.to_dec_string(),
            self.depth.to_dec_string()
        )
    }
}

#[cfg(test)]
mod tests {
    use super::MathBox;
    use crate::{Dim, NumericError};

    #[test]
    fn hpack_accounts_for_child_baseline_shift() {
        let raised =
            MathBox::rule(Dim::one(), Dim::from_i64(2), Dim::from_i64(3)).with_shift(Dim::one());
        let lowered = MathBox::rule(Dim::one(), Dim::from_i64(4), Dim::from_i64(5))
            .with_shift(-Dim::from_i64(2));
        let packed = MathBox::hpack(vec![raised, lowered]).unwrap();

        assert_eq!(packed.width, Dim::from_i64(2));
        assert_eq!(packed.height, Dim::from_i64(3));
        assert_eq!(packed.depth, Dim::from_i64(7));
    }

    #[test]
    fn hpack_propagates_exact_width_overflow() {
        let max = Dim::parse("170141183460469231731687303715884105727").unwrap();
        let child = MathBox::rule(max, Dim::zero(), Dim::zero());
        assert_eq!(
            MathBox::hpack(vec![child.clone(), child]),
            Err(NumericError::ArithmeticOverflow)
        );
    }
}
