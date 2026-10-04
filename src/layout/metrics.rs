//! OpenType MATH constants as [`Dim`](crate::Dim). TeX σ-names are documented on fields.

use crate::dim::Dim;
use crate::error::{Error, NumericError};
use crate::font::{MathFont, MathFontView};
use crate::layout::style::MathStyle;

/// MATH-table parameters used by the layout engine.
///
/// # Examples
///
/// ```no_run
/// use texpose::{MathFont, MathParams, MathStyle};
/// # fn font_bytes() -> &'static [u8] { unimplemented!() }
///
/// let font = MathFont::from_bytes(font_bytes()).unwrap();
/// let p = MathParams::from_font(&font).unwrap();
/// assert!(!p.axis_height.is_zero());
/// assert!(!p.em(MathStyle::Text).unwrap().is_zero());
/// ```
#[derive(Clone, Debug)]
pub struct MathParams {
    /// σ1: x-height (em).
    pub x_height: Dim,
    /// σ2: quad / em width.
    pub quad: Dim,
    /// σ6 / σ22: math axis height.
    pub axis_height: Dim,
    /// Accent base height.
    pub accent_base_height: Dim,
    /// Flattened accent base height (cramped / tall bases).
    pub flattened_accent_base_height: Dim,
    /// σ8: default rule thickness (fraction / radical bar).
    pub fraction_rule_thickness: Dim,
    /// σ10 analogue: numerator shift (text).
    pub fraction_numerator_shift_up: Dim,
    /// σ9 analogue: numerator shift (display).
    pub fraction_numerator_display_style_shift_up: Dim,
    /// σ12 analogue: denominator shift (text).
    pub fraction_denominator_shift_down: Dim,
    /// σ11 analogue: denominator shift (display).
    pub fraction_denominator_display_style_shift_down: Dim,
    /// Minimum gap num ↔ rule (text).
    pub fraction_numerator_gap_min: Dim,
    /// Minimum gap num ↔ rule (display).
    pub fraction_num_display_style_gap_min: Dim,
    /// Minimum gap rule ↔ den (text).
    pub fraction_denominator_gap_min: Dim,
    /// Minimum gap rule ↔ den (display).
    pub fraction_denom_display_style_gap_min: Dim,
    /// σ14 analogue: superscript shift (text).
    pub superscript_shift_up: Dim,
    /// σ15 analogue: superscript shift cramped.
    pub superscript_shift_up_cramped: Dim,
    /// σ16 analogue: subscript shift.
    pub subscript_shift_down: Dim,
    /// Minimum gap between sub and sup.
    pub sub_superscript_gap_min: Dim,
    /// Space after a script.
    pub space_after_script: Dim,
    /// Radical vertical gap (text).
    pub radical_vertical_gap: Dim,
    /// Radical vertical gap (display).
    pub radical_display_style_vertical_gap: Dim,
    /// Radical rule thickness.
    pub radical_rule_thickness: Dim,
    /// Extra ascender above radical rule.
    pub radical_extra_ascender: Dim,
    /// Kern before a radical degree.
    pub radical_kern_before_degree: Dim,
    /// Kern after a radical degree.
    pub radical_kern_after_degree: Dim,
    /// Degree bottom raise percent (integer 0–100).
    pub radical_degree_bottom_raise_percent: i16,
    /// Overbar gap.
    pub overbar_vertical_gap: Dim,
    /// Overbar rule thickness.
    pub overbar_rule_thickness: Dim,
    /// Overbar extra ascender.
    pub overbar_extra_ascender: Dim,
    /// Underbar gap.
    pub underbar_vertical_gap: Dim,
    /// Underbar rule thickness.
    pub underbar_rule_thickness: Dim,
    /// Underbar extra descender.
    pub underbar_extra_descender: Dim,
    /// Upper limit gap min.
    pub upper_limit_gap_min: Dim,
    /// Upper limit baseline rise min.
    pub upper_limit_baseline_rise_min: Dim,
    /// Lower limit gap min.
    pub lower_limit_gap_min: Dim,
    /// Lower limit baseline drop min.
    pub lower_limit_baseline_drop_min: Dim,
    /// Display operator min height (font units, as Dim em).
    pub display_operator_min_height: Dim,
    /// Script scale (percent, e.g. 70).
    pub script_percent_scale_down: i16,
    /// Scriptscript scale (percent, e.g. 55).
    pub script_script_percent_scale_down: i16,
    /// `unitsPerEm`.
    pub units_per_em: u16,
}

impl MathParams {
    /// Load validated MATH constants from `font`.
    ///
    /// MATH-table absence and malformation are rejected while constructing
    /// [`MathFont`], so layout never needs an absent-table fallback.
    pub fn from_font(font: &MathFont) -> Result<Self, Error> {
        let font = font.operation_view();
        Ok(Self::from_view(&font))
    }

    pub(crate) fn from_view(font: &MathFontView<'_>) -> Self {
        let face = font.face();
        let c = font.math_constants();
        let xh = match face.x_height() {
            Some(value) => font.font_units(i64::from(value)),
            None => Dim::ratio(1, 2).expect("static nonzero x-height fallback denominator"),
        };
        Self {
            x_height: xh,
            quad: Dim::one(),
            axis_height: font.math_value(c.axis_height()),
            accent_base_height: font.math_value(c.accent_base_height()),
            flattened_accent_base_height: font.math_value(c.flattened_accent_base_height()),
            fraction_rule_thickness: font.math_value(c.fraction_rule_thickness()),
            fraction_numerator_shift_up: font.math_value(c.fraction_numerator_shift_up()),
            fraction_numerator_display_style_shift_up: font
                .math_value(c.fraction_numerator_display_style_shift_up()),
            fraction_denominator_shift_down: font.math_value(c.fraction_denominator_shift_down()),
            fraction_denominator_display_style_shift_down: font
                .math_value(c.fraction_denominator_display_style_shift_down()),
            fraction_numerator_gap_min: font.math_value(c.fraction_numerator_gap_min()),
            fraction_num_display_style_gap_min: font
                .math_value(c.fraction_num_display_style_gap_min()),
            fraction_denominator_gap_min: font.math_value(c.fraction_denominator_gap_min()),
            fraction_denom_display_style_gap_min: font
                .math_value(c.fraction_denom_display_style_gap_min()),
            superscript_shift_up: font.math_value(c.superscript_shift_up()),
            superscript_shift_up_cramped: font.math_value(c.superscript_shift_up_cramped()),
            subscript_shift_down: font.math_value(c.subscript_shift_down()),
            sub_superscript_gap_min: font.math_value(c.sub_superscript_gap_min()),
            space_after_script: font.math_value(c.space_after_script()),
            radical_vertical_gap: font.math_value(c.radical_vertical_gap()),
            radical_display_style_vertical_gap: font
                .math_value(c.radical_display_style_vertical_gap()),
            radical_rule_thickness: font.math_value(c.radical_rule_thickness()),
            radical_extra_ascender: font.math_value(c.radical_extra_ascender()),
            radical_kern_before_degree: font.math_value(c.radical_kern_before_degree()),
            radical_kern_after_degree: font.math_value(c.radical_kern_after_degree()),
            radical_degree_bottom_raise_percent: c.radical_degree_bottom_raise_percent(),
            overbar_vertical_gap: font.math_value(c.overbar_vertical_gap()),
            overbar_rule_thickness: font.math_value(c.overbar_rule_thickness()),
            overbar_extra_ascender: font.math_value(c.overbar_extra_ascender()),
            underbar_vertical_gap: font.math_value(c.underbar_vertical_gap()),
            underbar_rule_thickness: font.math_value(c.underbar_rule_thickness()),
            underbar_extra_descender: font.math_value(c.underbar_extra_descender()),
            upper_limit_gap_min: font.math_value(c.upper_limit_gap_min()),
            upper_limit_baseline_rise_min: font.math_value(c.upper_limit_baseline_rise_min()),
            lower_limit_gap_min: font.math_value(c.lower_limit_gap_min()),
            lower_limit_baseline_drop_min: font.math_value(c.lower_limit_baseline_drop_min()),
            display_operator_min_height: font
                .font_units(i64::from(c.display_operator_min_height())),
            script_percent_scale_down: c.script_percent_scale_down(),
            script_script_percent_scale_down: c.script_script_percent_scale_down(),
            units_per_em: font.units_per_em_nonzero().get(),
        }
    }

    /// Scale factor for `style` (1, script%, or scriptscript%).
    #[must_use]
    pub fn scale(&self, style: MathStyle) -> Dim {
        let percent = match style.script_level() {
            0 => return Dim::one(),
            1 => self.script_percent_scale_down,
            _ => self.script_script_percent_scale_down,
        };
        Dim::ratio(i64::from(percent), 100)
            .expect("i16 percentage over static nonzero denominator fits Dim")
    }

    /// Current em (`quad * scale`).
    ///
    /// # Errors
    ///
    /// Returns [`NumericError::ArithmeticOverflow`] if caller-supplied public
    /// parameters make the exact product exceed the supported [`Dim`] range.
    pub fn em(&self, style: MathStyle) -> Result<Dim, NumericError> {
        self.quad.checked_mul(&self.scale(style))
    }

    /// One mu at `style` (`current math quad / 18`).
    ///
    /// # Errors
    ///
    /// Propagates exact-arithmetic overflow from scaling the current math quad.
    pub fn mu(&self, style: MathStyle) -> Result<Dim, NumericError> {
        self.quad
            .checked_mul(&self.scale(style))?
            .checked_div(&Dim::from_i64(18))
    }
}
