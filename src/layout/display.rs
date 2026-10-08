//! Public backend-neutral display-list boundary.

use crate::color::Color;
use crate::dim::Dim;
use crate::error::{Error, NumericError};
use crate::font::MathFont;
use crate::layout::engine;
use crate::layout::numbering::NumberingState;
use crate::layout::style::MathStyle;
use crate::layout::{BoxContent, LayoutDiagnostic, LayoutOutput, MathBox};
use crate::parser::MathNode;

// Bound nested paint repetition independently of AST limits.
const MAX_PMB_PAINT_STEPS: usize = 65_536;

/// Flat backend-neutral mathematical display list.
///
/// Coordinates are absolute root-em units. The origin is the formula left edge
/// on its root baseline, `+x` points right, and `+y` points up. Width, height,
/// and depth are positive extents from that origin. [`Self::ops`] is paint
/// order.
#[derive(Clone)]
pub struct MathLayout {
    font: MathFont,
    width: f32,
    height: f32,
    depth: f32,
    ops: Vec<MathOp>,
    diagnostics: Vec<LayoutDiagnostic>,
}

impl MathLayout {
    /// Exact font handle used to resolve every glyph id in this layout.
    #[must_use]
    pub fn font(&self) -> &MathFont {
        &self.font
    }

    /// Formula width in root-em units.
    #[must_use]
    pub fn width(&self) -> f32 {
        self.width
    }

    /// Positive extent above the root baseline in root-em units.
    #[must_use]
    pub fn height(&self) -> f32 {
        self.height
    }

    /// Positive extent below the root baseline in root-em units.
    #[must_use]
    pub fn depth(&self) -> f32 {
        self.depth
    }

    /// Positioned drawing operations in paint order.
    #[must_use]
    pub fn ops(&self) -> &[MathOp] {
        &self.ops
    }

    /// Recoverable layout diagnostics in deterministic traversal order.
    #[must_use]
    pub fn diagnostics(&self) -> &[LayoutDiagnostic] {
        &self.diagnostics
    }
}

/// One absolute drawing operation in a [`MathLayout`].
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub enum MathOp {
    /// Draw one glyph from [`MathLayout::font`].
    #[non_exhaustive]
    Glyph {
        /// OpenType glyph id in the retained face.
        glyph_id: u16,
        /// Glyph origin x coordinate in root-em units.
        x: f32,
        /// Glyph baseline y coordinate in root-em units.
        baseline: f32,
        /// Outline scale relative to the root em.
        scale: f32,
        /// Explicit foreground color, or `None` to inherit the consumer's default foreground.
        color: Option<Color>,
    },
    /// Fill an axis-aligned rule rectangle.
    #[non_exhaustive]
    Rule {
        /// Left edge in root-em units.
        x: f32,
        /// Bottom edge in root-em units.
        y: f32,
        /// Positive width in root-em units.
        width: f32,
        /// Positive height in root-em units.
        height: f32,
        /// Explicit fill color, or `None` to inherit the consumer's default foreground.
        color: Option<Color>,
    },
    /// Stroke one free line segment.
    #[non_exhaustive]
    Line {
        /// Start x coordinate in root-em units.
        x1: f32,
        /// Start y coordinate in root-em units.
        y1: f32,
        /// End x coordinate in root-em units.
        x2: f32,
        /// End y coordinate in root-em units.
        y2: f32,
        /// Positive stroke thickness in root-em units.
        thickness: f32,
        /// Explicit stroke color, or `None` to inherit the consumer's default foreground.
        color: Option<Color>,
    },
    /// Paint an inward border on an axis-aligned outer rectangle.
    ///
    /// The border occupies `thickness` inside all four edges and therefore
    /// does not expand the logical box.
    #[non_exhaustive]
    Frame {
        /// Left edge in root-em units.
        x: f32,
        /// Bottom edge in root-em units.
        y: f32,
        /// Positive width in root-em units.
        width: f32,
        /// Positive height in root-em units.
        height: f32,
        /// Positive stroke thickness in root-em units.
        thickness: f32,
        /// Explicit stroke color, or `None` to inherit the consumer's default foreground.
        color: Option<Color>,
    },
    /// Fill a background rectangle.
    #[non_exhaustive]
    Background {
        /// Left edge in root-em units.
        x: f32,
        /// Bottom edge in root-em units.
        y: f32,
        /// Positive width in root-em units.
        width: f32,
        /// Positive height in root-em units.
        height: f32,
        /// Explicit fill color, or `None` to inherit the consumer's default foreground.
        color: Option<Color>,
    },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum ExactMathOp {
    Glyph {
        glyph_id: u16,
        x: Dim,
        baseline: Dim,
        scale: Dim,
        color: Option<Color>,
    },
    Rule {
        x: Dim,
        y: Dim,
        width: Dim,
        height: Dim,
        color: Option<Color>,
    },
    Line {
        x1: Dim,
        y1: Dim,
        x2: Dim,
        y2: Dim,
        thickness: Dim,
        color: Option<Color>,
    },
    Frame {
        x: Dim,
        y: Dim,
        width: Dim,
        height: Dim,
        thickness: Dim,
        color: Option<Color>,
    },
    Background {
        x: Dim,
        y: Dim,
        width: Dim,
        height: Dim,
        color: Option<Color>,
    },
}

/// Lay out one math tree as a flat backend-neutral display list.
///
/// Recoverable degradation is retained in [`MathLayout::diagnostics`].
///
/// # Errors
///
/// Returns the same parser-independent font, unsupported-construct, malformed,
/// numeric, and option errors as the exact layout engine. Exact geometry is
/// converted to finite `f32` only after positioning is complete.
pub fn layout(node: &MathNode, font: &MathFont, style: MathStyle) -> Result<MathLayout, Error> {
    finish(font, engine::layout_with_diagnostics(node, font, style)?)
}

/// Lay out with an explicit physical root-em size in TeX points.
///
/// # Errors
///
/// Same as [`layout`], plus [`Error::InvalidOption`] for a non-positive root em.
pub fn layout_with_em_size_pt(
    node: &MathNode,
    font: &MathFont,
    style: MathStyle,
    em_size_pt: &Dim,
) -> Result<MathLayout, Error> {
    finish(
        font,
        engine::layout_with_em_size_pt_and_diagnostics(node, font, style, em_size_pt)?,
    )
}

/// Lay out with caller-owned equation numbering state.
///
/// # Errors
///
/// Same as [`layout`], plus numbering-configuration failures.
pub fn layout_with_numbering(
    node: &MathNode,
    font: &MathFont,
    style: MathStyle,
    state: &mut NumberingState,
) -> Result<MathLayout, Error> {
    finish(
        font,
        engine::layout_with_numbering_and_diagnostics(node, font, style, state)?,
    )
}

/// Lay out with caller-owned numbering and an explicit physical root-em size.
///
/// # Errors
///
/// Same as [`layout_with_em_size_pt`] plus numbering-configuration failures.
pub fn layout_with_numbering_and_em_size_pt(
    node: &MathNode,
    font: &MathFont,
    style: MathStyle,
    state: &mut NumberingState,
    em_size_pt: &Dim,
) -> Result<MathLayout, Error> {
    finish(
        font,
        engine::layout_with_numbering_and_em_size_pt_and_diagnostics(
            node, font, style, state, em_size_pt,
        )?,
    )
}

/// Lay out with an explicit maximum syntax-tree nesting depth.
///
/// # Errors
///
/// Same as [`layout`]. Trees deeper than `max_depth` return
/// [`Error::Unsupported`].
pub fn layout_with_max_depth(
    node: &MathNode,
    font: &MathFont,
    style: MathStyle,
    max_depth: usize,
) -> Result<MathLayout, Error> {
    finish(
        font,
        engine::layout_with_max_depth_and_diagnostics(node, font, style, max_depth)?,
    )
}

fn finish(font: &MathFont, exact: LayoutOutput) -> Result<MathLayout, Error> {
    let width = dim_to_f32(&exact.math_box.width)?;
    let height = dim_to_f32(&exact.math_box.height)?;
    let depth = dim_to_f32(&exact.math_box.depth)?;
    let exact_ops = flatten_exact(&exact.math_box)?;
    let mut ops = Vec::with_capacity(exact_ops.len());
    for op in exact_ops {
        ops.push(convert_op(op)?);
    }
    Ok(MathLayout {
        font: font.clone(),
        width,
        height,
        depth,
        ops,
        diagnostics: exact.diagnostics,
    })
}

pub(crate) fn flatten_exact(math_box: &MathBox) -> Result<Vec<ExactMathOp>, NumericError> {
    let mut out = Vec::new();
    let mut paint_budget = MAX_PMB_PAINT_STEPS;
    flatten_into(
        math_box,
        &Dim::zero(),
        &Dim::zero(),
        None,
        &mut out,
        &mut paint_budget,
    )?;
    Ok(out)
}

fn flatten_into(
    math_box: &MathBox,
    x: &Dim,
    parent_baseline: &Dim,
    color: Option<Color>,
    out: &mut Vec<ExactMathOp>,
    paint_budget: &mut usize,
) -> Result<(), NumericError> {
    let baseline = parent_baseline.checked_add(&math_box.shift)?;
    match &math_box.content {
        BoxContent::Empty | BoxContent::Kern(_) => {}
        BoxContent::Glyph {
            glyph_id, scale, ..
        } => out.push(ExactMathOp::Glyph {
            glyph_id: *glyph_id,
            x: x.clone(),
            baseline,
            scale: scale.clone(),
            color,
        }),
        BoxContent::Rule => {
            let height = math_box.height.checked_add(&math_box.depth)?;
            if math_box.width > Dim::zero() && height > Dim::zero() {
                out.push(ExactMathOp::Rule {
                    x: x.clone(),
                    y: baseline.checked_sub(&math_box.depth)?,
                    width: math_box.width.clone(),
                    height,
                    color,
                });
            }
        }
        BoxContent::HList(children) => {
            let mut child_x = x.clone();
            for child in children {
                flatten_into(child, &child_x, &baseline, color, out, paint_budget)?;
                child_x = child_x.checked_add(&child.width)?;
            }
        }
        BoxContent::VList(children) => {
            if let Some((first, rest)) = children.split_first() {
                let mut child_baseline = baseline.clone();
                flatten_into(first, x, &child_baseline, color, out, paint_budget)?;
                child_baseline = child_baseline.checked_sub(&first.depth)?;
                for child in rest {
                    child_baseline = child_baseline.checked_sub(&child.height)?;
                    flatten_into(child, x, &child_baseline, color, out, paint_budget)?;
                    child_baseline = child_baseline.checked_sub(&child.depth)?;
                }
            }
        }
        BoxContent::Color(next, inner) => {
            flatten_into(inner, x, &baseline, Some(*next), out, paint_budget)?
        }
        BoxContent::BackColor(background, inner) => {
            let height = math_box.height.checked_add(&math_box.depth)?;
            if math_box.width > Dim::zero() && height > Dim::zero() {
                out.push(ExactMathOp::Background {
                    x: x.clone(),
                    y: baseline.checked_sub(&math_box.depth)?,
                    width: math_box.width.clone(),
                    height,
                    color: Some(*background),
                });
            }
            flatten_into(inner, x, &baseline, color, out, paint_budget)?;
        }
        BoxContent::Overlap(children) => {
            for child in children {
                flatten_into(child, x, &baseline, color, out, paint_budget)?;
            }
        }
        BoxContent::PaintCopies { inner, dx } => {
            // Nested repeats multiply paint commands despite linear box storage.
            if out.len() >= MAX_PMB_PAINT_STEPS || *paint_budget == 0 {
                return Err(NumericError::OutOfRange);
            }
            *paint_budget -= 1;
            let second_x = x.checked_add(dx)?;
            let third_x = second_x.checked_add(dx)?;
            flatten_into(inner, x, &baseline, color, out, paint_budget)?;
            flatten_into(inner, &second_x, &baseline, color, out, paint_budget)?;
            flatten_into(inner, &third_x, &baseline, color, out, paint_budget)?;
        }
        BoxContent::Line {
            x1,
            y1,
            x2,
            y2,
            thickness,
        } => out.push(ExactMathOp::Line {
            x1: x.checked_add(x1)?,
            y1: baseline.checked_add(y1)?,
            x2: x.checked_add(x2)?,
            y2: baseline.checked_add(y2)?,
            thickness: thickness.clone(),
            color,
        }),
        BoxContent::Frame {
            thickness,
            stroke,
            inner,
        } => {
            flatten_into(inner, x, &baseline, color, out, paint_budget)?;
            let height = math_box.height.checked_add(&math_box.depth)?;
            if math_box.width > Dim::zero() && height > Dim::zero() && thickness > &Dim::zero() {
                out.push(ExactMathOp::Frame {
                    x: x.clone(),
                    y: baseline.checked_sub(&math_box.depth)?,
                    width: math_box.width.clone(),
                    height,
                    thickness: thickness.clone(),
                    color: (*stroke).or(color),
                });
            }
        }
    }
    Ok(())
}

fn convert_op(op: ExactMathOp) -> Result<MathOp, NumericError> {
    Ok(match op {
        ExactMathOp::Glyph {
            glyph_id,
            x,
            baseline,
            scale,
            color,
        } => MathOp::Glyph {
            glyph_id,
            x: dim_to_f32(&x)?,
            baseline: dim_to_f32(&baseline)?,
            scale: dim_to_f32(&scale)?,
            color,
        },
        ExactMathOp::Rule {
            x,
            y,
            width,
            height,
            color,
        } => MathOp::Rule {
            x: dim_to_f32(&x)?,
            y: dim_to_f32(&y)?,
            width: dim_to_f32(&width)?,
            height: dim_to_f32(&height)?,
            color,
        },
        ExactMathOp::Line {
            x1,
            y1,
            x2,
            y2,
            thickness,
            color,
        } => MathOp::Line {
            x1: dim_to_f32(&x1)?,
            y1: dim_to_f32(&y1)?,
            x2: dim_to_f32(&x2)?,
            y2: dim_to_f32(&y2)?,
            thickness: dim_to_f32(&thickness)?,
            color,
        },
        ExactMathOp::Frame {
            x,
            y,
            width,
            height,
            thickness,
            color,
        } => MathOp::Frame {
            x: dim_to_f32(&x)?,
            y: dim_to_f32(&y)?,
            width: dim_to_f32(&width)?,
            height: dim_to_f32(&height)?,
            thickness: dim_to_f32(&thickness)?,
            color,
        },
        ExactMathOp::Background {
            x,
            y,
            width,
            height,
            color,
        } => MathOp::Background {
            x: dim_to_f32(&x)?,
            y: dim_to_f32(&y)?,
            width: dim_to_f32(&width)?,
            height: dim_to_f32(&height)?,
            color,
        },
    })
}

// Direct exact-rational -> binary32 conversion. `Dim` guarantees a positive
// denominator and numerator/denominator magnitudes <= i128::MAX. Therefore
// every intermediate left shift below is bounded by 127 bits, and the rounded
// result is finite. Generating significand bits directly avoids an f64
// intermediate and its possible double rounding.
fn fraction_step(rem: u128, den: u128) -> (u32, u128) {
    let complement = den - rem;
    if rem >= complement {
        (1, rem - complement)
    } else {
        (0, rem + rem)
    }
}

fn normalized_fraction(n: u128, d: u128, exponent: i32) -> (u128, u128) {
    if exponent >= 0 {
        let den = d << exponent as u32;
        return (n - den, den);
    }

    let steps = (-exponent) as u32;
    let mut scaled = n;
    for step in 0..steps {
        let complement = d - scaled;
        if scaled >= complement {
            debug_assert_eq!(step + 1, steps);
            return (scaled - complement, d);
        }
        scaled += scaled;
    }
    unreachable!("normalization exponent must expose the leading binary digit")
}

fn rational_to_f32_bits(num: i128, den: i128) -> u32 {
    debug_assert!(den > 0);
    if num == 0 {
        return 0;
    }

    let sign = if num < 0 { 1u32 << 31 } else { 0 };
    let n = num.unsigned_abs();
    let d = den as u128;
    let n_bits = (u128::BITS - n.leading_zeros()) as i32;
    let d_bits = (u128::BITS - d.leading_zeros()) as i32;
    let mut exponent = n_bits - d_bits;
    let below_power = if exponent >= 0 {
        n < (d << exponent as u32)
    } else {
        (n << (-exponent) as u32) < d
    };
    if below_power {
        exponent -= 1;
    }

    if exponent > 127 {
        return sign | 0x7f80_0000;
    }

    let (mut rem, norm_den) = normalized_fraction(n, d, exponent);
    let fraction_bits = if exponent >= -126 {
        23
    } else if exponent >= -149 {
        (149 + exponent) as u32
    } else if exponent == -150 {
        return sign | u32::from(rem != 0);
    } else {
        return sign;
    };

    let mut significand = 1u32;
    for _ in 0..fraction_bits {
        let (bit, next_rem) = fraction_step(rem, norm_den);
        significand = (significand << 1) | bit;
        rem = next_rem;
    }

    let complement = norm_den - rem;
    if rem > complement || (rem == complement && significand & 1 != 0) {
        significand += 1;
    }

    if exponent >= -126 {
        if significand == 1 << 24 {
            significand >>= 1;
            exponent += 1;
            if exponent > 127 {
                return sign | 0x7f80_0000;
            }
        }
        let biased = (exponent + 127) as u32;
        return sign | (biased << 23) | (significand & 0x7f_ffff);
    }

    if significand >= 1 << 23 {
        sign | (1 << 23)
    } else {
        sign | significand
    }
}

pub(crate) fn dim_to_f32(value: &Dim) -> Result<f32, NumericError> {
    let (num, den) = value.as_ratio();
    let out = f32::from_bits(rational_to_f32_bits(num, den));
    if out.is_finite() {
        Ok(out)
    } else {
        Err(NumericError::OutOfRange)
    }
}

#[cfg(test)]
mod tests {
    use super::{dim_to_f32, flatten_exact, ExactMathOp};
    use crate::layout::{BoxContent, MathBox};
    use crate::{Color, Dim};

    #[test]
    fn conversion_contract_covers_required_value_classes() {
        let cases = [
            ("zero", Dim::zero(), 0x0000_0000),
            (
                "negative coordinate",
                Dim::ratio(-7, 3).unwrap(),
                0xc015_5555,
            ),
            (
                "fractional font-unit ratio",
                Dim::ratio(7, 10).unwrap(),
                0x3f33_3333,
            ),
            (
                "small script-scale value",
                Dim::ratio(1, 15).unwrap(),
                0x3d88_8889,
            ),
            (
                "large legal magnitude",
                Dim::parse("170141183460469231731687303715884105727").unwrap(),
                0x7f00_0000,
            ),
        ];
        for (name, value, expected) in cases {
            let actual = dim_to_f32(&value).unwrap();
            assert!(actual.is_finite(), "{name}");
            assert_eq!(actual.to_bits(), expected, "{name}");
        }
    }

    #[test]
    fn conversion_rounds_directly_to_binary32_ties_to_even() {
        let half_ulp = Dim::ratio(1, 1 << 24).unwrap();
        let tiny = Dim::ratio(1, 1 << 40)
            .unwrap()
            .checked_mul(&Dim::ratio(1, 1 << 40).unwrap())
            .unwrap();
        let midpoint = Dim::one().checked_add(&half_ulp).unwrap();
        let above_midpoint = midpoint.checked_add(&tiny).unwrap();

        assert_eq!(dim_to_f32(&midpoint).unwrap().to_bits(), 0x3f80_0000);
        assert_eq!(dim_to_f32(&above_midpoint).unwrap().to_bits(), 0x3f80_0001);
        assert_eq!(
            dim_to_f32(&(-above_midpoint)).unwrap().to_bits(),
            0xbf80_0001
        );
    }

    #[test]
    fn exact_flatten_preserves_paint_order_and_color_state() {
        let glyph = MathBox {
            width: Dim::one(),
            height: Dim::one(),
            depth: Dim::zero(),
            italic: Dim::zero(),
            shift: Dim::zero(),
            content: BoxContent::glyph('x', 7, Dim::one()),
        };
        let colored = MathBox {
            width: glyph.width.clone(),
            height: glyph.height.clone(),
            depth: glyph.depth.clone(),
            italic: Dim::zero(),
            shift: Dim::zero(),
            content: BoxContent::Color(Color::rgb(255, 0, 0), Box::new(glyph)),
        };
        let background = MathBox {
            width: colored.width.clone(),
            height: colored.height.clone(),
            depth: colored.depth.clone(),
            italic: Dim::zero(),
            shift: Dim::zero(),
            content: BoxContent::BackColor(Color::rgb(255, 255, 0), Box::new(colored)),
        };

        let trace = flatten_exact(&background).unwrap();
        assert!(matches!(
            trace.as_slice(),
            [
                ExactMathOp::Background { color, .. },
                ExactMathOp::Glyph {
                    glyph_id: 7,
                    color: glyph_color,
                    ..
                }
            ] if *color == Some(Color::rgb(255, 255, 0))
                && *glyph_color == Some(Color::rgb(255, 0, 0))
        ));
    }
}
