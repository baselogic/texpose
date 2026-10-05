//! `MathNode` → `MathBox`. All sizes are [`Dim`](crate::Dim).

use core::cell::{Cell, RefCell};
use core::cmp::Ordering;

use crate::atoms::symbol_atom_kind;
use crate::color::Color;
use crate::dim::Dim;
use crate::error::{Error, FontError, NumericError};
use crate::font::{MathFont, MathFontView, MathGsubContext, MathKernCorner};
use crate::layout::assembly::{solve_glyph_assembly, AssemblySolution};
use crate::layout::metrics::MathParams;
use crate::layout::numbering::{NumberingPlan, NumberingState};
use crate::layout::semantic::{
    noad_class, normalize_row, operator_semantics, script_semantics, script_styles, LimitPlacement,
    OperatorNucleus, OperatorSemantics, ScriptSemantics, SemanticItem,
};
use crate::layout::space::{atom_space_mu, space_width};
use crate::layout::style::MathStyle;
use crate::layout::{BoxContent, LayoutDiagnostic, LayoutOutput, MathBox, RootEmSize};
use crate::parser::DEFAULT_MAX_NESTING_DEPTH;
use crate::parser::{
    AccentKind, AtomKind, ColSpec, DelimSize, Delimiter, EnvRow, FractionAlignment, FractionRule,
    FractionSpec, FractionStyle, IntegralKind, Length, MathNode, MatrixStyle, PhantomKind,
    SpaceKind, TextStyle,
};
use crate::style_map::styled_char;
use crate::symbols::lookup;

const DEFAULT_LAYOUT_EM_SIZE_PT: i64 = 10;
const TEX_NULL_DELIMITER_SPACE_PT_NUM: i64 = 6;
const TEX_NULL_DELIMITER_SPACE_PT_DEN: i64 = 5;
const TEX_DELIMITER_FACTOR_NUM: i64 = 901;
const TEX_DELIMITER_FACTOR_DEN: i64 = 500;
const TEX_DELIMITER_SHORTFALL_PT: i64 = 5;
const TEX_ARRAY_COLSEP_PT: i64 = 5;
const TEX_MIN_ALIGN_SEP_PT: i64 = 10;
const TEX_JOT_PT: i64 = 3;
const TEX_LINE_SKIP_PT: i64 = 1;
// Standard LaTeX article/report/book defaults used by \fbox and color boxes.
const TEX_FBOX_SEP_PT: i64 = 3;
const TEX_FBOX_RULE_PT_NUM: i64 = 2;
const TEX_FBOX_RULE_PT_DEN: i64 = 5;
const TEX_CANCEL_LINE_PT_NUM: i64 = 2;
const TEX_CANCEL_LINE_PT_DEN: i64 = 5;
// cancel.sty adds two physical points to the selected picture-line span.
// TeXpose maps that discrete picture-font construction to a backend-neutral
// free line extending one physical point beyond each measured box edge.
const TEX_CANCEL_OVERSHOOT_PT: i64 = 1;

/// Lay out `node` in `style` using caller-provided OpenType MATH metrics.
///
/// Every dimension on the returned [`MathBox`] is a [`Dim`](crate::Dim).
/// Missing cmap entries degrade deterministically instead of failing layout.
/// This compatibility entry point discards recoverable diagnostics; use
/// [`layout_with_diagnostics`] when the caller must observe them.
///
/// # Arguments
///
/// * `node` — parsed math tree.
/// * `font` — face providing MATH constants and glyph metrics.
/// * `style` — TeX math style (`Display`, `Text`, scripts).
///
/// # Returns
///
/// A backend-neutral mathematical box tree.
///
/// # Errors
///
/// * [`Error::Font`] — a construct-specific source glyph or metric cannot be used.
/// * [`Error::Unsupported`] — construct or MATH table the engine will not fake.
/// * [`Error::Malformed`] — invalid structure discovered during layout.
/// * [`Error::Numeric`] — exact dimension arithmetic exceeded the supported
///   [`Dim`](crate::Dim) range.
///
/// # Examples
///
/// ```no_run
/// use texpose::{layout, parse, MathFont, MathStyle};
/// # fn font_bytes() -> &'static [u8] { unimplemented!() }
///
/// let ast = parse(r"\frac{1}{2}").unwrap();
/// let font = MathFont::from_bytes(font_bytes()).unwrap();
/// let boxed = layout(&ast, &font, MathStyle::Text).unwrap();
/// assert!(!boxed.width.is_zero());
/// ```
pub fn layout(node: &MathNode, font: &MathFont, style: MathStyle) -> Result<MathBox, Error> {
    Ok(layout_with_diagnostics(node, font, style)?.math_box)
}

/// Lay out `node` and return recoverable diagnostics with the box tree.
///
/// A required Unicode scalar missing from cmap produces
/// [`LayoutDiagnostic::MissingGlyph`] and a deterministic substitute box while
/// layout continues. An unusable OpenType MATH glyph assembly produces
/// [`LayoutDiagnostic::ExtensibleFallback`] while retaining the largest valid
/// ready-made construction candidate. Other construct-specific source glyph
/// failures retain their documented degradation policy.
///
/// # Errors
///
/// Same unrecoverable failures as [`layout`].
pub fn layout_with_diagnostics(
    node: &MathNode,
    font: &MathFont,
    style: MathStyle,
) -> Result<LayoutOutput, Error> {
    let mut state = NumberingState::default();
    let root_em_size = default_root_em_size();
    layout_impl(
        node,
        font,
        style,
        &mut state,
        DEFAULT_MAX_NESTING_DEPTH,
        &root_em_size,
    )
}

/// Lay out `node` using an explicit physical root em size in TeX points.
///
/// Returned dimensions remain normalized em units. The physical root em is
/// used only to normalize absolute TeX dimensions such as
/// `\nulldelimiterspace`. Recoverable diagnostics are discarded; use
/// [`layout_with_em_size_pt_and_diagnostics`] to retain them.
///
/// # Errors
///
/// Same as [`layout`], plus [`Error::InvalidOption`] when `em_size_pt` is not
/// positive.
pub fn layout_with_em_size_pt(
    node: &MathNode,
    font: &MathFont,
    style: MathStyle,
    em_size_pt: &Dim,
) -> Result<MathBox, Error> {
    Ok(layout_with_em_size_pt_and_diagnostics(node, font, style, em_size_pt)?.math_box)
}

/// Lay out with an explicit physical root em size and retain diagnostics.
///
/// # Errors
///
/// Same as [`layout_with_em_size_pt`].
pub fn layout_with_em_size_pt_and_diagnostics(
    node: &MathNode,
    font: &MathFont,
    style: MathStyle,
    em_size_pt: &Dim,
) -> Result<LayoutOutput, Error> {
    let root_em_size = RootEmSize::new(em_size_pt.clone())?;
    let mut state = NumberingState::default();
    layout_impl(
        node,
        font,
        style,
        &mut state,
        DEFAULT_MAX_NESTING_DEPTH,
        &root_em_size,
    )
}

/// Lay out with a caller-owned equation counter and `\label` / `\ref` table.
///
/// # Arguments
///
/// * `node` — parsed math tree.
/// * `font` — face providing MATH constants and glyph metrics.
/// * `style` — TeX math style.
/// * `state` — counter and label map; survives across calls.
///
/// # Returns
///
/// A box tree. Numbers assigned for this tree are recorded in `state`. Recoverable
/// diagnostics are discarded; use [`layout_with_numbering_and_diagnostics`] to
/// retain them.
///
/// # Errors
///
/// Same as [`layout`], plus [`Error::InvalidOption`] when the numbering
/// configuration cannot produce the next automatic equation number.
///
/// # Examples
///
/// ```no_run
/// use texpose::{layout_with_numbering, parse, MathFont, MathStyle, NumberingState};
/// # fn font_bytes() -> &'static [u8] { unimplemented!() }
///
/// let ast = parse(r"\begin{equation}x\end{equation}").unwrap();
/// let font = MathFont::from_bytes(font_bytes()).unwrap();
/// let mut state = NumberingState::default();
/// let boxed = layout_with_numbering(&ast, &font, MathStyle::Display, &mut state).unwrap();
/// assert!(!boxed.width.is_zero());
/// ```
pub fn layout_with_numbering(
    node: &MathNode,
    font: &MathFont,
    style: MathStyle,
    state: &mut NumberingState,
) -> Result<MathBox, Error> {
    Ok(layout_with_numbering_and_diagnostics(node, font, style, state)?.math_box)
}

/// Lay out with caller-owned numbering and retain recoverable diagnostics.
///
/// # Errors
///
/// Same as [`layout_with_numbering`].
pub fn layout_with_numbering_and_diagnostics(
    node: &MathNode,
    font: &MathFont,
    style: MathStyle,
    state: &mut NumberingState,
) -> Result<LayoutOutput, Error> {
    let root_em_size = default_root_em_size();
    layout_impl(
        node,
        font,
        style,
        state,
        DEFAULT_MAX_NESTING_DEPTH,
        &root_em_size,
    )
}

/// Lay out with caller-owned numbering and an explicit physical root em size
/// in TeX points.
///
/// Returned dimensions remain normalized em units. `em_size_pt` is validated
/// into the internal physical root-em type before any absolute-unit resolution.
/// Recoverable diagnostics are discarded; use
/// [`layout_with_numbering_and_em_size_pt_and_diagnostics`] to retain them.
///
/// # Errors
///
/// Same as [`layout_with_em_size_pt`], plus the numbering failures documented
/// by [`layout_with_numbering`].
pub fn layout_with_numbering_and_em_size_pt(
    node: &MathNode,
    font: &MathFont,
    style: MathStyle,
    state: &mut NumberingState,
    em_size_pt: &Dim,
) -> Result<MathBox, Error> {
    Ok(
        layout_with_numbering_and_em_size_pt_and_diagnostics(node, font, style, state, em_size_pt)?
            .math_box,
    )
}

/// Lay out with caller-owned numbering and physical root em, retaining diagnostics.
///
/// # Errors
///
/// Same as [`layout_with_numbering_and_em_size_pt`].
pub fn layout_with_numbering_and_em_size_pt_and_diagnostics(
    node: &MathNode,
    font: &MathFont,
    style: MathStyle,
    state: &mut NumberingState,
    em_size_pt: &Dim,
) -> Result<LayoutOutput, Error> {
    let root_em_size = RootEmSize::new(em_size_pt.clone())?;
    layout_impl(
        node,
        font,
        style,
        state,
        DEFAULT_MAX_NESTING_DEPTH,
        &root_em_size,
    )
}

/// Lay out with an explicit nesting limit instead of
/// [`DEFAULT_MAX_NESTING_DEPTH`](crate::DEFAULT_MAX_NESTING_DEPTH).
///
/// Use the same limit given to [`ParseOptions::with_max_depth`](crate::ParseOptions::with_max_depth),
/// so that every tree the parser accepts can also be laid out. Recoverable
/// diagnostics are discarded; use [`layout_with_max_depth_and_diagnostics`] to
/// retain them.
///
/// # Errors
///
/// Same as [`layout`]. A tree nesting deeper than `max_depth` returns
/// [`Error::Unsupported`].
///
/// # Examples
///
/// ```no_run
/// use texpose::{layout_with_max_depth, parse, MathFont, MathStyle};
/// # fn font_bytes() -> &'static [u8] { unimplemented!() }
///
/// let ast = parse(r"\frac{1}{2}").unwrap();
/// let font = MathFont::from_bytes(font_bytes()).unwrap();
/// assert!(layout_with_max_depth(&ast, &font, MathStyle::Text, 64).is_ok());
/// assert!(layout_with_max_depth(&ast, &font, MathStyle::Text, 1).is_err());
/// ```
pub fn layout_with_max_depth(
    node: &MathNode,
    font: &MathFont,
    style: MathStyle,
    max_depth: usize,
) -> Result<MathBox, Error> {
    Ok(layout_with_max_depth_and_diagnostics(node, font, style, max_depth)?.math_box)
}

/// Lay out with an explicit nesting limit and retain recoverable diagnostics.
///
/// # Errors
///
/// Same as [`layout_with_max_depth`].
pub fn layout_with_max_depth_and_diagnostics(
    node: &MathNode,
    font: &MathFont,
    style: MathStyle,
    max_depth: usize,
) -> Result<LayoutOutput, Error> {
    let mut state = NumberingState::default();
    let root_em_size = default_root_em_size();
    layout_impl(node, font, style, &mut state, max_depth, &root_em_size)
}

fn default_root_em_size() -> RootEmSize {
    RootEmSize::new(Dim::from_i64(DEFAULT_LAYOUT_EM_SIZE_PT))
        .expect("positive static default root em size")
}

fn layout_impl(
    node: &MathNode,
    font: &MathFont,
    style: MathStyle,
    state: &mut NumberingState,
    max_depth: usize,
    root_em_size: &RootEmSize,
) -> Result<LayoutOutput, Error> {
    // Parse the selected OpenType face once and reuse it for the complete
    // layout operation.
    let font = font.operation_view();
    let params = MathParams::from_view(&font);
    let script_placement = ScriptPlacementParams::from_view(&font);
    let substack = SubstackParams::from_view(&font, &params)?;
    let fraction_stack = FractionStackParams::from_view(&font);
    let null_delimiter_space = resolve_length(
        &Length::TexPt(Dim::ratio(
            TEX_NULL_DELIMITER_SPACE_PT_NUM,
            TEX_NULL_DELIMITER_SPACE_PT_DEN,
        )?),
        MathStyle::Text,
        &params,
        root_em_size,
    )?;
    let delimiter_shortfall = resolve_length(
        &Length::TexPt(Dim::from_i64(TEX_DELIMITER_SHORTFALL_PT)),
        MathStyle::Text,
        &params,
        root_em_size,
    )?;
    let numbering = state.prepare(node)?;
    let output = {
        let engine = Engine {
            font,
            params,
            script_placement,
            substack,
            fraction_stack,
            null_delimiter_space,
            delimiter_shortfall,
            root_em_size: root_em_size.clone(),
            numbers: state,
            numbering: &numbering,
            idx: Cell::new(0),
            depth: Cell::new(0),
            max_depth,
            diagnostics: RefCell::new(Vec::new()),
        };
        let math_box = engine.layout(node, style)?;
        LayoutOutput {
            math_box,
            diagnostics: engine.diagnostics.into_inner(),
        }
    };
    state.commit(numbering);
    Ok(output)
}

fn resolve_length(
    length: &Length,
    style: MathStyle,
    params: &MathParams,
    root_em_size: &RootEmSize,
) -> Result<Dim, NumericError> {
    match length {
        Length::Em(value) => value.checked_mul(&params.em(style)?),
        Length::Mu(value) => value.checked_mul(&params.mu(style)?),
        Length::TexPt(value) => value.checked_div(root_em_size.tex_points()),
        Length::BigPt(value) => value
            .checked_mul(&Dim::ratio(7227, 7200)?)?
            .checked_div(root_em_size.tex_points()),
    }
}

struct Engine<'font, 'state> {
    font: MathFontView<'font>,
    params: MathParams,
    script_placement: ScriptPlacementParams,
    substack: SubstackParams,
    fraction_stack: FractionStackParams,
    null_delimiter_space: Dim,
    delimiter_shortfall: Dim,
    root_em_size: RootEmSize,
    numbers: &'state NumberingState,
    numbering: &'state NumberingPlan,
    idx: Cell<usize>,
    /// Current nesting depth, bounded by `max_depth`.
    depth: Cell<usize>,
    /// Deepest nesting accepted.
    max_depth: usize,
    diagnostics: RefCell<Vec<LayoutDiagnostic>>,
}

struct ScriptPlacementParams {
    subscript_top_max: Dim,
    subscript_baseline_drop_min: Dim,
    superscript_bottom_min: Dim,
    superscript_bottom_max_with_subscript: Dim,
    superscript_baseline_drop_max: Dim,
}

impl ScriptPlacementParams {
    fn from_view(font: &MathFontView<'_>) -> Self {
        let constants = font.math_constants();

        Self {
            subscript_top_max: font.math_value(constants.subscript_top_max()),
            subscript_baseline_drop_min: font.math_value(constants.subscript_baseline_drop_min()),
            superscript_bottom_min: font.math_value(constants.superscript_bottom_min()),
            superscript_bottom_max_with_subscript: font
                .math_value(constants.superscript_bottom_max_with_subscript()),
            superscript_baseline_drop_max: font
                .math_value(constants.superscript_baseline_drop_max()),
        }
    }
}

struct SubstackParams {
    baseline_skip: Dim,
    line_skip: Dim,
}

impl SubstackParams {
    fn from_view(font: &MathFontView<'_>, params: &MathParams) -> Result<Self, Error> {
        let constants = font.math_constants();
        let script_scale = params.scale(MathStyle::Script);

        let top = font.math_value(constants.stack_top_shift_up());
        let bottom = font.math_value(constants.stack_bottom_shift_down());
        let gap = font.math_value(constants.stack_gap_min());

        Ok(Self {
            baseline_skip: top.checked_add(&bottom)?.checked_mul(&script_scale)?,
            line_skip: gap.checked_mul(&script_scale)?,
        })
    }
}
/// OpenType MATH stack constants used by ruleless generalized fractions.
/// A no-rule fraction is a stack, not a ruled fraction with zero thickness.
struct FractionStackParams {
    top_shift_up: Dim,
    top_display_shift_up: Dim,
    bottom_shift_down: Dim,
    bottom_display_shift_down: Dim,
    gap_min: Dim,
    display_gap_min: Dim,
}

impl FractionStackParams {
    fn from_view(font: &MathFontView<'_>) -> Self {
        let constants = font.math_constants();
        Self {
            top_shift_up: font.math_value(constants.stack_top_shift_up()),
            top_display_shift_up: font.math_value(constants.stack_top_display_style_shift_up()),
            bottom_shift_down: font.math_value(constants.stack_bottom_shift_down()),
            bottom_display_shift_down: font
                .math_value(constants.stack_bottom_display_style_shift_down()),
            gap_min: font.math_value(constants.stack_gap_min()),
            display_gap_min: font.math_value(constants.stack_display_style_gap_min()),
        }
    }

    fn scaled(&self, style: MathStyle, scale: &Dim) -> Result<(Dim, Dim, Dim), Error> {
        let (top, bottom, gap) = if style.is_display() {
            (
                &self.top_display_shift_up,
                &self.bottom_display_shift_down,
                &self.display_gap_min,
            )
        } else {
            (&self.top_shift_up, &self.bottom_shift_down, &self.gap_min)
        };
        Ok((
            top.checked_mul(scale)?,
            bottom.checked_mul(scale)?,
            gap.checked_mul(scale)?,
        ))
    }
}

struct Item {
    bx: MathBox,
    class: Option<AtomKind>,
}

impl<'font, 'state> Engine<'font, 'state> {
    fn layout(&self, node: &MathNode, style: MathStyle) -> Result<MathBox, Error> {
        Ok(self.item(node, style)?.bx)
    }

    /// Lay out `node` one level deeper, refusing to descend past `max_depth`.
    ///
    /// `parse` bounds the depth of any tree it builds, so this catches only a
    /// tree assembled by hand. It exists because `layout` is public and must
    /// not overflow the caller's stack whatever it is handed.
    fn item(&self, node: &MathNode, style: MathStyle) -> Result<Item, Error> {
        let depth = self.depth.get();
        if depth >= self.max_depth {
            return Err(Error::Unsupported {
                what: format!("tree nests deeper than {} levels", self.max_depth),
            });
        }
        self.depth.set(depth + 1);
        let out = self.item_inner(node, style);
        self.depth.set(depth);
        out
    }

    fn item_inner(&self, node: &MathNode, style: MathStyle) -> Result<Item, Error> {
        if let Some(operator) = operator_semantics(node, style) {
            return self.operator_noad(operator);
        }
        if let Some(scripts) = script_semantics(node, style) {
            return self.scripts(scripts);
        }

        match node {
            MathNode::Atom(c, k) => {
                let bx = self.glyph(math_italic(math_char(*c)), style)?;
                Ok(Item {
                    bx,
                    class: Some(*k),
                })
            }
            MathNode::Symbol(name) => {
                let ch = math_italic(symbol_char(name)?);
                let bx = self.glyph(ch, style)?;
                Ok(Item {
                    bx,
                    class: Some(symbol_class(name)),
                })
            }
            MathNode::Row(items) => self.row(items, style),
            MathNode::Fraction(spec) => self.generalized_fraction(spec, style),
            MathNode::Radical(deg, rad) => self.radical(deg.as_deref(), rad, style),
            MathNode::Superscript(_, _)
            | MathNode::Subscript(_, _)
            | MathNode::SubSup(_, _, _)
            | MathNode::Operator(_, _)
            | MathNode::Sum(_, _)
            | MathNode::Product(_, _)
            | MathNode::Integral(_, _, _)
            | MathNode::Limit(_) => {
                unreachable!("script/operator noads are normalized before geometry")
            }
            MathNode::Limits(_, _) => Err(Error::Malformed {
                what: "limit control does not wrap an operator nucleus".into(),
            }),
            MathNode::Style(_) => Ok(Item {
                bx: MathBox::empty(),
                class: None,
            }),
            MathNode::Delimited(open, body, close) => self.delimited(open, body, close, style),
            MathNode::SizedDelim(d, size, k) => {
                // amsmath measures fixed-size delimiters in a fresh inline math
                // formula, so their glyph size and axis are textstyle even when
                // the surrounding formula is scriptstyle.
                let delimiter_style = MathStyle::Text;
                let needed = self.explicit_delim_target(*size)?;
                let axis = self
                    .params
                    .axis_height
                    .checked_mul(&self.params.scale(delimiter_style))?;
                let bx =
                    self.center_delimiter(self.delim_box(d, &needed, delimiter_style)?, &axis)?;
                Ok(Item {
                    class: Some(*k),
                    bx,
                })
            }
            MathNode::Space(kind) => Ok(Item {
                bx: MathBox::kern(self.space_dim(kind, style)?),
                class: None,
            }),
            MathNode::Strut(h, d) => Ok(Item {
                bx: MathBox {
                    width: Dim::zero(),
                    height: self.resolve_length(h, style)?,
                    depth: self.resolve_length(d, style)?,
                    italic: Dim::zero(),
                    shift: Dim::zero(),
                    content: BoxContent::Empty,
                },
                class: Some(AtomKind::Ord),
            }),
            MathNode::Rule(_width, height) => {
                // Preserve both parsed units without changing the inherited
                // vertical-strut geometry of `\rule`; width geometry is separate.
                Ok(Item {
                    bx: MathBox {
                        width: Dim::zero(),
                        height: self.resolve_length(height, style)?,
                        depth: Dim::zero(),
                        italic: Dim::zero(),
                        shift: Dim::zero(),
                        content: BoxContent::Empty,
                    },
                    class: Some(AtomKind::Ord),
                })
            }
            MathNode::Phantom(kind, inner) => {
                // LaTeX measures math phantoms with an ordinary math hbox in the
                // current style. OpenType MATH italic correction is not appended to
                // that hbox width; copy exactly the measured box dimensions, then
                // discard paint and any box-level italic metadata.
                let mut bx = self.layout(inner, style)?;
                bx.italic = Dim::zero();
                bx.content = BoxContent::Empty;
                match kind {
                    PhantomKind::Full => {}
                    PhantomKind::Vertical => bx.width = Dim::zero(),
                    PhantomKind::Horizontal => {
                        bx.height = Dim::zero();
                        bx.depth = Dim::zero();
                    }
                }
                Ok(Item {
                    class: Some(AtomKind::Ord),
                    bx,
                })
            }
            MathNode::MathAlphabet(s, ts) => self.math_alphabet_run(s, *ts, style),
            MathNode::LiteralText(s) => self.literal_text_run(s, style),
            MathNode::OverUnder(base, over, under) => self.over_under(
                base,
                over.as_deref(),
                under.as_deref(),
                style,
                noad_class(node).unwrap_or(AtomKind::Op),
            ),
            MathNode::StackRel(base, over) => self.over_under(
                base,
                Some(over.as_ref()),
                None,
                style,
                noad_class(node).unwrap_or(AtomKind::Rel),
            ),
            MathNode::Accent(base, kind) => self.accent(base, *kind, style),
            MathNode::CancelTo(value, expr) => self.cancelto(value, expr, style),
            MathNode::Matrix(ms, spec, rows) => self.matrix(*ms, spec, rows, style),
            MathNode::Substack(lines) => self.substack(lines, style),
            MathNode::Ref(key) => self.reference(key, style),
            MathNode::Tag { star, body } => self.tag_box(*star, body, style),
            MathNode::Label(_) | MathNode::NoNumber => Ok(Item {
                bx: MathBox::empty(),
                class: None,
            }),
            MathNode::Hline => Err(Error::Unsupported {
                what: "hline outside array".into(),
            }),
            MathNode::Intertext(n) => Ok(Item {
                class: Some(AtomKind::Ord),
                bx: self.layout(n, MathStyle::Text)?,
            }),
            MathNode::Color(c, body) | MathNode::TextColor(c, body) => {
                let inner = self.layout(body, style)?;
                Ok(Item {
                    class: noad_class(body),
                    bx: color_wrap(*c, inner),
                })
            }
            MathNode::ColorBox(c, body) => {
                // TeXpose keeps color-box bodies in math mode, but follows the
                // LaTeX hbox model: terminal math italic is not appended, and
                // physical \fboxsep is independent of the current math style.
                let inner = self.layout(body, style)?;
                let pad = self.tex_points_in_root_em(TEX_FBOX_SEP_PT)?;
                Ok(Item {
                    class: Some(AtomKind::Ord),
                    bx: back_color_wrap(*c, pad_box(inner, &pad)?),
                })
            }
            MathNode::FColorBox(border, fill, body) => {
                let inner = self.layout(body, style)?;
                Ok(Item {
                    class: Some(AtomKind::Ord),
                    bx: self.framed_color_box(inner, Some(*border), Some(*fill))?,
                })
            }
        }
    }

    fn glyph(&self, ch: char, style: MathStyle) -> Result<MathBox, Error> {
        self.glyph_with_context(ch, style, MathGsubContext::None)
    }

    fn glyph_with_context(
        &self,
        ch: char,
        style: MathStyle,
        context: MathGsubContext,
    ) -> Result<MathBox, Error> {
        match self.font.glyph_index(ch) {
            Some(glyph_id) => self.glyph_id_with_context(ch, glyph_id, style, context),
            None => self.missing_glyph(ch, style),
        }
    }

    fn strict_glyph(&self, ch: char, style: MathStyle) -> Result<MathBox, Error> {
        self.strict_glyph_with_context(ch, style, MathGsubContext::None)
    }

    fn strict_glyph_with_context(
        &self,
        ch: char,
        style: MathStyle,
        context: MathGsubContext,
    ) -> Result<MathBox, Error> {
        let glyph_id = self
            .font
            .glyph_index(ch)
            .ok_or(FontError::MissingGlyph { ch })?;
        self.glyph_id_with_context(ch, glyph_id, style, context)
    }

    fn missing_glyph(&self, ch: char, style: MathStyle) -> Result<MathBox, Error> {
        self.diagnostics
            .borrow_mut()
            .push(LayoutDiagnostic::MissingGlyph { ch });

        let scale = self.params.scale(style);
        match self.font.glyph_id(ch, 0) {
            Ok(glyph) if glyph.advance_fu != 0 => {
                return Ok(MathBox {
                    width: glyph.advance.checked_mul(&scale)?,
                    height: glyph.height.checked_mul(&scale)?,
                    depth: glyph.depth.checked_mul(&scale)?,
                    italic: Dim::zero(),
                    shift: Dim::zero(),
                    content: BoxContent::glyph(ch, 0, scale),
                });
            }
            Ok(_) | Err(_) => {}
        }

        let em = self.params.em(style)?;
        Ok(MathBox {
            width: em.clone(),
            height: em,
            depth: Dim::zero(),
            italic: Dim::zero(),
            shift: Dim::zero(),
            content: BoxContent::Empty,
        })
    }

    fn glyph_id(&self, ch: char, gid: u16, style: MathStyle) -> Result<MathBox, Error> {
        self.glyph_id_with_context(ch, gid, style, MathGsubContext::None)
    }

    fn glyph_id_with_context(
        &self,
        ch: char,
        gid: u16,
        style: MathStyle,
        context: MathGsubContext,
    ) -> Result<MathBox, Error> {
        let glyph_id = self
            .font
            .math_gsub_glyph_id(gid, style.script_level(), context);
        let g = self.font.glyph_id(ch, glyph_id)?;
        let scale = self.params.scale(style);
        let italic = self.font.italic_correction(glyph_id);

        Ok(MathBox {
            width: g.advance.checked_mul(&scale)?,
            height: g.height.checked_mul(&scale)?,
            depth: g.depth.checked_mul(&scale)?,
            italic: italic.checked_mul(&scale)?,
            shift: Dim::zero(),
            content: BoxContent::glyph(ch, glyph_id, scale),
        })
    }

    fn space_dim(&self, kind: &SpaceKind, style: MathStyle) -> Result<Dim, Error> {
        let mu = self.params.mu(style)?;
        match kind {
            SpaceKind::Thin => Ok(mu.checked_mul(&Dim::from_i64(3))?),
            SpaceKind::Medium => Ok(mu.checked_mul(&Dim::from_i64(4))?),
            SpaceKind::Thick => Ok(mu.checked_mul(&Dim::from_i64(5))?),
            SpaceKind::NegThin => Ok(-mu.checked_mul(&Dim::from_i64(3))?),
            SpaceKind::Quad => Ok(self.params.em(style)?),
            SpaceKind::Qquad => Ok(self.params.em(style)?.checked_mul(&Dim::from_i64(2))?),
            SpaceKind::ControlSpace => Ok(self.params.em(style)?.checked_div(&Dim::from_i64(3))?),
            SpaceKind::Hspace(length) => Ok(self.resolve_length(length, style)?),
        }
    }

    fn math_alphabet_run(&self, s: &str, ts: TextStyle, style: MathStyle) -> Result<Item, Error> {
        if ts == TextStyle::Pmb {
            return self.pmb(s, style);
        }
        let mut kids = Vec::new();
        for c in s.chars() {
            if c == ' ' {
                kids.push(MathBox::kern(
                    self.params.mu(style)?.checked_mul(&Dim::from_i64(4))?,
                ));
            } else {
                kids.push(self.glyph(styled_char(c, ts), style)?);
            }
        }
        Ok(Item {
            bx: MathBox::hpack(kids)?,
            class: Some(AtomKind::Ord),
        })
    }

    fn literal_text_run(&self, s: &str, style: MathStyle) -> Result<Item, Error> {
        let mut kids = Vec::with_capacity(s.chars().count());
        for ch in s.chars() {
            kids.push(self.literal_glyph(ch, style)?);
        }
        Ok(Item {
            bx: MathBox::hpack(kids)?,
            class: Some(AtomKind::Ord),
        })
    }

    fn literal_glyph(&self, ch: char, style: MathStyle) -> Result<MathBox, Error> {
        let Some(glyph_id) = self.font.glyph_index(ch) else {
            return self.missing_glyph(ch, style);
        };
        let glyph = self.font.glyph_id(ch, glyph_id)?;
        let scale = self.params.scale(style);
        Ok(MathBox {
            width: glyph.advance.checked_mul(&scale)?,
            height: glyph.height.checked_mul(&scale)?,
            depth: glyph.depth.checked_mul(&scale)?,
            italic: Dim::zero(),
            shift: Dim::zero(),
            content: BoxContent::glyph(ch, glyph.glyph_id, scale),
        })
    }

    fn pmb(&self, s: &str, style: MathStyle) -> Result<Item, Error> {
        let base = self.math_alphabet_run(s, TextStyle::Rm, style)?;
        let dx = self.params.em(style)?.checked_div(&Dim::from_i64(25))?;
        let shifted = MathBox::hpack(vec![MathBox::kern(dx.clone()), base.bx.clone()])?;
        Ok(Item {
            class: Some(AtomKind::Ord),
            bx: MathBox {
                width: base.bx.width.checked_add(&dx)?,
                height: base.bx.height.clone(),
                depth: base.bx.depth.clone(),
                italic: Dim::zero(),
                shift: Dim::zero(),
                content: BoxContent::Overlap(vec![base.bx, shifted]),
            },
        })
    }

    fn operator_noad(&self, semantics: OperatorSemantics<'_>) -> Result<Item, Error> {
        let OperatorSemantics {
            nucleus,
            lower,
            upper,
            style,
            lower_style,
            upper_style,
            placement,
        } = semantics;

        match nucleus {
            OperatorNucleus::Integral(kind) => self.integral_op(
                kind,
                lower,
                upper,
                style,
                (lower_style, upper_style),
                placement,
            ),
            OperatorNucleus::Named(name) => {
                if let Some(ch) = single_glyph(name) {
                    if !ch.is_ascii_alphabetic() {
                        return self.large_op(
                            ch,
                            lower,
                            upper,
                            style,
                            (lower_style, upper_style),
                            placement,
                        );
                    }
                }

                let op = self.math_alphabet_run(name, TextStyle::Rm, style)?.bx;
                if lower.is_none() && upper.is_none() {
                    return Ok(Item {
                        bx: op,
                        class: Some(AtomKind::Op),
                    });
                }
                self.attach_limits(
                    op,
                    upper,
                    lower,
                    style,
                    (lower_style, upper_style),
                    placement == LimitPlacement::Limits,
                )
            }
        }
    }

    fn row(&self, items: &[MathNode], style: MathStyle) -> Result<Item, Error> {
        if items.is_empty() {
            return Ok(Item {
                bx: MathBox::empty(),
                class: Some(AtomKind::Ord),
            });
        }

        let sequence = normalize_row(items, style);
        let mut noad_classes = sequence.iter().filter_map(|item| match item {
            SemanticItem::Noad { class, .. } => Some(*class),
            _ => None,
        });
        let first_class = noad_classes.next();
        let row_class = if first_class.is_some() && noad_classes.next().is_none() {
            first_class
        } else {
            Some(AtomKind::Ord)
        };
        let mut current_style = style;
        let mut previous_noad = None;
        let mut out = Vec::new();

        for semantic in sequence {
            match semantic {
                SemanticItem::Style(next_style) => current_style = next_style,
                SemanticItem::Glue(kind) => {
                    out.push(MathBox::kern(self.space_dim(kind, current_style)?));
                }
                SemanticItem::Control(node) => {
                    out.push(self.item(node, current_style)?.bx);
                }
                SemanticItem::Noad { node, class } => {
                    if let Some(left) = previous_noad {
                        let mu = atom_space_mu(left, class, current_style);
                        let width = space_width(mu, &self.params, current_style)?;
                        if !width.is_zero() {
                            out.push(MathBox::kern(width));
                        }
                    }

                    let laid = self.item(node, current_style)?;
                    if row_needs_math_italic_kern(node) && !laid.bx.italic.is_zero() {
                        out.push(laid.bx.clone());
                        out.push(MathBox::kern(laid.bx.italic));
                    } else {
                        out.push(laid.bx);
                    }
                    previous_noad = Some(class);
                }
            }
        }

        Ok(Item {
            bx: MathBox::hpack(out)?,
            class: row_class,
        })
    }

    fn clean_math_component(&self, node: &MathNode, style: MathStyle) -> Result<MathBox, Error> {
        let bx = self.layout(node, style)?;
        if row_needs_math_italic_kern(node) && !bx.italic.is_zero() {
            let italic = bx.italic.clone();
            Ok(MathBox::hpack(vec![bx, MathBox::kern(italic)])?)
        } else {
            Ok(bx)
        }
    }

    fn clean_script_component(&self, node: &MathNode, style: MathStyle) -> Result<MathBox, Error> {
        let mut bx = self.layout(node, style)?;
        if row_needs_math_italic_kern(node) && !bx.italic.is_zero() {
            // TeX clean_box keeps a terminal math-character italic correction
            // in the script box width. Preserve direct Glyph content here so
            // MathKern can still inspect the script glyph after cleaning.
            bx.width = bx.width.checked_add(&bx.italic)?;
            bx.italic = Dim::zero();
        }
        Ok(bx)
    }

    fn over_under(
        &self,
        base: &MathNode,
        over: Option<&MathNode>,
        under: Option<&MathNode>,
        style: MathStyle,
        class: AtomKind,
    ) -> Result<Item, Error> {
        let (under_style, over_style) = script_styles(style);
        // amsmath implements overset/underset as a boxed math operator with limits.
        // Stackrel uses the same geometry but wraps the result in a relation. Measure
        // each participant as a complete math list before stretching/centering so
        // terminal math italic is materialized exactly once.
        let mut base_box = self.clean_math_component(base, style)?;
        let over_box = match over {
            Some(node) => Some(self.clean_math_component(node, over_style)?),
            None => None,
        };
        let under_box = match under {
            Some(node) => Some(self.clean_math_component(node, under_style)?),
            None => None,
        };
        let mut needed = base_box.width.clone();
        if let Some(ref bx) = over_box {
            needed = needed.max_ref(&bx.width);
        }
        if let Some(ref bx) = under_box {
            needed = needed.max_ref(&bx.width);
        }
        base_box = self.stretch_h(base_box, &needed, style)?;
        let mut item = self.attach_limit_boxes(base_box, over_box, under_box, style)?;
        item.class = Some(class);
        Ok(item)
    }

    fn generalized_fraction(&self, spec: &FractionSpec, style: MathStyle) -> Result<Item, Error> {
        let fraction_style = match spec.style {
            FractionStyle::Inherit => style,
            FractionStyle::Display => MathStyle::Display,
            FractionStyle::Text => MathStyle::Text,
            FractionStyle::Script => MathStyle::Script,
            FractionStyle::ScriptScript => MathStyle::ScriptScript,
        };

        let num_b = self.clean_math_component(&spec.numerator, fraction_style.numerator())?;
        let den_b = self.clean_math_component(&spec.denominator, fraction_style.denominator())?;
        let scale = self.params.scale(fraction_style);
        let axis = self.params.axis_height.checked_mul(&scale)?;

        let rule_thickness = match &spec.rule {
            FractionRule::None => None,
            FractionRule::Default => Some(self.params.fraction_rule_thickness.checked_mul(&scale)?),
            FractionRule::Exact(length) => {
                let thickness = self.resolve_length(length, fraction_style)?;
                if thickness < Dim::zero() {
                    return Err(Error::Malformed {
                        what: "negative fraction rule thickness".into(),
                    });
                }
                if thickness.is_zero() {
                    None
                } else {
                    Some(thickness)
                }
            }
        };

        let (num_shift, den_shift, rule_layer) = if let Some(thick) = rule_thickness {
            let half = thick.checked_div(&Dim::from_i64(2))?;
            let (shift_up0, shift_dn0, gap_num, gap_den) = if fraction_style.is_display() {
                (
                    self.params
                        .fraction_numerator_display_style_shift_up
                        .checked_mul(&scale)?,
                    self.params
                        .fraction_denominator_display_style_shift_down
                        .checked_mul(&scale)?,
                    self.params
                        .fraction_num_display_style_gap_min
                        .checked_mul(&scale)?,
                    self.params
                        .fraction_denom_display_style_gap_min
                        .checked_mul(&scale)?,
                )
            } else {
                (
                    self.params
                        .fraction_numerator_shift_up
                        .checked_mul(&scale)?,
                    self.params
                        .fraction_denominator_shift_down
                        .checked_mul(&scale)?,
                    self.params.fraction_numerator_gap_min.checked_mul(&scale)?,
                    self.params
                        .fraction_denominator_gap_min
                        .checked_mul(&scale)?,
                )
            };
            let num_floor = axis
                .checked_add(&half)?
                .checked_add(&gap_num)?
                .checked_add(&num_b.depth)?;
            let den_floor = den_b
                .height
                .checked_add(&gap_den)?
                .checked_add(&half)?
                .checked_sub(&axis)?
                .clamp_nonneg();
            let bar_shift = axis.checked_sub(&half)?;
            (
                shift_up0.max_ref(&num_floor),
                shift_dn0.max_ref(&den_floor),
                Some((thick, bar_shift)),
            )
        } else {
            let (mut num_shift, mut den_shift, gap_min) =
                self.fraction_stack.scaled(fraction_style, &scale)?;
            let actual_gap = num_shift
                .checked_sub(&num_b.depth)?
                .checked_add(&den_shift)?
                .checked_sub(&den_b.height)?;
            if actual_gap < gap_min {
                let correction = gap_min
                    .checked_sub(&actual_gap)?
                    .checked_div(&Dim::from_i64(2))?;
                num_shift = num_shift.checked_add(&correction)?;
                den_shift = den_shift.checked_add(&correction)?;
            }
            (num_shift, den_shift, None)
        };

        let content_width = num_b.width.max_ref(&den_b.width);
        let num_c = match spec.numerator_alignment {
            FractionAlignment::Default | FractionAlignment::Center => {
                center_in(num_b, &content_width)?
            }
            FractionAlignment::Left => align_in(num_b, &content_width, ColSpec::Left)?,
            FractionAlignment::Right => align_in(num_b, &content_width, ColSpec::Right)?,
        };
        let den_c = center_in(den_b, &content_width)?;
        let num_h = num_c.height.clone();
        let den_d = den_c.depth.clone();
        let mut layers = vec![num_c.with_shift(num_shift.clone())];
        if let Some((thick, bar_shift)) = rule_layer {
            if !thick.is_zero() {
                layers.push(
                    MathBox::rule(content_width.clone(), thick, Dim::zero()).with_shift(bar_shift),
                );
            }
        }
        layers.push(den_c.with_shift(-den_shift.clone()));
        let inner = MathBox {
            width: content_width,
            height: num_shift.checked_add(&num_h)?,
            depth: den_shift.checked_add(&den_d)?,
            italic: Dim::zero(),
            shift: Dim::zero(),
            content: BoxContent::Overlap(layers),
        };

        let needed = self.delimiter_target(&inner.height, &inner.depth, &axis)?;
        let left = if spec.left_delimiter == Delimiter::Empty {
            MathBox::kern(self.null_delimiter_space.clone())
        } else {
            self.center_delimiter(
                self.delim_box(&spec.left_delimiter, &needed, fraction_style)?,
                &axis,
            )?
        };
        let right = if spec.right_delimiter == Delimiter::Empty {
            MathBox::kern(self.null_delimiter_space.clone())
        } else {
            self.center_delimiter(
                self.delim_box(&spec.right_delimiter, &needed, fraction_style)?,
                &axis,
            )?
        };
        Ok(Item {
            class: Some(AtomKind::Inner),
            bx: shifted_hpack(vec![left, inner, right])?,
        })
    }

    fn radical(
        &self,
        deg: Option<&MathNode>,
        rad: &MathNode,
        style: MathStyle,
    ) -> Result<Item, Error> {
        let rad_b = self.layout(rad, style.cramp())?;
        // A direct math-character radicand retains its MATH italic correction
        // in `MathBox::italic`, while row packing materializes the same terminal
        // correction as an explicit kern and clears the packed box italic. The
        // radical boundary needs the complete horizontal extent exactly once so
        // the overbar and outer width agree for both representations.
        let radicand_width = rad_b.width.checked_add(&rad_b.italic)?;
        let s = self.params.scale(style);
        let thick = self.params.radical_rule_thickness.checked_mul(&s)?;
        let extra = self.params.radical_extra_ascender.checked_mul(&s)?;
        let gap = if style.is_display() {
            self.params
                .radical_display_style_vertical_gap
                .checked_mul(&s)?
        } else {
            self.params.radical_vertical_gap.checked_mul(&s)?
        };
        // RadicalExtraAscender reserves whitespace above the
        // finished radical; it is not part of the minimum span passed to MathVariants.
        let needed = rad_b
            .height
            .checked_add(&rad_b.depth)?
            .checked_add(&gap)?
            .checked_add(&thick)?;
        let mut surd = self.sized_glyph('√', &needed, style)?;
        let surd_span = surd.height.checked_add(&surd.depth)?;
        let radicand_span = rad_b.height.checked_add(&rad_b.depth)?;

        // A ready-made radical variant can be taller
        // than the minimum request. Redistribute that discrete slack into
        // the vertical gap instead of lowering the entire surd to the
        // original rule position.
        let distributed_gap = surd_span
            .checked_sub(&thick)?
            .checked_sub(&radicand_span)?
            .checked_add(&gap)?
            .checked_div(&Dim::from_i64(2))?;
        let gap = gap.max_ref(&distributed_gap);
        let inner_ascent = rad_b.height.checked_add(&gap)?.checked_add(&thick)?;
        let surd_descent = surd_span.checked_sub(&inner_ascent)?.clamp_nonneg();
        surd.shift = inner_ascent.checked_sub(&surd.height)?;

        let bar = MathBox::rule(radicand_width.clone(), thick.clone(), Dim::zero())
            .with_shift(rad_b.height.checked_add(&gap)?);
        let rad_col = MathBox {
            width: radicand_width,
            height: inner_ascent.checked_add(&extra)?,
            depth: rad_b.depth.clone(),
            italic: Dim::zero(),
            shift: Dim::zero(),
            content: BoxContent::Overlap(vec![bar, rad_b.clone()]),
        };
        let mut kids = vec![surd, rad_col];
        let mut width = MathBox::hpack(vec![kids[0].clone(), kids[1].clone()])?.width;
        let mut height = inner_ascent.checked_add(&extra)?;
        let mut depth = rad_b.depth.max_ref(&surd_descent);
        if let Some(d) = deg {
            let db = self.layout(d, MathStyle::ScriptScript)?;
            let before = self.params.radical_kern_before_degree.checked_mul(&s)?;
            let configured_after = self.params.radical_kern_after_degree.checked_mul(&s)?;
            // LuaTeX clamps a negative after-degree kern so a narrow degree can
            // never pull the radical sign left of the degree origin. Express the
            // same constraint directly: before + degree + after >= 0.
            let minimum_after = -before.checked_add(&db.width)?;
            let after = configured_after.max_ref(&minimum_after);
            let pct = Dim::from_i64(i64::from(self.params.radical_degree_bottom_raise_percent))
                .checked_div(&Dim::from_i64(100))?;

            // Measure the configured percentage
            // upward from the bottom of the complete shifted radical sign.
            let raise = surd_span.checked_mul(&pct)?.checked_sub(&surd_descent)?;
            height = height.max_ref(&db.height.checked_add(&raise)?);
            depth = depth.max_ref(&db.depth.checked_sub(&raise)?.clamp_nonneg());
            let deg_box = db.with_shift(raise);
            kids = vec![
                MathBox::kern(before),
                deg_box,
                MathBox::kern(after),
                kids[0].clone(),
                kids[1].clone(),
            ];
            width = MathBox::hpack(kids.clone())?.width;
        }
        Ok(Item {
            class: Some(AtomKind::Inner),
            bx: MathBox {
                width,
                height,
                depth,
                italic: Dim::zero(),
                shift: Dim::zero(),
                content: BoxContent::HList(kids),
            },
        })
    }

    fn scripts(&self, semantics: ScriptSemantics<'_>) -> Result<Item, Error> {
        // LuaTeX attaches scripts to a direct character nucleus before closing
        // the accent noad. Keep the accent target/attachment anchored to that
        // original character while the scripted body owns the final width.
        if let MathNode::Accent(base, kind) = semantics.base {
            if is_scripted_character_accent(*kind) {
                let nucleus_style = semantics.style.cramp();
                if let Some(nucleus) = self.direct_accent_nucleus(base, *kind, nucleus_style)? {
                    let scripted = self.attach_scripts_to_box(
                        nucleus.clone(),
                        noad_class(base),
                        semantics.sub,
                        semantics.sup,
                        semantics.style,
                        (semantics.sub_style, semantics.sup_style),
                    )?;
                    let width = scripted.bx.width.clone();
                    return Ok(Item {
                        class: Some(AtomKind::Ord),
                        bx: self.math_accent_box(
                            nucleus,
                            scripted.bx,
                            *kind,
                            semantics.style,
                            width,
                        )?,
                    });
                }
            }
        }

        let base_it = self.item(semantics.base, semantics.style)?;
        self.attach_scripts_to_box(
            base_it.bx,
            base_it.class,
            semantics.sub,
            semantics.sup,
            semantics.style,
            (semantics.sub_style, semantics.sup_style),
        )
    }

    fn attach_scripts_to_box(
        &self,
        base: MathBox,
        class: Option<AtomKind>,
        sub: Option<&MathNode>,
        sup: Option<&MathNode>,
        style: MathStyle,
        script_styles: (MathStyle, MathStyle),
    ) -> Result<Item, Error> {
        let (sub_style, sup_style) = script_styles;
        if sub.is_none() && sup.is_none() {
            return Ok(Item { bx: base, class });
        }

        let s = self.params.scale(style);
        let base_glyph = direct_glyph(&base);
        let direct_character_nucleus = base_glyph.is_some();
        let base_uses_ink_box = match base_glyph {
            Some((glyph_id, _)) => self.font.is_extended_shape(glyph_id),
            None => true,
        };
        let after = self.params.space_after_script.checked_mul(&s)?;

        let mut sup_shift = Dim::zero();
        let mut sub_shift = Dim::zero();
        let sup_laid = if let Some(e) = sup {
            sup_shift = if style.is_cramped() {
                self.params.superscript_shift_up_cramped.checked_mul(&s)?
            } else {
                self.params.superscript_shift_up.checked_mul(&s)?
            };
            Some(self.clean_script_component(e, sup_style)?)
        } else {
            None
        };
        let sub_laid = if let Some(u) = sub {
            sub_shift = self.params.subscript_shift_down.checked_mul(&s)?;
            Some(self.clean_script_component(u, sub_style)?)
        } else {
            None
        };

        if let Some(sp) = &sup_laid {
            let min_bottom = self
                .script_placement
                .superscript_bottom_min
                .checked_mul(&s)?;
            sup_shift = sup_shift.max_ref(&effective_depth(sp)?.checked_add(&min_bottom)?);

            if base_uses_ink_box {
                let base_top = base.height.checked_add(&base.shift)?.clamp_nonneg();
                let max_drop = self
                    .script_placement
                    .superscript_baseline_drop_max
                    .checked_mul(&s)?;
                sup_shift = sup_shift.max_ref(&base_top.checked_sub(&max_drop)?.clamp_nonneg());
            }
        }

        if let Some(sb) = &sub_laid {
            let max_top = self.script_placement.subscript_top_max.checked_mul(&s)?;
            sub_shift =
                sub_shift.max_ref(&effective_height(sb)?.checked_sub(&max_top)?.clamp_nonneg());

            if base_uses_ink_box {
                let base_depth = base.depth.checked_sub(&base.shift)?.clamp_nonneg();
                let min_drop = self
                    .script_placement
                    .subscript_baseline_drop_min
                    .checked_mul(&s)?;
                sub_shift = sub_shift.max_ref(&base_depth.checked_add(&min_drop)?);
            }
        }

        self.enforce_paired_script_constraints(
            sup_laid.as_ref(),
            sub_laid.as_ref(),
            &s,
            direct_character_nucleus,
            &mut sup_shift,
            &mut sub_shift,
        )?;

        let sup_offset = match &sup_laid {
            Some(sp) => base
                .italic
                .checked_add(&self.superscript_math_kern(&base, sp, &sup_shift)?)?,
            None => Dim::zero(),
        };
        let sub_offset = match &sub_laid {
            Some(sb) => self.subscript_math_kern(&base, sb, &sub_shift)?,
            None => Dim::zero(),
        };

        let mut common_offset = Dim::zero();
        if sup_laid.is_some() {
            common_offset = common_offset.min_ref(&sup_offset);
        }
        if sub_laid.is_some() {
            common_offset = common_offset.min_ref(&sub_offset);
        }

        let mut slot_w = (-common_offset.clone()).clamp_nonneg();
        let mut slot_h = Dim::zero();
        let mut slot_d = Dim::zero();
        let mut slot_kids = Vec::new();

        if let Some(sp) = sup_laid {
            let lead = sup_offset.checked_sub(&common_offset)?;
            let mut branch = position_script(sp, lead)?;
            let branch_shift = branch.shift.checked_add(&sup_shift)?;
            slot_w = slot_w.max_ref(&branch.width);
            branch.shift = branch_shift;
            slot_h = slot_h.max_ref(&effective_height(&branch)?);
            slot_d = slot_d.max_ref(&effective_depth(&branch)?);
            slot_kids.push(branch);
        }
        if let Some(sb) = sub_laid {
            let lead = sub_offset.checked_sub(&common_offset)?;
            let mut branch = position_script(sb, lead)?;
            let branch_shift = branch.shift.checked_sub(&sub_shift)?;
            slot_w = slot_w.max_ref(&branch.width);
            branch.shift = branch_shift;
            slot_h = slot_h.max_ref(&effective_height(&branch)?);
            slot_d = slot_d.max_ref(&effective_depth(&branch)?);
            slot_kids.push(branch);
        }

        // TeX82 make_op removes an operator nucleus italic correction from
        // the common width when a subscript is present without stacked limits,
        // while retaining that correction as the horizontal separation between
        // the superscript and subscript origins. Keep the shared G7 attachment
        // machinery, but backtrack the whole script slot by the operator italic
        // correction before applying the independent super/sub offsets.
        let operator_backtrack = if class == Some(AtomKind::Op) && sub.is_some() {
            -base.italic.clone()
        } else {
            Dim::zero()
        };

        let mut kids = vec![base.clone()];
        if !operator_backtrack.is_zero() {
            kids.push(MathBox::kern(operator_backtrack.clone()));
        }
        if !common_offset.is_zero() {
            kids.push(MathBox::kern(common_offset.clone()));
        }
        kids.push(MathBox {
            width: slot_w.clone(),
            height: slot_h.clone(),
            depth: slot_d.clone(),
            italic: Dim::zero(),
            shift: Dim::zero(),
            content: BoxContent::Overlap(slot_kids),
        });
        if !after.is_zero() {
            kids.push(MathBox::kern(after.clone()));
        }

        let width = base
            .width
            .checked_add(&operator_backtrack)?
            .checked_add(&common_offset)?
            .checked_add(&slot_w)?
            .checked_add(&after)?;
        Ok(Item {
            class,
            bx: MathBox {
                width,
                height: base
                    .height
                    .checked_add(&base.shift)?
                    .clamp_nonneg()
                    .max_ref(&slot_h),
                depth: base
                    .depth
                    .checked_sub(&base.shift)?
                    .clamp_nonneg()
                    .max_ref(&slot_d),
                italic: Dim::zero(),
                shift: Dim::zero(),
                content: BoxContent::HList(kids),
            },
        })
    }

    fn box_math_kern(
        &self,
        math_box: &MathBox,
        corner: MathKernCorner,
        correction_height: &Dim,
    ) -> Result<Dim, Error> {
        let Some((glyph_id, scale)) = direct_glyph(math_box) else {
            return Ok(Dim::zero());
        };
        Ok(self
            .font
            .math_kern(glyph_id, corner, correction_height, scale)?)
    }

    fn superscript_math_kern(
        &self,
        base: &MathBox,
        script: &MathBox,
        shift_up: &Dim,
    ) -> Result<Dim, Error> {
        // OpenType evaluates each side at the same physical horizontal line,
        // expressed relative to that glyph's own baseline.
        let base_at_script_bottom = shift_up
            .checked_add(&script.shift)?
            .checked_sub(&script.depth)?
            .checked_sub(&base.shift)?;
        let script_at_its_bottom = -script.depth.clone();
        let first = self
            .box_math_kern(base, MathKernCorner::TopRight, &base_at_script_bottom)?
            .checked_add(&self.box_math_kern(
                script,
                MathKernCorner::BottomLeft,
                &script_at_its_bottom,
            )?)?;

        let base_at_its_top = base.height.clone();
        let script_at_base_top = base
            .shift
            .checked_add(&base.height)?
            .checked_sub(shift_up)?
            .checked_sub(&script.shift)?;
        let second = self
            .box_math_kern(base, MathKernCorner::TopRight, &base_at_its_top)?
            .checked_add(&self.box_math_kern(
                script,
                MathKernCorner::BottomLeft,
                &script_at_base_top,
            )?)?;

        Ok(first.min_ref(&second))
    }

    fn subscript_math_kern(
        &self,
        base: &MathBox,
        script: &MathBox,
        shift_down: &Dim,
    ) -> Result<Dim, Error> {
        // As for superscripts, the two lookups use one shared physical line
        // but correction heights relative to the base and script baselines.
        let base_at_script_top = script
            .height
            .checked_add(&script.shift)?
            .checked_sub(shift_down)?
            .checked_sub(&base.shift)?;
        let script_at_its_top = script.height.clone();
        let first = self
            .box_math_kern(base, MathKernCorner::BottomRight, &base_at_script_top)?
            .checked_add(&self.box_math_kern(
                script,
                MathKernCorner::TopLeft,
                &script_at_its_top,
            )?)?;

        let base_at_its_bottom = -base.depth.clone();
        let script_at_base_bottom = base
            .shift
            .checked_sub(&base.depth)?
            .checked_add(shift_down)?
            .checked_sub(&script.shift)?;
        let second = self
            .box_math_kern(base, MathKernCorner::BottomRight, &base_at_its_bottom)?
            .checked_add(&self.box_math_kern(
                script,
                MathKernCorner::TopLeft,
                &script_at_base_bottom,
            )?)?;

        Ok(first.min_ref(&second))
    }

    fn enforce_paired_script_constraints(
        &self,
        sup: Option<&MathBox>,
        sub: Option<&MathBox>,
        scale: &Dim,
        direct_character_nucleus: bool,
        sup_shift: &mut Dim,
        sub_shift: &mut Dim,
    ) -> Result<(), Error> {
        let (Some(sp), Some(sb)) = (sup, sub) else {
            return Ok(());
        };

        let gap = sup_shift
            .checked_add(sub_shift)?
            .checked_sub(&effective_depth(sp)?)?
            .checked_sub(&effective_height(sb)?)?;
        let min_gap = self.params.sub_superscript_gap_min.checked_mul(scale)?;

        if gap < min_gap {
            *sub_shift = sub_shift.checked_add(&min_gap.checked_sub(&gap)?)?;

            // `SuperscriptBottomMaxWithSubscript` redistributes an actual gap
            // repair; it is not an independent minimum superscript position.
            if direct_character_nucleus {
                let current_bottom = sup_shift.checked_sub(&effective_depth(sp)?)?;
                let target_bottom = self
                    .script_placement
                    .superscript_bottom_max_with_subscript
                    .checked_mul(scale)?;

                if current_bottom < target_bottom {
                    let raise = target_bottom.checked_sub(&current_bottom)?;
                    let lowered_subscript = sub_shift.checked_sub(&raise)?;

                    if lowered_subscript >= Dim::zero() {
                        *sup_shift = sup_shift.checked_add(&raise)?;
                        *sub_shift = lowered_subscript;
                    }
                }
            }
        }
        Ok(())
    }
    fn delimited(
        &self,
        open: &Delimiter,
        body: &MathNode,
        close: &Delimiter,
        style: MathStyle,
    ) -> Result<Item, Error> {
        let body_b = self.layout(body, style)?;
        let s = self.params.scale(style);
        let axis = self.params.axis_height.checked_mul(&s)?;
        let needed = self.delimiter_target(&body_b.height, &body_b.depth, &axis)?;
        // TeX null \left/\right delimiters occupy \nulldelimiterspace;
        // they are not zero-width parser placeholders.
        let left = if matches!(open, Delimiter::Empty) {
            MathBox::kern(self.null_delimiter_space.clone())
        } else {
            self.center_delimiter(self.delim_box(open, &needed, style)?, &axis)?
        };
        let right = if matches!(close, Delimiter::Empty) {
            MathBox::kern(self.null_delimiter_space.clone())
        } else {
            self.center_delimiter(self.delim_box(close, &needed, style)?, &axis)?
        };
        Ok(Item {
            class: Some(AtomKind::Inner),
            bx: shifted_hpack(vec![left, body_b, right])?,
        })
    }

    fn delimiter_target(&self, height: &Dim, depth: &Dim, axis: &Dim) -> Result<Dim, Error> {
        let above = height.checked_sub(axis)?.clamp_nonneg();
        let below = depth.checked_add(axis)?;
        self.delimiter_target_from_max_distance(&above.max_ref(&below))
    }

    fn delimiter_target_from_max_distance(&self, max_distance: &Dim) -> Result<Dim, Error> {
        let factor = Dim::ratio(TEX_DELIMITER_FACTOR_NUM, TEX_DELIMITER_FACTOR_DEN)?;
        let factor_target = max_distance.checked_mul(&factor)?;
        let shortfall_target = max_distance
            .checked_mul(&Dim::from_i64(2))?
            .checked_sub(&self.delimiter_shortfall)?
            .clamp_nonneg();
        Ok(factor_target.max_ref(&shortfall_target))
    }

    fn center_delimiter(&self, mut bx: MathBox, axis: &Dim) -> Result<MathBox, Error> {
        if matches!(bx.content, BoxContent::Empty) {
            return Ok(bx);
        }
        let center = bx
            .height
            .checked_sub(&bx.depth)?
            .checked_div(&Dim::from_i64(2))?;
        bx.shift = axis.checked_sub(&center)?;
        Ok(bx)
    }

    fn explicit_delim_target(&self, size: DelimSize) -> Result<Dim, Error> {
        // amsmath sets \big@size to 1.2 times the textstyle math
        // parenthesis height+depth, then uses factors 1, 1.5, 2, and 2.5
        // for \big, \Big, \bigg, and \Bigg. The resulting vcenter is
        // fed through TeX's ordinary delimiter-factor/shortfall algorithm.
        let paren = self.font.glyph('(')?;
        let paren_span = paren.height.checked_add(&paren.depth)?;
        let (numerator, denominator) = match size {
            DelimSize::Big => (6, 5),
            DelimSize::Big2 => (9, 5),
            DelimSize::Bigg => (12, 5),
            DelimSize::Bigg2 => (3, 1),
        };
        let vcenter_extent = paren_span
            .checked_mul(&Dim::from_i64(numerator))?
            .checked_div(&Dim::from_i64(denominator))?;
        let max_distance = vcenter_extent.checked_div(&Dim::from_i64(2))?;
        self.delimiter_target_from_max_distance(&max_distance)
    }

    fn delim_box(&self, d: &Delimiter, needed: &Dim, style: MathStyle) -> Result<MathBox, Error> {
        match d {
            Delimiter::Empty => Ok(MathBox::empty()),
            Delimiter::Char(c) => self.sized_glyph(*c, needed, style),
            Delimiter::Named(n) => {
                let ch = named_delim(n)?;
                self.sized_glyph(ch, needed, style)
            }
        }
    }

    fn sized_glyph(&self, ch: char, needed: &Dim, style: MathStyle) -> Result<MathBox, Error> {
        let base_glyph_id = self
            .font
            .glyph_index(ch)
            .ok_or(FontError::MissingGlyph { ch })?;
        let mut best = self.glyph_id(ch, base_glyph_id, style)?;
        let mut best_span = best.height.checked_add(&best.depth)?;
        let scale = self.params.scale(style);
        for variant in self.font.vertical_variant_records(base_glyph_id) {
            let candidate_span = variant.advance.checked_mul(&scale)?;
            let candidate = match self.glyph_id(ch, variant.glyph_id, style) {
                Ok(candidate) => candidate,
                Err(Error::Font(FontError::MissingGlyph { .. })) => continue,
                Err(error) => return Err(error),
            };
            if variant_measure_is_better(&best_span, &candidate_span, needed) {
                best_span = candidate_span;
                best = candidate;
            }
        }
        if best_span >= *needed {
            return Ok(best);
        }

        let target = needed.checked_div(&scale)?;
        let assembly = match self.font.vertical_assembly(base_glyph_id) {
            Ok(Some(assembly)) => assembly,
            Ok(None) => return Ok(best),
            Err(_) => {
                self.extensible_fallback(ch);
                return Ok(best);
            }
        };
        let solution = match solve_glyph_assembly(&assembly, &target) {
            Ok(solution) => solution,
            Err(_) => {
                self.extensible_fallback(ch);
                return Ok(best);
            }
        };
        match self.render_vertical_assembly(ch, &solution, style)? {
            Some(assembled) => Ok(assembled),
            None => {
                self.extensible_fallback(ch);
                Ok(best)
            }
        }
    }

    fn stretch_h(&self, base: MathBox, needed: &Dim, style: MathStyle) -> Result<MathBox, Error> {
        let (ch, glyph_id) = match &base.content {
            BoxContent::Glyph { ch, glyph_id, .. } => (*ch, *glyph_id),
            _ => return Ok(base),
        };
        self.stretch_h_from_construction(base, ch, glyph_id, needed, style, MathGsubContext::None)
    }

    fn stretch_h_from_construction(
        &self,
        base: MathBox,
        ch: char,
        construction_glyph_id: u16,
        needed: &Dim,
        style: MathStyle,
        context: MathGsubContext,
    ) -> Result<MathBox, Error> {
        let scale = self.params.scale(style);
        let mut best = base;
        let mut best_extent = best.width.clone();

        for variant in self.font.horizontal_variant_records(construction_glyph_id) {
            let candidate_extent = variant.advance.checked_mul(&scale)?;
            let candidate = match self.glyph_id_with_context(ch, variant.glyph_id, style, context) {
                Ok(candidate) => candidate,
                Err(Error::Font(FontError::MissingGlyph { .. })) => continue,
                Err(error) => return Err(error),
            };
            if variant_measure_is_better(&best_extent, &candidate_extent, needed) {
                best_extent = candidate_extent;
                best = candidate;
            }
        }
        if best_extent >= *needed {
            return Ok(best);
        }

        let target = needed.checked_div(&scale)?;
        let assembly = match self.font.horizontal_assembly(construction_glyph_id) {
            Ok(Some(assembly)) => assembly,
            Ok(None) => return Ok(best),
            Err(_) => {
                self.extensible_fallback(ch);
                return Ok(best);
            }
        };
        let solution = match solve_glyph_assembly(&assembly, &target) {
            Ok(solution) => solution,
            Err(_) => {
                self.extensible_fallback(ch);
                return Ok(best);
            }
        };
        let assembled_extent = solution.advance.checked_mul(&scale)?;
        match self.render_horizontal_assembly(ch, &solution, style, context)? {
            Some(assembled) if assembled_extent > best_extent => Ok(assembled),
            Some(_) => Ok(best),
            None => {
                self.extensible_fallback(ch);
                Ok(best)
            }
        }
    }

    fn render_horizontal_assembly(
        &self,
        ch: char,
        solution: &AssemblySolution,
        style: MathStyle,
        context: MathGsubContext,
    ) -> Result<Option<MathBox>, Error> {
        let scale = self.params.scale(style);
        let mut children = Vec::with_capacity(solution.placements.len() * 2);
        for placement in &solution.placements {
            let glyph = match self.glyph_id_with_context(ch, placement.glyph_id, style, context) {
                Ok(glyph) => glyph,
                Err(Error::Font(FontError::MissingGlyph { .. })) => return Ok(None),
                Err(error) => return Err(error),
            };
            let desired_advance = placement.advance.checked_mul(&scale)?;
            let adjustment = desired_advance.checked_sub(&glyph.width)?;
            children.push(glyph);
            if !adjustment.is_zero() {
                children.push(MathBox::kern(adjustment));
            }
        }

        let mut result = MathBox::hpack(children)?;
        result.width = solution.advance.checked_mul(&scale)?;
        result.italic = solution.italic_correction.checked_mul(&scale)?;
        Ok(Some(result))
    }

    fn render_vertical_assembly(
        &self,
        ch: char,
        solution: &AssemblySolution,
        style: MathStyle,
    ) -> Result<Option<MathBox>, Error> {
        let scale = self.params.scale(style);
        let mut children = Vec::with_capacity(solution.placements.len());
        let mut origin = Dim::zero();
        let mut width = Dim::zero();

        for placement in &solution.placements {
            let mut glyph = match self.glyph_id(ch, placement.glyph_id, style) {
                Ok(glyph) => glyph,
                Err(Error::Font(FontError::MissingGlyph { .. })) => return Ok(None),
                Err(error) => return Err(error),
            };
            width = width.max_ref(&glyph.width);
            // Vertical MATH assembly parts are ordered bottom-to-top. Aligning
            // each glyph's ink bottom with its part origin mirrors the vertical
            // origin adjustment used by mature OpenType MATH clients and keeps
            // the assembly's advance independent from incidental ink bounds.
            glyph.shift = origin.checked_add(&glyph.depth)?;
            children.push(glyph);
            origin = origin.checked_add(&placement.advance.checked_mul(&scale)?)?;
        }

        let advance = solution.advance.checked_mul(&scale)?;
        if origin != advance {
            return Ok(None);
        }

        // OpenType stores vertical assembly parts bottom-to-top, which is the
        // order used above to solve their origins. `Overlap` children are paint
        // order, while TeX vertical lists traverse top-to-bottom; reverse only
        // after placement so paint order changes without changing geometry.
        children.reverse();

        Ok(Some(MathBox {
            width,
            height: advance,
            depth: Dim::zero(),
            italic: solution.italic_correction.checked_mul(&scale)?,
            shift: Dim::zero(),
            content: BoxContent::Overlap(children),
        }))
    }

    fn extensible_fallback(&self, ch: char) {
        self.diagnostics
            .borrow_mut()
            .push(LayoutDiagnostic::ExtensibleFallback { ch });
    }

    fn large_op(
        &self,
        ch: char,
        lower: Option<&MathNode>,
        upper: Option<&MathNode>,
        style: MathStyle,
        script_styles: (MathStyle, MathStyle),
        placement: LimitPlacement,
    ) -> Result<Item, Error> {
        let (lower_style, upper_style) = script_styles;
        let min_h = if style.is_display() {
            self.params
                .display_operator_min_height
                .checked_mul(&self.params.scale(style))?
        } else {
            Dim::zero()
        };
        let op = self.center_large_operator_on_axis(self.sized_glyph(ch, &min_h, style)?, style)?;
        if placement == LimitPlacement::Side {
            return self.attach_limits(op, upper, lower, style, (lower_style, upper_style), false);
        }

        self.attach_large_op_limits(op, upper, lower, style, upper_style, lower_style)
    }

    fn integral_op(
        &self,
        kind: IntegralKind,
        lower: Option<&MathNode>,
        upper: Option<&MathNode>,
        style: MathStyle,
        script_styles: (MathStyle, MathStyle),
        placement: LimitPlacement,
    ) -> Result<Item, Error> {
        let (lower_style, upper_style) = script_styles;
        let ch = match kind {
            IntegralKind::Int => '∫',
            IntegralKind::Iint => '∬',
            IntegralKind::Iiint => '∭',
            IntegralKind::Oint => '∮',
            IntegralKind::Oiint => '∯',
        };
        let min_h = if style.is_display() {
            self.params
                .display_operator_min_height
                .checked_mul(&self.params.scale(style))?
        } else {
            Dim::zero()
        };
        let op = self.center_large_operator_on_axis(self.sized_glyph(ch, &min_h, style)?, style)?;

        if placement == LimitPlacement::Limits {
            return self.attach_large_op_limits(op, upper, lower, style, upper_style, lower_style);
        }

        self.attach_limits(op, upper, lower, style, (lower_style, upper_style), false)
    }

    fn center_large_operator_on_axis(
        &self,
        mut op: MathBox,
        style: MathStyle,
    ) -> Result<MathBox, Error> {
        let axis = self
            .params
            .axis_height
            .checked_mul(&self.params.scale(style))?;
        let center = op
            .height
            .checked_sub(&op.depth)?
            .checked_div(&Dim::from_i64(2))?;
        op.shift = axis.checked_sub(&center)?;
        Ok(op)
    }

    fn attach_large_op_limits(
        &self,
        op: MathBox,
        over: Option<&MathNode>,
        under: Option<&MathNode>,
        style: MathStyle,
        over_style: MathStyle,
        under_style: MathStyle,
    ) -> Result<Item, Error> {
        let s = self.params.scale(style);
        let op_shift = op.shift.clone();
        let op_h = op.height.checked_add(&op_shift)?.clamp_nonneg();
        let op_d = op.depth.checked_sub(&op_shift)?.clamp_nonneg();

        let over_b = match over {
            Some(o) => Some(self.layout(o, over_style)?),
            None => None,
        };
        let under_b = match under {
            Some(u) => Some(self.layout(u, under_style)?),
            None => None,
        };

        let half_italic = op.italic.checked_div(&Dim::from_i64(2))?;
        let mut width = op.width.clone();
        if let Some(ref o) = over_b {
            width = width.max_ref(&o.width);
        }
        if let Some(ref u) = under_b {
            width = width.max_ref(&u.width);
        }

        let mut height = op_h.clone();
        let mut depth = op_d.clone();
        let mut centered_source = op;
        centered_source.shift = Dim::zero();
        let mut centered_op = center_in(centered_source, &width)?;
        centered_op.shift = op_shift;
        let mut kids = Vec::with_capacity(3);

        if let Some(ob) = over_b {
            let gap = self.params.upper_limit_gap_min.checked_mul(&s)?;
            let rise = self.params.upper_limit_baseline_rise_min.checked_mul(&s)?;

            // Baseline-rise and edge-gap minima
            // are independent constraints. Limit depth participates only
            // in the edge-gap inequality.
            let over_height = effective_height(&ob)?;
            let over_depth = effective_depth(&ob)?;
            let offset = rise.max_ref(&gap.checked_add(&over_depth)?);
            let sh = op_h.checked_add(&offset)?;
            height = height.max_ref(&sh.checked_add(&over_height)?);
            kids.push(center_in_with_offset(ob, &width, &half_italic)?.with_shift(sh));
        }

        kids.push(centered_op);

        if let Some(ub) = under_b {
            let gap = self.params.lower_limit_gap_min.checked_mul(&s)?;
            let drop = self.params.lower_limit_baseline_drop_min.checked_mul(&s)?;

            // The lower relation mirrors the upper relation: limit ascent
            // participates only in the edge-gap inequality.
            let under_height = effective_height(&ub)?;
            let under_depth = effective_depth(&ub)?;
            let offset = drop.max_ref(&gap.checked_add(&under_height)?);
            let sh = op_d.checked_add(&offset)?;
            depth = depth.max_ref(&sh.checked_add(&under_depth)?);
            kids.push(center_in_with_offset(ub, &width, &(-half_italic.clone()))?.with_shift(-sh));
        }

        Ok(Item {
            class: Some(AtomKind::Op),
            bx: MathBox {
                width,
                height,
                depth,
                italic: Dim::zero(),
                shift: Dim::zero(),
                content: BoxContent::Overlap(kids),
            },
        })
    }

    fn attach_limits(
        &self,
        op: MathBox,
        over: Option<&MathNode>,
        under: Option<&MathNode>,
        style: MathStyle,
        script_styles: (MathStyle, MathStyle),
        as_limits: bool,
    ) -> Result<Item, Error> {
        let (under_style, over_style) = script_styles;
        if !as_limits {
            return self.attach_scripts_to_box(
                op,
                Some(AtomKind::Op),
                under,
                over,
                style,
                (under_style, over_style),
            );
        }
        let over_b = match over {
            Some(o) => Some(self.clean_math_component(o, over_style)?),
            None => None,
        };
        let under_b = match under {
            Some(u) => Some(self.clean_math_component(u, under_style)?),
            None => None,
        };
        self.attach_limit_boxes(op, over_b, under_b, style)
    }

    fn attach_limit_boxes(
        &self,
        op: MathBox,
        over_b: Option<MathBox>,
        under_b: Option<MathBox>,
        style: MathStyle,
    ) -> Result<Item, Error> {
        let s = self.params.scale(style);
        let op_h = effective_height(&op)?;
        let op_d = effective_depth(&op)?;
        let mut width = op.width.clone();
        if let Some(ref o) = over_b {
            width = width.max_ref(&o.width);
        }
        if let Some(ref u) = under_b {
            width = width.max_ref(&u.width);
        }
        let mut height = op_h.clone();
        let mut depth = op_d.clone();
        let centered_op = center_in(op, &width)?;
        let mut kids = Vec::with_capacity(3);
        if let Some(ob) = over_b {
            let gap = self.params.upper_limit_gap_min.checked_mul(&s)?;
            let rise = self.params.upper_limit_baseline_rise_min.checked_mul(&s)?;
            let over_height = effective_height(&ob)?;
            let over_depth = effective_depth(&ob)?;
            let offset = rise.max_ref(&gap.checked_add(&over_depth)?);
            let sh = op_h.checked_add(&offset)?;
            height = height.max_ref(&sh.checked_add(&over_height)?);
            kids.push(center_in(ob, &width)?.with_shift(sh));
        }
        kids.push(centered_op);
        if let Some(ub) = under_b {
            let gap = self.params.lower_limit_gap_min.checked_mul(&s)?;
            let drop = self.params.lower_limit_baseline_drop_min.checked_mul(&s)?;
            let under_height = effective_height(&ub)?;
            let under_depth = effective_depth(&ub)?;
            let offset = drop.max_ref(&gap.checked_add(&under_height)?);
            let sh = op_d.checked_add(&offset)?;
            depth = depth.max_ref(&sh.checked_add(&under_depth)?);
            kids.push(center_in(ub, &width)?.with_shift(-sh));
        }
        Ok(Item {
            class: Some(AtomKind::Op),
            bx: MathBox {
                width,
                height,
                depth,
                italic: Dim::zero(),
                shift: Dim::zero(),
                content: BoxContent::Overlap(kids),
            },
        })
    }

    fn not_overlay(&self, base: &MathNode, b: MathBox, style: MathStyle) -> Result<Item, Error> {
        let slash = match self.strict_glyph('\u{0338}', style) {
            Ok(bx) => bx,
            Err(_) => self.strict_glyph('/', style)?,
        };
        let width = b.width.max_ref(&slash.width);
        let two = Dim::from_i64(2);
        let b_axis = b.height.checked_sub(&b.depth)?.checked_div(&two)?;
        let s_axis = slash.height.checked_sub(&slash.depth)?.checked_div(&two)?;
        let raise = b_axis.checked_sub(&s_axis)?;
        let slash_top = slash.height.checked_add(&raise)?;
        let height = b.height.max_ref(&slash_top.max_ref(&slash.height));
        let depth = b
            .depth
            .max_ref(&slash.depth.checked_sub(&raise)?.clamp_nonneg());
        Ok(Item {
            class: noad_class(base),
            bx: MathBox {
                width: width.clone(),
                height,
                depth,
                italic: Dim::zero(),
                shift: Dim::zero(),
                content: BoxContent::Overlap(vec![
                    center_in(b, &width)?,
                    center_in(slash, &width)?.with_shift(raise),
                ]),
            },
        })
    }

    fn accent(&self, base: &MathNode, kind: AccentKind, style: MathStyle) -> Result<Item, Error> {
        if kind == AccentKind::Boxed {
            // amsmath defines \boxed through \fbox around a fresh displaystyle
            // math formula. As with an ordinary math hbox, terminal math italic
            // is not appended merely because the final item is a math character.
            let inner = self.layout(base, MathStyle::Display)?;
            return Ok(Item {
                class: Some(AtomKind::Ord),
                bx: self.framed_color_box(inner, None, None)?,
            });
        }
        if matches!(
            kind,
            AccentKind::Cancel | AccentKind::BCancel | AccentKind::XCancel
        ) {
            // cancel.sty measures an ordinary math hbox and overlays its marks
            // without changing horizontal spacing or appending terminal italic.
            let b = self.layout(base, style)?;
            return Ok(Item {
                class: Some(AtomKind::Ord),
                bx: self.cancel_box(b, kind)?,
            });
        }

        let nucleus_style = if is_tex_accent(kind) {
            style.cramp()
        } else {
            style
        };
        let b = if uses_dotless_accent_base(kind) {
            self.accent_nucleus(base, kind, nucleus_style)?
        } else {
            self.layout(base, nucleus_style)?
        };
        if kind == AccentKind::Not {
            return self.not_overlay(base, b, style);
        }
        if matches!(kind, AccentKind::Overline | AccentKind::Underline) {
            return self.bar_rule(b, kind == AccentKind::Underline, style);
        }

        let width = accent_nucleus_width(&b)?;
        Ok(Item {
            class: Some(AtomKind::Ord),
            bx: self.math_accent_box(b.clone(), b, kind, style, width)?,
        })
    }

    fn math_accent_box(
        &self,
        anchor: MathBox,
        body: MathBox,
        kind: AccentKind,
        style: MathStyle,
        width: Dim,
    ) -> Result<MathBox, Error> {
        let nucleus_style = style.cramp();
        let flatten = is_diacritic_accent(kind)
            && anchor.height
                > self
                    .params
                    .flattened_accent_base_height
                    .checked_mul(&self.params.scale(nucleus_style))?;
        let accent_context = if flatten {
            MathGsubContext::FlattenedAccent
        } else {
            MathGsubContext::None
        };
        let (mut acc, construction_glyph_id) = self.accent_glyph(kind, style, accent_context)?;

        if is_stretchy_accent(kind) {
            let (ch, _) = single_glyph_source(&acc).ok_or_else(|| Error::Malformed {
                what: "accent source must be a single glyph before stretching".into(),
            })?;
            let target = accent_nucleus_width(&anchor)?;
            acc = self.stretch_h_from_construction(
                acc,
                ch,
                construction_glyph_id,
                &target,
                style,
                accent_context,
            )?;
        }

        let x_off = self.accent_x_off(&anchor, &acc, style)?;
        if is_under_accent(kind) {
            return place_under_accent(body, acc, x_off, width);
        }

        let raise = self.accent_raise(&anchor, style)?;
        overlay_accent(body, acc, x_off, raise, width)
    }

    fn direct_accent_nucleus(
        &self,
        base: &MathNode,
        kind: AccentKind,
        style: MathStyle,
    ) -> Result<Option<MathBox>, Error> {
        if !uses_dotless_accent_base(kind) {
            return Ok(match base {
                MathNode::Atom(c, _) => Some(self.glyph(math_italic(math_char(*c)), style)?),
                MathNode::Symbol(name) => Some(self.glyph(math_italic(symbol_char(name)?), style)?),
                MathNode::MathAlphabet(text, text_style) if *text_style != TextStyle::Pmb => {
                    let mut chars = text.chars();
                    match (chars.next(), chars.next()) {
                        (Some(ch), None) if ch != ' ' => {
                            Some(self.glyph(styled_char(ch, *text_style), style)?)
                        }
                        _ => None,
                    }
                }
                _ => None,
            });
        }

        Ok(match base {
            MathNode::Atom(c, _) => Some(self.glyph_with_context(
                math_italic(math_char(*c)),
                style,
                MathGsubContext::DotlessAccentBase,
            )?),
            MathNode::Symbol(name) => Some(self.glyph_with_context(
                math_italic(symbol_char(name)?),
                style,
                MathGsubContext::DotlessAccentBase,
            )?),
            MathNode::MathAlphabet(text, text_style) if *text_style != TextStyle::Pmb => {
                let mut chars = text.chars();
                match (chars.next(), chars.next()) {
                    (Some(ch), None) if ch != ' ' => Some(self.glyph_with_context(
                        styled_char(ch, *text_style),
                        style,
                        MathGsubContext::DotlessAccentBase,
                    )?),
                    _ => None,
                }
            }
            _ => None,
        })
    }

    fn accent_nucleus(
        &self,
        base: &MathNode,
        kind: AccentKind,
        style: MathStyle,
    ) -> Result<MathBox, Error> {
        match self.direct_accent_nucleus(base, kind, style)? {
            Some(nucleus) => Ok(nucleus),
            None => self.layout(base, style),
        }
    }

    fn accent_glyph(
        &self,
        kind: AccentKind,
        style: MathStyle,
        context: MathGsubContext,
    ) -> Result<(MathBox, u16), Error> {
        if accent_prefers_math_construction(kind) {
            for &ch in accent_candidates(kind) {
                let Some(glyph_id) = self.font.glyph_index(ch) else {
                    continue;
                };
                if !self.has_horizontal_math_construction(glyph_id) {
                    continue;
                }
                if let Ok(bx) = self.glyph_id_with_context(ch, glyph_id, style, context) {
                    return Ok((bx, glyph_id));
                }
            }
        }

        for &ch in accent_candidates(kind) {
            let Some(glyph_id) = self.font.glyph_index(ch) else {
                continue;
            };
            if let Ok(bx) = self.glyph_id_with_context(ch, glyph_id, style, context) {
                return Ok((bx, glyph_id));
            }
        }
        Err(Error::Unsupported {
            what: format!("accent {}", kind.gold()),
        })
    }

    // Source candidates are semantic alternatives. Prefer a candidate that
    // actually participates in MathVariants before falling back to a merely
    // renderable spacing glyph.
    fn has_horizontal_math_construction(&self, glyph_id: u16) -> bool {
        if !self.font.horizontal_variant_records(glyph_id).is_empty() {
            return true;
        }
        matches!(self.font.horizontal_assembly(glyph_id), Ok(Some(_)))
    }

    // MathTopAccentAttachment is in design-space em units. If either side has
    // no single-glyph attachment, OpenType defines the advance-width center as
    // the geometric fallback. Assemblies therefore center by solved width.
    fn accent_x_off(&self, base: &MathBox, acc: &MathBox, style: MathStyle) -> Result<Dim, Error> {
        let scale = self.params.scale(style);
        let two = Dim::from_i64(2);
        let base_att = match single_glyph_source(base)
            .and_then(|(_, glyph_id)| self.font.top_accent_attachment(glyph_id))
        {
            Some(value) => value.checked_mul(&scale)?,
            None => base.width.checked_div(&two)?,
        };
        let acc_att = match single_glyph_source(acc)
            .and_then(|(_, glyph_id)| self.font.top_accent_attachment(glyph_id))
        {
            Some(value) => value.checked_mul(&scale)?,
            None => acc.width.checked_div(&two)?,
        };
        base_att.checked_sub(&acc_att).map_err(Error::from)
    }

    // AccentBaseHeight controls placement. FlattenedAccentBaseHeight only
    // selects the `flac` shape and must not become a second placement rule.
    fn accent_raise(&self, base: &MathBox, style: MathStyle) -> Result<Dim, Error> {
        let base_height = self
            .params
            .accent_base_height
            .checked_mul(&self.params.scale(style))?;
        Ok(base.height.checked_sub(&base_height)?.clamp_nonneg())
    }

    fn cancelto(&self, value: &MathNode, expr: &MathNode, style: MathStyle) -> Result<Item, Error> {
        // cancel.sty defaults to the "smaller" option: a displaystyle target is
        // textstyle, textstyle becomes scriptstyle, and deeper styles clamp at
        // scriptscriptstyle.  The default overlap mode must not widen the
        // cancelled expression even when the target protrudes past it.
        let b = self.layout(expr, style)?;
        let val_style = if style.is_display() {
            MathStyle::Text
        } else if style.script_level() == 0 {
            MathStyle::Script
        } else {
            MathStyle::ScriptScript
        };
        let val = self.layout(value, val_style)?;
        let (x_left, y_low, x_right, y_high, thickness, overshoot) =
            self.cancel_line_metrics(&b)?;

        let line_height = b.height.checked_add(&overshoot)?;
        let line_depth = b.depth.checked_add(&overshoot)?;
        let line = |x1: Dim, y1: Dim, x2: Dim, y2: Dim| MathBox {
            width: b.width.clone(),
            height: line_height.clone(),
            depth: line_depth.clone(),
            italic: Dim::zero(),
            shift: Dim::zero(),
            content: BoxContent::Line {
                x1,
                y1,
                x2,
                y2,
                thickness: thickness.clone(),
            },
        };

        let mut kids = vec![
            b.clone(),
            line(x_left, y_low, x_right.clone(), y_high.clone()),
        ];

        // The public display list has a generic line primitive rather than a
        // backend-specific arrow primitive.  Represent the arrowhead by two
        // short physical line segments so renderers retain a real arrow while
        // the logical width stays that of the expression, matching cancel.sty
        // default overlap semantics.
        let head_long = self.tex_point_ratio_in_root_em(3, 2)?;
        let head_short = self.tex_point_ratio_in_root_em(1, 2)?;
        kids.push(line(
            x_right.clone(),
            y_high.clone(),
            x_right.checked_sub(&head_long)?,
            y_high.checked_sub(&head_short)?,
        ));
        kids.push(line(
            x_right.clone(),
            y_high.clone(),
            x_right.checked_sub(&head_short)?,
            y_high.checked_sub(&head_long)?,
        ));

        let label_gap = self.tex_point_ratio_in_root_em(1, 2)?;
        let half_val = val.width.checked_div(&Dim::from_i64(2))?;
        let val_x = x_right.checked_sub(&half_val)?;
        let val_shift = y_high.checked_add(&label_gap)?.checked_add(&val.depth)?;
        let val_height = val_shift.checked_add(&val.height)?;
        kids.push(shift_x(val, val_x)?.with_shift(val_shift));

        Ok(Item {
            class: Some(AtomKind::Ord),
            bx: MathBox {
                width: b.width.clone(),
                height: line_height.max_ref(&val_height),
                depth: line_depth,
                italic: Dim::zero(),
                shift: Dim::zero(),
                content: BoxContent::Overlap(kids),
            },
        })
    }

    fn framed_color_box(
        &self,
        inner: MathBox,
        stroke: Option<Color>,
        fill: Option<Color>,
    ) -> Result<MathBox, Error> {
        let sep = self.tex_points_in_root_em(TEX_FBOX_SEP_PT)?;
        let rule = self.tex_point_ratio_in_root_em(TEX_FBOX_RULE_PT_NUM, TEX_FBOX_RULE_PT_DEN)?;
        let padded = pad_box(inner, &sep)?;
        let decorated = match fill {
            Some(color) => back_color_wrap(color, padded),
            None => padded,
        };
        // LaTeX's frame rules sit outside the fboxsep-padded contents.  Model
        // that physical rule extent explicitly so width/height/depth include
        // both sides of the frame rather than treating the stroke as paint-only.
        let framed_inner = pad_box(decorated, &rule)?;
        Ok(frame_wrap(rule, stroke, framed_inner))
    }

    fn cancel_line_metrics(&self, b: &MathBox) -> Result<(Dim, Dim, Dim, Dim, Dim, Dim), Error> {
        let overshoot = self.tex_points_in_root_em(TEX_CANCEL_OVERSHOOT_PT)?;
        let thickness =
            self.tex_point_ratio_in_root_em(TEX_CANCEL_LINE_PT_NUM, TEX_CANCEL_LINE_PT_DEN)?;
        let x_left = -overshoot.clone();
        let y_low = -b.depth.checked_add(&overshoot)?;
        let x_right = b.width.checked_add(&overshoot)?;
        let y_high = b.height.checked_add(&overshoot)?;
        Ok((x_left, y_low, x_right, y_high, thickness, overshoot))
    }

    fn cancel_box(&self, b: MathBox, kind: AccentKind) -> Result<MathBox, Error> {
        let (x_left, y_low, x_right, y_high, thickness, overshoot) =
            self.cancel_line_metrics(&b)?;
        let height = b.height.checked_add(&overshoot)?;
        let depth = b.depth.checked_add(&overshoot)?;
        let mk = |x1: Dim, y1: Dim, x2: Dim, y2: Dim| MathBox {
            width: b.width.clone(),
            height: height.clone(),
            depth: depth.clone(),
            italic: Dim::zero(),
            shift: Dim::zero(),
            content: BoxContent::Line {
                x1,
                y1,
                x2,
                y2,
                thickness: thickness.clone(),
            },
        };
        let mut kids = vec![b.clone()];
        match kind {
            AccentKind::Cancel => kids.push(mk(x_left, y_low, x_right, y_high)),
            AccentKind::BCancel => kids.push(mk(x_left, y_high, x_right, y_low)),
            AccentKind::XCancel => {
                kids.push(mk(
                    x_left.clone(),
                    y_low.clone(),
                    x_right.clone(),
                    y_high.clone(),
                ));
                kids.push(mk(x_left, y_high, x_right, y_low));
            }
            _ => {}
        }
        Ok(MathBox {
            width: b.width.clone(),
            height,
            depth,
            italic: Dim::zero(),
            shift: Dim::zero(),
            content: BoxContent::Overlap(kids),
        })
    }

    fn bar_rule(&self, b: MathBox, under: bool, style: MathStyle) -> Result<Item, Error> {
        let width = accent_nucleus_width(&b)?;
        let s = self.params.scale(style);
        let gap = if under {
            self.params.underbar_vertical_gap.checked_mul(&s)?
        } else {
            self.params.overbar_vertical_gap.checked_mul(&s)?
        };
        let thick = if under {
            self.params.underbar_rule_thickness.checked_mul(&s)?
        } else {
            self.params.overbar_rule_thickness.checked_mul(&s)?
        };
        let extra = if under {
            self.params.underbar_extra_descender.checked_mul(&s)?
        } else {
            self.params.overbar_extra_ascender.checked_mul(&s)?
        };
        let mut height = b.height.clone();
        let mut depth = b.depth.clone();
        let bar = if under {
            depth = depth
                .checked_add(&gap)?
                .checked_add(&thick)?
                .checked_add(&extra)?;
            let shift = b.depth.checked_add(&gap)?;
            MathBox::rule(width.clone(), thick, Dim::zero()).with_shift(-shift)
        } else {
            height = height
                .checked_add(&gap)?
                .checked_add(&thick)?
                .checked_add(&extra)?;
            let shift = b.height.checked_add(&gap)?;
            MathBox::rule(width.clone(), thick, Dim::zero()).with_shift(shift)
        };
        let layers = if under { vec![b, bar] } else { vec![bar, b] };
        Ok(Item {
            class: Some(AtomKind::Ord),
            bx: MathBox {
                width,
                height,
                depth,
                italic: Dim::zero(),
                shift: Dim::zero(),
                content: BoxContent::Overlap(layers),
            },
        })
    }

    fn take_number(&self) -> Option<String> {
        let i = self.idx.get();
        self.idx.set(i + 1);
        self.numbering.assigned(i).map(str::to_string)
    }

    fn number_box(&self, s: &str, style: MathStyle) -> Result<MathBox, Error> {
        Ok(self.math_alphabet_run(s, TextStyle::Rm, style)?.bx)
    }

    fn attach_number(
        &self,
        body: MathBox,
        num: Option<MathBox>,
        style: MathStyle,
    ) -> Result<MathBox, Error> {
        let Some(num) = num else {
            return Ok(body);
        };
        let gap = self.params.em(style)?;
        Ok(MathBox::hpack(vec![body, MathBox::kern(gap), num])?)
    }

    fn reference(&self, key: &str, style: MathStyle) -> Result<Item, Error> {
        let s = self
            .numbering
            .lookup(self.numbers, key)
            .ok_or_else(|| Error::Unsupported {
                what: format!("undefined label {key}"),
            })?;
        self.math_alphabet_run(s, TextStyle::Rm, style)
    }

    fn tag_box(&self, star: bool, body: &MathNode, style: MathStyle) -> Result<Item, Error> {
        let inner = self.layout(body, MathStyle::Text)?;
        let bx = if star {
            inner
        } else {
            let open = self.glyph('(', style)?;
            let close = self.glyph(')', style)?;
            MathBox::hpack(vec![open, inner, close])?
        };
        Ok(Item {
            class: Some(AtomKind::Ord),
            bx,
        })
    }

    fn substack(&self, lines: &[MathNode], style: MathStyle) -> Result<Item, Error> {
        if lines.is_empty() {
            return Ok(Item {
                class: Some(AtomKind::Ord),
                bx: MathBox::empty(),
            });
        }

        // AMSMath subarray rows are always Script style,
        // even when the surrounding expression is already Script.
        let mut laid = Vec::with_capacity(lines.len());
        for line in lines {
            laid.push(self.clean_math_component(line, MathStyle::Script)?);
        }
        let width = laid
            .iter()
            .fold(Dim::zero(), |current, row| current.max_ref(&row.width));
        let rows = laid
            .into_iter()
            .map(|row| center_in(row, &width))
            .collect::<Result<Vec<_>, _>>()?;
        let gaps = self.substack_interrow_gaps(&rows)?;
        let stack = self.center_amsmath_stack(rows, &gaps, style)?;

        // Large-operator limit placement overwrites the outer
        // MathBox::shift. Keep substack vcentering on the child.
        Ok(Item {
            class: Some(AtomKind::Ord),
            bx: shifted_hpack(vec![stack])?,
        })
    }

    fn substack_interrow_gaps(&self, rows: &[MathBox]) -> Result<Vec<Dim>, Error> {
        rows.windows(2)
            .map(|pair| {
                let candidate = self
                    .substack
                    .baseline_skip
                    .checked_sub(&pair[0].depth)?
                    .checked_sub(&pair[1].height)?;
                Ok(
                    if candidate.cmp(&self.substack.line_skip) != Ordering::Less {
                        candidate
                    } else {
                        self.substack.line_skip.clone()
                    },
                )
            })
            .collect()
    }

    fn matrix(
        &self,
        style_m: MatrixStyle,
        spec: &[ColSpec],
        rows: &[EnvRow],
        style: MathStyle,
    ) -> Result<Item, Error> {
        if rows.is_empty() {
            return Ok(Item {
                bx: MathBox::empty(),
                class: Some(AtomKind::Inner),
            });
        }
        let body_style = if style_m.is_display_env() {
            MathStyle::Display
        } else {
            style
        };
        match style_m {
            MatrixStyle::Align => self.align_env(rows, body_style, true),
            MatrixStyle::Aligned => self.amsmath_aligned(rows, body_style),
            MatrixStyle::Split => self.align_env(rows, body_style, false),
            MatrixStyle::Gather => self.gather_env(rows, body_style),
            MatrixStyle::Multline => self.multline_env(rows, body_style),
            MatrixStyle::Equation => self.equation_env(rows, body_style),
            MatrixStyle::Array => self.array_env(spec, rows, body_style),
            MatrixStyle::Cases => self.cases_env(rows, body_style),
            _ => self.centered_matrix(style_m, rows, body_style),
        }
    }

    // AMSMath matrix/cases/aligned layouts have
    // environment-specific cell styles, physical spacing and vertical
    // centering rules that the generic grid does not.
    fn centered_matrix(
        &self,
        style_m: MatrixStyle,
        rows: &[EnvRow],
        style: MathStyle,
    ) -> Result<Item, Error> {
        let data = data_cells(rows)?;
        let cells = self.layout_environment_rows(&data, MathStyle::Text)?;
        let column_gap = self.tex_points_in_root_em(TEX_ARRAY_COLSEP_PT * 2)?;
        let packed_rows = self.build_amsmath_rows(
            cells,
            &column_gap,
            &Dim::one(),
            |_| ColSpec::Center,
            |column| column > 0,
        )?;
        let gaps = vec![Dim::zero(); packed_rows.len().saturating_sub(1)];
        let stack = self.center_amsmath_stack(packed_rows, &gaps, style)?;
        let bx = self.wrap_amsmath_stack(stack, matrix_delims(style_m), style, false)?;
        Ok(Item {
            class: Some(AtomKind::Inner),
            bx,
        })
    }

    fn cases_env(&self, rows: &[EnvRow], style: MathStyle) -> Result<Item, Error> {
        let data = data_cells(rows)?;
        let cells = self.layout_environment_rows(&data, MathStyle::Text)?;
        let stretch = Dim::ratio(6, 5)?;
        let packed_rows = self.build_amsmath_rows(
            cells,
            &Dim::one(),
            &stretch,
            |_| ColSpec::Left,
            |column| column > 0,
        )?;
        let gaps = vec![Dim::zero(); packed_rows.len().saturating_sub(1)];
        let stack = self.center_amsmath_stack(packed_rows, &gaps, style)?;
        let bx = self.wrap_amsmath_stack(stack, (Some('{'), None), style, true)?;
        Ok(Item {
            class: Some(AtomKind::Inner),
            bx,
        })
    }

    fn amsmath_aligned(&self, rows: &[EnvRow], style: MathStyle) -> Result<Item, Error> {
        if rows.iter().any(|row| !matches!(row, EnvRow::Cells { .. })) {
            return self.align_env(rows, style, false);
        }

        let mut cells = Vec::with_capacity(rows.len());
        for row in rows {
            let EnvRow::Cells { cells: row, .. } = row else {
                unreachable!("non-cell aligned rows handled above");
            };
            let mut laid = Vec::with_capacity(row.len());
            for (column, cell) in row.iter().enumerate() {
                let mut bx = self.clean_math_component(cell, MathStyle::Display)?;
                if column % 2 == 1 {
                    let leading = self.aligned_leading_ord_space(cell)?;
                    if !leading.is_zero() {
                        bx = MathBox::hpack(vec![MathBox::kern(leading), bx])?;
                    }
                }
                laid.push(bx);
            }
            cells.push(laid);
        }

        let pair_gap = self.tex_points_in_root_em(TEX_MIN_ALIGN_SEP_PT)?;
        let packed_rows = self.build_amsmath_rows(
            cells,
            &pair_gap,
            &Dim::one(),
            |column| {
                if column % 2 == 0 {
                    ColSpec::Right
                } else {
                    ColSpec::Left
                }
            },
            |column| column > 0 && column % 2 == 0,
        )?;
        let gaps = self.aligned_interrow_gaps(&packed_rows)?;
        let stack = self.center_amsmath_stack(packed_rows, &gaps, style)?;
        let bx = self.wrap_amsmath_stack(stack, (None, None), style, false)?;
        Ok(Item {
            class: Some(AtomKind::Inner),
            bx,
        })
    }

    fn resolve_length(&self, length: &Length, style: MathStyle) -> Result<Dim, NumericError> {
        resolve_length(length, style, &self.params, &self.root_em_size)
    }

    fn tex_points_in_root_em(&self, points: i64) -> Result<Dim, Error> {
        self.tex_point_ratio_in_root_em(points, 1)
    }

    fn tex_point_ratio_in_root_em(&self, numerator: i64, denominator: i64) -> Result<Dim, Error> {
        let points = Dim::ratio(numerator, denominator)?;
        Ok(self.resolve_length(&Length::TexPt(points), MathStyle::Text)?)
    }

    fn layout_environment_rows(
        &self,
        rows: &[Vec<MathNode>],
        style: MathStyle,
    ) -> Result<Vec<Vec<MathBox>>, Error> {
        rows.iter()
            .map(|row| {
                row.iter()
                    .map(|cell| self.clean_math_component(cell, style))
                    .collect::<Result<Vec<_>, _>>()
            })
            .collect()
    }

    fn aligned_leading_ord_space(&self, cell: &MathNode) -> Result<Dim, Error> {
        let bare = self.clean_math_component(cell, MathStyle::Display)?;
        let empty_ord = MathNode::Row(Vec::new());
        let prefixed = match cell {
            MathNode::Row(items) => {
                let mut with_ord = Vec::with_capacity(items.len().saturating_add(1));
                with_ord.push(empty_ord);
                with_ord.extend(items.iter().cloned());
                MathNode::Row(with_ord)
            }
            _ => MathNode::Row(vec![empty_ord, cell.clone()]),
        };
        let with_ord = self.layout(&prefixed, MathStyle::Display)?;
        Ok(with_ord.width.checked_sub(&bare.width)?.clamp_nonneg())
    }

    fn amsmath_strut(stretch: &Dim) -> Result<MathBox, Error> {
        Ok(MathBox::rule(
            Dim::zero(),
            Dim::ratio(7, 10)?.checked_mul(stretch)?,
            Dim::ratio(3, 10)?.checked_mul(stretch)?,
        ))
    }

    fn build_amsmath_rows(
        &self,
        rows: Vec<Vec<MathBox>>,
        gap: &Dim,
        strut_stretch: &Dim,
        align_for_column: impl Fn(usize) -> ColSpec,
        gap_before_column: impl Fn(usize) -> bool,
    ) -> Result<Vec<MathBox>, Error> {
        let ncols = rows.iter().map(Vec::len).max().unwrap_or(0);
        let mut widths = vec![Dim::zero(); ncols];
        for row in &rows {
            for (column, cell) in row.iter().enumerate() {
                widths[column] = widths[column].max_ref(&cell.width);
            }
        }

        rows.into_iter()
            .map(|mut row| {
                while row.len() < ncols {
                    row.push(MathBox::empty());
                }
                let mut parts = Vec::with_capacity(row.len().saturating_mul(2).saturating_add(1));
                parts.push(Self::amsmath_strut(strut_stretch)?);
                for (column, cell) in row.into_iter().enumerate() {
                    if gap_before_column(column) {
                        parts.push(MathBox::kern(gap.clone()));
                    }
                    parts.push(align_in(cell, &widths[column], align_for_column(column))?);
                }
                Ok(MathBox::hpack(parts)?)
            })
            .collect()
    }

    fn aligned_interrow_gaps(&self, rows: &[MathBox]) -> Result<Vec<Dim>, Error> {
        let jot = self.tex_points_in_root_em(TEX_JOT_PT)?;
        let baseline_skip = Dim::one().checked_add(&jot)?;
        let line_skip = self
            .tex_points_in_root_em(TEX_LINE_SKIP_PT)?
            .checked_add(&jot)?;
        rows.windows(2)
            .map(|pair| {
                let candidate = baseline_skip
                    .checked_sub(&pair[0].depth)?
                    .checked_sub(&pair[1].height)?;
                Ok(if candidate.cmp(&jot) == Ordering::Less {
                    line_skip.clone()
                } else {
                    candidate
                })
            })
            .collect()
    }

    fn center_amsmath_stack(
        &self,
        rows: Vec<MathBox>,
        gaps: &[Dim],
        style: MathStyle,
    ) -> Result<MathBox, Error> {
        debug_assert_eq!(gaps.len(), rows.len().saturating_sub(1));
        let mut children = Vec::with_capacity(rows.len().saturating_mul(2).saturating_sub(1));
        for (row_index, row) in rows.into_iter().enumerate() {
            if let Some(gap) = row_index.checked_sub(1).and_then(|index| gaps.get(index)) {
                if !gap.is_zero() {
                    children.push(sep_row(gap.clone()));
                }
            }
            children.push(row);
        }
        if children.is_empty() {
            return Ok(MathBox::empty());
        }

        let mut stack = MathBox::vpack(children)?;
        let span = stack.height.checked_add(&stack.depth)?;
        let axis = self
            .params
            .axis_height
            .checked_mul(&self.params.scale(style))?;
        let desired_height = span.checked_div(&Dim::from_i64(2))?.checked_add(&axis)?;
        stack.shift = desired_height.checked_sub(&stack.height)?;
        Ok(stack)
    }

    fn wrap_amsmath_stack(
        &self,
        stack: MathBox,
        delims: (Option<char>, Option<char>),
        style: MathStyle,
        right_null_delimiter: bool,
    ) -> Result<MathBox, Error> {
        let scale = self.params.scale(style);
        let axis = self.params.axis_height.checked_mul(&scale)?;
        let effective_height = stack.height.checked_add(&stack.shift)?.clamp_nonneg();
        let effective_depth = stack.depth.checked_sub(&stack.shift)?.clamp_nonneg();
        let needed = self.delimiter_target(&effective_height, &effective_depth, &axis)?;
        let mut children = Vec::with_capacity(3);
        if let Some(ch) = delims.0 {
            children.push(self.center_delimiter(self.sized_glyph(ch, &needed, style)?, &axis)?);
        }
        children.push(stack);
        if let Some(ch) = delims.1 {
            children.push(self.center_delimiter(self.sized_glyph(ch, &needed, style)?, &axis)?);
        } else if right_null_delimiter {
            children.push(MathBox::kern(self.null_delimiter_space.clone()));
        }
        shifted_hpack(children)
    }

    fn align_env(&self, rows: &[EnvRow], style: MathStyle, numbered: bool) -> Result<Item, Error> {
        let mut math_rows: Vec<Vec<MathBox>> = Vec::new();
        let mut nums: Vec<Option<MathBox>> = Vec::new();
        let mut extras: Vec<RowKind> = Vec::new();
        let mut ncols = 0;
        for row in rows {
            match row {
                EnvRow::Hline => extras.push(RowKind::Hline),
                EnvRow::Intertext(n) => extras.push(RowKind::Intertext(Box::new(
                    self.layout(n, MathStyle::Text)?,
                ))),
                EnvRow::Cells { cells, .. } => {
                    let mut rboxes = Vec::new();
                    for c in cells {
                        rboxes.push(self.layout(c, style)?);
                    }
                    ncols = ncols.max(rboxes.len());
                    math_rows.push(rboxes);
                    extras.push(RowKind::Cells);
                    if numbered {
                        nums.push(match self.take_number() {
                            Some(s) => Some(self.number_box(&s, MathStyle::Text)?),
                            None => None,
                        });
                    }
                }
            }
        }
        let mut col_w = vec![Dim::zero(); ncols];
        for row in &math_rows {
            for (j, cell) in row.iter().enumerate() {
                col_w[j] = col_w[j].max_ref(&cell.width);
            }
        }
        let pair_sep = self.params.em(style)?;
        let row_sep = self.params.em(style)?.checked_div(&Dim::from_i64(5))?;
        let mut packed = Vec::new();
        let mut mi = 0;
        for extra in extras {
            match extra {
                RowKind::Hline => {
                    return Err(Error::Unsupported {
                        what: "hline in align".into(),
                    });
                }
                RowKind::Intertext(t) => {
                    if !packed.is_empty() {
                        packed.push(sep_row(row_sep.clone()));
                    }
                    packed.push(*t);
                }
                RowKind::Cells => {
                    if !packed.is_empty() {
                        packed.push(sep_row(row_sep.clone()));
                    }
                    let mut parts = Vec::new();
                    let row = pad_row(math_rows[mi].clone(), ncols);
                    for (j, cell) in row.into_iter().enumerate() {
                        if j > 0 && j % 2 == 0 {
                            parts.push(MathBox::kern(pair_sep.clone()));
                        }
                        let align = if j % 2 == 0 {
                            ColSpec::Right
                        } else {
                            ColSpec::Left
                        };
                        parts.push(align_in(cell, &col_w[j], align)?);
                    }
                    let mut body = MathBox::hpack(parts)?;
                    if numbered {
                        body = self.attach_number(body, nums[mi].clone(), style)?;
                    }
                    packed.push(body);
                    mi += 1;
                }
            }
        }
        Ok(Item {
            class: Some(AtomKind::Inner),
            bx: MathBox::vpack(packed)?,
        })
    }

    fn gather_env(&self, rows: &[EnvRow], style: MathStyle) -> Result<Item, Error> {
        let row_sep = self.params.em(style)?.checked_div(&Dim::from_i64(5))?;
        let mut bodies = Vec::new();
        let mut kinds = Vec::new();
        for row in rows {
            match row {
                EnvRow::Hline => {
                    return Err(Error::Unsupported {
                        what: "hline in gather".into(),
                    });
                }
                EnvRow::Intertext(n) => kinds.push(RowKind::Intertext(Box::new(
                    self.layout(n, MathStyle::Text)?,
                ))),
                EnvRow::Cells { cells, .. } => {
                    let node = if cells.len() == 1 {
                        cells[0].clone()
                    } else {
                        MathNode::Row(cells.clone())
                    };
                    bodies.push(self.layout(&node, style)?);
                    kinds.push(RowKind::Cells);
                }
            }
        }
        let max_w = bodies.iter().fold(Dim::zero(), |w, b| w.max_ref(&b.width));
        let mut packed = Vec::new();
        let mut bi = 0;
        for kind in kinds {
            if !packed.is_empty() {
                packed.push(sep_row(row_sep.clone()));
            }
            match kind {
                RowKind::Intertext(t) => packed.push(*t),
                RowKind::Cells => {
                    let body = align_in(bodies[bi].clone(), &max_w, ColSpec::Center)?;
                    let num = match self.take_number() {
                        Some(s) => Some(self.number_box(&s, MathStyle::Text)?),
                        None => None,
                    };
                    packed.push(self.attach_number(body, num, style)?);
                    bi += 1;
                }
                RowKind::Hline => {}
            }
        }
        Ok(Item {
            class: Some(AtomKind::Inner),
            bx: MathBox::vpack(packed)?,
        })
    }

    fn multline_env(&self, rows: &[EnvRow], style: MathStyle) -> Result<Item, Error> {
        let data = data_cells(rows)?;
        if data.is_empty() {
            let _ = self.take_number();
            return Ok(Item {
                bx: MathBox::empty(),
                class: Some(AtomKind::Inner),
            });
        }
        let mut bodies = Vec::new();
        for row in &data {
            let node = if row.len() == 1 {
                row[0].clone()
            } else {
                MathNode::Row(row.clone())
            };
            bodies.push(self.layout(&node, style)?);
        }
        let max_w = bodies.iter().fold(Dim::zero(), |w, b| w.max_ref(&b.width));
        let n = bodies.len();
        let row_sep = self.params.em(style)?.checked_div(&Dim::from_i64(5))?;
        let mut packed = Vec::new();
        for (i, b) in bodies.into_iter().enumerate() {
            if i > 0 {
                packed.push(sep_row(row_sep.clone()));
            }
            let align = if i == 0 {
                ColSpec::Left
            } else if i + 1 == n {
                ColSpec::Right
            } else {
                ColSpec::Center
            };
            packed.push(align_in(b, &max_w, align)?);
        }
        let mut inner = MathBox::vpack(packed)?;
        let num = match self.take_number() {
            Some(s) => Some(self.number_box(&s, MathStyle::Text)?),
            None => None,
        };
        inner = self.attach_number(inner, num, style)?;
        Ok(Item {
            class: Some(AtomKind::Inner),
            bx: inner,
        })
    }

    fn equation_env(&self, rows: &[EnvRow], style: MathStyle) -> Result<Item, Error> {
        let data = data_cells(rows)?;
        let mut parts = Vec::new();
        for row in &data {
            for (i, c) in row.iter().enumerate() {
                if i > 0 {
                    parts.push(MathNode::Space(SpaceKind::Quad));
                }
                parts.push(c.clone());
            }
        }
        let node = wrap_nodes(parts);
        let body = self.layout(&node, style)?;
        let num = match self.take_number() {
            Some(s) => Some(self.number_box(&s, MathStyle::Text)?),
            None => None,
        };
        Ok(Item {
            class: Some(AtomKind::Inner),
            bx: self.attach_number(body, num, style)?,
        })
    }

    fn array_env(
        &self,
        spec: &[ColSpec],
        rows: &[EnvRow],
        style: MathStyle,
    ) -> Result<Item, Error> {
        let thick = self
            .params
            .fraction_rule_thickness
            .checked_mul(&self.params.scale(style))?;
        let col_sep = self.params.mu(style)?.checked_mul(&Dim::from_i64(10))?;
        let row_sep = self.params.em(style)?.checked_div(&Dim::from_i64(5))?;
        let mut kinds = Vec::new();
        let mut data: Vec<Vec<MathBox>> = Vec::new();
        let mut ncols_data = 0;
        for row in rows {
            match row {
                EnvRow::Hline => kinds.push(RowKind::Hline),
                EnvRow::Intertext(n) => kinds.push(RowKind::Intertext(Box::new(
                    self.layout(n, MathStyle::Text)?,
                ))),
                EnvRow::Cells { cells, .. } => {
                    let mut rboxes = Vec::new();
                    for c in cells {
                        rboxes.push(self.layout(c, style)?);
                    }
                    ncols_data = ncols_data.max(rboxes.len());
                    data.push(rboxes);
                    kinds.push(RowKind::Cells);
                }
            }
        }
        let mut spec: Vec<ColSpec> = if spec.is_empty() {
            vec![ColSpec::Center; ncols_data]
        } else {
            spec.to_vec()
        };
        let mut ndata = spec.iter().filter(|c| !c.is_rule()).count();
        while ndata < ncols_data {
            spec.push(ColSpec::Center);
            ndata += 1;
        }
        for row in &mut data {
            while row.len() < ndata {
                row.push(MathBox::empty());
            }
        }
        let mut col_w: Vec<Dim> = spec
            .iter()
            .map(|c| {
                if c.is_rule() {
                    thick.clone()
                } else {
                    Dim::zero()
                }
            })
            .collect();
        for row in &data {
            let mut dj = 0;
            for (j, sp) in spec.iter().enumerate() {
                if sp.is_rule() {
                    continue;
                }
                if dj < row.len() {
                    col_w[j] = col_w[j].max_ref(&row[dj].width);
                }
                dj += 1;
            }
        }
        let mut table_w = Dim::zero();
        for (j, _) in spec.iter().enumerate() {
            if j > 0 && !spec[j].is_rule() && !spec[j - 1].is_rule() {
                table_w = table_w.checked_add(&col_sep)?;
            }
            table_w = table_w.checked_add(&col_w[j])?;
        }
        let mut packed = Vec::new();
        let mut di = 0;
        for kind in kinds {
            match kind {
                RowKind::Hline => {
                    packed.push(MathBox::rule(table_w.clone(), thick.clone(), Dim::zero()));
                }
                RowKind::Intertext(t) => {
                    if !packed.is_empty() {
                        packed.push(sep_row(row_sep.clone()));
                    }
                    packed.push(*t);
                }
                RowKind::Cells => {
                    if !packed.is_empty()
                        && !matches!(packed.last().map(|b| &b.content), Some(BoxContent::Rule))
                    {
                        packed.push(sep_row(row_sep.clone()));
                    }
                    let row = &data[di];
                    let mut rh = Dim::zero();
                    let mut rd = Dim::zero();
                    for c in row {
                        rh = rh.max_ref(&c.height);
                        rd = rd.max_ref(&c.depth);
                    }
                    let mut parts = Vec::new();
                    let mut dj = 0;
                    for (j, sp) in spec.iter().enumerate() {
                        if j > 0 && !sp.is_rule() && !spec[j - 1].is_rule() {
                            parts.push(MathBox::kern(col_sep.clone()));
                        }
                        if sp.is_rule() {
                            parts.push(MathBox {
                                width: thick.clone(),
                                height: rh.clone(),
                                depth: rd.clone(),
                                italic: Dim::zero(),
                                shift: Dim::zero(),
                                content: BoxContent::Rule,
                            });
                        } else {
                            let cell = row.get(dj).cloned().unwrap_or_else(MathBox::empty);
                            parts.push(align_in(cell, &col_w[j], *sp)?);
                            dj += 1;
                        }
                    }
                    packed.push(MathBox::hpack(parts)?);
                    di += 1;
                }
            }
        }
        Ok(Item {
            class: Some(AtomKind::Inner),
            bx: MathBox::vpack(packed)?,
        })
    }
}

fn effective_height(math_box: &MathBox) -> Result<Dim, NumericError> {
    Ok(math_box.height.checked_add(&math_box.shift)?.clamp_nonneg())
}

fn effective_depth(math_box: &MathBox) -> Result<Dim, NumericError> {
    Ok(math_box.depth.checked_sub(&math_box.shift)?.clamp_nonneg())
}

fn direct_glyph(math_box: &MathBox) -> Option<(u16, &Dim)> {
    match &math_box.content {
        BoxContent::Glyph {
            glyph_id, scale, ..
        } => Some((*glyph_id, scale)),
        _ => None,
    }
}

fn position_script(mut script: MathBox, leading_kern: Dim) -> Result<MathBox, Error> {
    if leading_kern.is_zero() {
        return Ok(script);
    }
    let intrinsic_shift = script.shift.clone();
    script.shift = Dim::zero();
    let height = script.height.clone();
    let depth = script.depth.clone();
    let packed = MathBox::hpack(vec![MathBox::kern(leading_kern), script])?;
    Ok(MathBox {
        width: packed.width,
        height,
        depth,
        italic: Dim::zero(),
        shift: intrinsic_shift,
        content: packed.content,
    })
}

fn shifted_hpack(children: Vec<MathBox>) -> Result<MathBox, Error> {
    let mut width = Dim::zero();
    let mut height = Dim::zero();
    let mut depth = Dim::zero();
    for child in &children {
        width = width.checked_add(&child.width)?;
        height = height.max_ref(&child.height.checked_add(&child.shift)?.clamp_nonneg());
        depth = depth.max_ref(&child.depth.checked_sub(&child.shift)?.clamp_nonneg());
    }
    Ok(MathBox {
        width,
        height,
        depth,
        italic: Dim::zero(),
        shift: Dim::zero(),
        content: BoxContent::HList(children),
    })
}

enum RowKind {
    Cells,
    Hline,
    Intertext(Box<MathBox>),
}

fn data_cells(rows: &[EnvRow]) -> Result<Vec<Vec<MathNode>>, Error> {
    let mut out = Vec::new();
    for r in rows {
        match r {
            EnvRow::Cells { cells, .. } => out.push(cells.clone()),
            EnvRow::Hline => {
                return Err(Error::Unsupported {
                    what: "hline in this environment".into(),
                });
            }
            EnvRow::Intertext(_) => {
                return Err(Error::Unsupported {
                    what: "intertext in this environment".into(),
                });
            }
        }
    }
    Ok(out)
}

fn sep_row(depth: Dim) -> MathBox {
    MathBox {
        width: Dim::zero(),
        height: Dim::zero(),
        depth,
        italic: Dim::zero(),
        shift: Dim::zero(),
        content: BoxContent::Empty,
    }
}

fn pad_row(mut row: Vec<MathBox>, ncols: usize) -> Vec<MathBox> {
    while row.len() < ncols {
        row.push(MathBox::empty());
    }
    row
}

fn wrap_nodes(items: Vec<MathNode>) -> MathNode {
    if items.len() == 1 {
        items
            .into_iter()
            .next()
            .unwrap_or(MathNode::Row(Vec::new()))
    } else {
        MathNode::Row(items)
    }
}

fn align_in(inner: MathBox, width: &Dim, align: ColSpec) -> Result<MathBox, Error> {
    if matches!(align, ColSpec::VRule) || inner.width.eq_dim(width) {
        return Ok(inner);
    }
    let extra = width.checked_sub(&inner.width)?;
    match align {
        ColSpec::Center => center_in(inner, width),
        ColSpec::Left => {
            let h = inner.height.clone();
            let d = inner.depth.clone();
            let packed = MathBox::hpack(vec![inner, MathBox::kern(extra)])?;
            Ok(MathBox {
                width: packed.width,
                height: h,
                depth: d,
                italic: Dim::zero(),
                shift: Dim::zero(),
                content: packed.content,
            })
        }
        ColSpec::Right => {
            let h = inner.height.clone();
            let d = inner.depth.clone();
            let packed = MathBox::hpack(vec![MathBox::kern(extra), inner])?;
            Ok(MathBox {
                width: packed.width,
                height: h,
                depth: d,
                italic: Dim::zero(),
                shift: Dim::zero(),
                content: packed.content,
            })
        }
        ColSpec::VRule => Ok(inner),
    }
}

fn color_wrap(c: Color, inner: MathBox) -> MathBox {
    MathBox {
        width: inner.width.clone(),
        height: inner.height.clone(),
        depth: inner.depth.clone(),
        italic: inner.italic.clone(),
        shift: inner.shift.clone(),
        content: BoxContent::Color(c, Box::new(inner)),
    }
}

fn back_color_wrap(c: Color, inner: MathBox) -> MathBox {
    MathBox {
        width: inner.width.clone(),
        height: inner.height.clone(),
        depth: inner.depth.clone(),
        italic: inner.italic.clone(),
        shift: inner.shift.clone(),
        content: BoxContent::BackColor(c, Box::new(inner)),
    }
}

fn frame_wrap(thickness: Dim, stroke: Option<Color>, inner: MathBox) -> MathBox {
    MathBox {
        width: inner.width.clone(),
        height: inner.height.clone(),
        depth: inner.depth.clone(),
        italic: Dim::zero(),
        shift: Dim::zero(),
        content: BoxContent::Frame {
            thickness,
            stroke,
            inner: Box::new(inner),
        },
    }
}

fn pad_box(inner: MathBox, pad: &Dim) -> Result<MathBox, Error> {
    let w = inner.width.checked_add(pad)?.checked_add(pad)?;
    let h = inner.height.checked_add(pad)?;
    let d = inner.depth.checked_add(pad)?;
    Ok(MathBox {
        width: w,
        height: h,
        depth: d,
        italic: Dim::zero(),
        shift: Dim::zero(),
        content: BoxContent::HList(vec![
            MathBox::kern(pad.clone()),
            inner,
            MathBox::kern(pad.clone()),
        ]),
    })
}

fn center_in(inner: MathBox, width: &Dim) -> Result<MathBox, Error> {
    if inner.width.eq_dim(width) {
        return Ok(inner);
    }
    let extra = width.checked_sub(&inner.width)?;
    let half = extra.checked_div(&Dim::from_i64(2))?;
    let h = inner.height.clone();
    let d = inner.depth.clone();
    let packed = MathBox::hpack(vec![
        MathBox::kern(half.clone()),
        inner,
        MathBox::kern(extra.checked_sub(&half)?),
    ])?;
    Ok(MathBox {
        width: packed.width,
        height: h,
        depth: d,
        italic: Dim::zero(),
        shift: Dim::zero(),
        content: packed.content,
    })
}

fn center_in_with_offset(
    inner: MathBox,
    width: &Dim,
    center_offset: &Dim,
) -> Result<MathBox, Error> {
    let extra = width.checked_sub(&inner.width)?;
    let left = extra
        .checked_div(&Dim::from_i64(2))?
        .checked_add(center_offset)?;
    let right = extra.checked_sub(&left)?;

    // Keep the TeX operator-stack logical width fixed while shifting the
    // limit inside that frame. A half-italic offset may therefore make one
    // padding kern negative; this is intentional protrusion, not extra width.
    let h = effective_height(&inner)?;
    let d = effective_depth(&inner)?;
    let packed = MathBox::hpack(vec![MathBox::kern(left), inner, MathBox::kern(right)])?;
    Ok(MathBox {
        width: packed.width,
        height: h,
        depth: d,
        italic: Dim::zero(),
        shift: Dim::zero(),
        content: packed.content,
    })
}

fn shift_x(inner: MathBox, x: Dim) -> Result<MathBox, Error> {
    if x.is_zero() {
        return Ok(inner);
    }
    let h = inner.height.clone();
    let d = inner.depth.clone();
    let packed = MathBox::hpack(vec![MathBox::kern(x), inner])?;
    Ok(MathBox {
        width: packed.width,
        height: h,
        depth: d,
        italic: Dim::zero(),
        shift: Dim::zero(),
        content: packed.content,
    })
}

fn overlay_accent(
    base: MathBox,
    acc: MathBox,
    x_off: Dim,
    raise: Dim,
    width: Dim,
) -> Result<MathBox, Error> {
    let acc_top = acc.height.checked_add(&raise)?;
    let acc_bot = raise.checked_sub(&acc.depth)?;
    let height = base.height.max_ref(&acc_top);
    let depth = base.depth.max_ref(&(-acc_bot).clamp_nonneg());
    let italic = acc.italic.clone();
    let accent = shift_x(acc, x_off)?.with_shift(raise);
    Ok(MathBox {
        width,
        height,
        depth,
        italic,
        shift: Dim::zero(),
        content: BoxContent::Overlap(vec![accent, base]),
    })
}

fn place_under_accent(
    base: MathBox,
    acc: MathBox,
    x_off: Dim,
    width: Dim,
) -> Result<MathBox, Error> {
    let raise = -base.depth.checked_add(&acc.height)?;
    let depth = base
        .depth
        .checked_add(&acc.height)?
        .checked_add(&acc.depth)?;
    let italic = acc.italic.clone();
    Ok(MathBox {
        width,
        height: base.height.clone(),
        depth,
        italic,
        shift: Dim::zero(),
        content: BoxContent::Overlap(vec![base, shift_x(acc, x_off)?.with_shift(raise)]),
    })
}

fn single_glyph_source(b: &MathBox) -> Option<(char, u16)> {
    match &b.content {
        BoxContent::Glyph { ch, glyph_id, .. } => Some((*ch, *glyph_id)),

        BoxContent::HList(children) | BoxContent::VList(children) => {
            let mut found = None;

            for child in children {
                if matches!(&child.content, BoxContent::Empty | BoxContent::Kern(_)) {
                    continue;
                }

                let glyph = single_glyph_source(child)?;

                if found.replace(glyph).is_some() {
                    return None;
                }
            }

            found
        }

        BoxContent::Color(_, inner)
        | BoxContent::BackColor(_, inner)
        | BoxContent::Frame { inner, .. } => single_glyph_source(inner),

        _ => None,
    }
}

fn uses_dotless_accent_base(kind: AccentKind) -> bool {
    !matches!(
        kind,
        AccentKind::Not
            | AccentKind::Boxed
            | AccentKind::Cancel
            | AccentKind::BCancel
            | AccentKind::XCancel
            | AccentKind::Underline
            | AccentKind::Underleftarrow
            | AccentKind::Underrightarrow
            | AccentKind::Underleftrightarrow
            | AccentKind::Underbrace
    )
}

fn is_tex_accent(kind: AccentKind) -> bool {
    matches!(
        kind,
        AccentKind::Hat
            | AccentKind::Check
            | AccentKind::Breve
            | AccentKind::Acute
            | AccentKind::Grave
            | AccentKind::Tilde
            | AccentKind::Bar
            | AccentKind::Vec
            | AccentKind::Dot
            | AccentKind::Ddot
            | AccentKind::Dddot
            | AccentKind::Ddddot
            | AccentKind::Ring
            | AccentKind::WideHat
            | AccentKind::WideTilde
            | AccentKind::Overleftarrow
            | AccentKind::Overrightarrow
            | AccentKind::Overleftrightarrow
            | AccentKind::Underleftarrow
            | AccentKind::Underrightarrow
            | AccentKind::Underleftrightarrow
            | AccentKind::Overbrace
            | AccentKind::Underbrace
    )
}

fn variant_measure_is_better(current: &Dim, candidate: &Dim, target: &Dim) -> bool {
    match (current >= target, candidate >= target) {
        (false, false) => candidate > current,
        (false, true) => true,
        (true, false) => false,
        (true, true) => candidate < current,
    }
}

fn accent_nucleus_width(base: &MathBox) -> Result<Dim, Error> {
    match &base.content {
        BoxContent::Glyph { .. } => base.width.checked_add(&base.italic).map_err(Error::from),
        _ => Ok(base.width.clone()),
    }
}

fn accent_prefers_math_construction(kind: AccentKind) -> bool {
    matches!(
        kind,
        AccentKind::Hat
            | AccentKind::WideHat
            | AccentKind::Tilde
            | AccentKind::WideTilde
            | AccentKind::Vec
            | AccentKind::Overleftarrow
            | AccentKind::Overrightarrow
            | AccentKind::Overleftrightarrow
            | AccentKind::Underleftarrow
            | AccentKind::Underrightarrow
            | AccentKind::Underleftrightarrow
            | AccentKind::Overbrace
            | AccentKind::Underbrace
    )
}

fn is_scripted_character_accent(kind: AccentKind) -> bool {
    is_tex_accent(kind) && !matches!(kind, AccentKind::Overbrace | AccentKind::Underbrace)
}

fn is_stretchy_accent(kind: AccentKind) -> bool {
    matches!(
        kind,
        AccentKind::WideHat
            | AccentKind::WideTilde
            | AccentKind::Overleftarrow
            | AccentKind::Overrightarrow
            | AccentKind::Overleftrightarrow
            | AccentKind::Underleftarrow
            | AccentKind::Underrightarrow
            | AccentKind::Underleftrightarrow
            | AccentKind::Overbrace
            | AccentKind::Underbrace
    )
}

fn is_diacritic_accent(kind: AccentKind) -> bool {
    matches!(
        kind,
        AccentKind::Hat
            | AccentKind::WideHat
            | AccentKind::Check
            | AccentKind::Breve
            | AccentKind::Acute
            | AccentKind::Grave
            | AccentKind::Tilde
            | AccentKind::WideTilde
            | AccentKind::Bar
            | AccentKind::Dot
            | AccentKind::Ddot
            | AccentKind::Dddot
            | AccentKind::Ddddot
            | AccentKind::Ring
    )
}

fn is_under_accent(kind: AccentKind) -> bool {
    matches!(
        kind,
        AccentKind::Underleftarrow
            | AccentKind::Underrightarrow
            | AccentKind::Underleftrightarrow
            | AccentKind::Underbrace
    )
}

fn accent_candidates(kind: AccentKind) -> &'static [char] {
    match kind {
        AccentKind::Hat | AccentKind::WideHat => &['\u{0302}', 'ˆ'],
        AccentKind::Check => &['ˇ', '\u{030C}'],
        AccentKind::Breve => &['˘', '\u{0306}'],
        AccentKind::Acute => &['´', '\u{0301}'],
        AccentKind::Grave => &['`', '\u{0300}'],
        AccentKind::Tilde | AccentKind::WideTilde => &['\u{0303}', '˜'],
        AccentKind::Bar => &['\u{0304}', '¯'],
        AccentKind::Vec | AccentKind::Overrightarrow => &['\u{20D7}', '→', '\u{27F6}'],
        AccentKind::Overleftarrow => &['\u{20D6}', '←', '\u{27F5}'],
        AccentKind::Overleftrightarrow => &['\u{20E1}', '↔', '\u{27F7}'],
        AccentKind::Underleftarrow => &['\u{20EE}', '←', '\u{27F5}'],
        AccentKind::Underrightarrow => &['\u{20EF}', '→', '\u{27F6}'],
        AccentKind::Underleftrightarrow => &['\u{034D}', '↔', '\u{27F7}'],
        AccentKind::Dot => &['˙', '\u{0307}'],
        AccentKind::Ddot => &['¨', '\u{0308}'],
        AccentKind::Dddot => &['\u{20DB}'],
        AccentKind::Ddddot => &['\u{20DC}'],
        AccentKind::Ring => &['˚', '\u{030A}'],
        AccentKind::Overbrace => &['⏞'],
        AccentKind::Underbrace => &['⏟'],
        AccentKind::Not
        | AccentKind::Overline
        | AccentKind::Underline
        | AccentKind::Cancel
        | AccentKind::BCancel
        | AccentKind::XCancel
        | AccentKind::Boxed => &[],
    }
}

fn single_glyph(name: &str) -> Option<char> {
    let e = lookup(name)?;
    let mut chars = e.glyph.chars();
    match (chars.next(), chars.next()) {
        (Some(c), None) => Some(c),
        _ => None,
    }
}

/// TeX's math-mode glyph for a bare source character.
///
/// `-` in math mode is `\mathchar"2200`, the minus sign, not the text hyphen.
/// The atom keeps its class, so binary-operator spacing is unchanged; `-`
/// inside `\text{...}` arrives as [`MathNode::LiteralText`] and is not mapped.
fn math_char(c: char) -> char {
    match c {
        '-' => '\u{2212}',
        other => other,
    }
}

/// TeX's default math-italic face for a variable letter.
///
/// Latin letters, lowercase Greek (with its variant forms), and `\partial` are
/// drawn from the Mathematical Italic block, as plain TeX's `\fam1` does.
/// Uppercase Greek, `\nabla`, digits and everything else stay upright. Explicit
/// font commands
/// Math-alphabet runs arrive as [`MathNode::MathAlphabet`] and literal text as
/// [`MathNode::LiteralText`]; both bypass default-variable remapping here.
fn is_default_math_variable(c: char) -> bool {
    let lower_greek = ('\u{03B1}'..='\u{03C9}').contains(&c);

    c.is_ascii_alphabetic() || lower_greek || matches!(c, 'ϵ' | 'ϑ' | 'ϰ' | 'ϕ' | 'ϱ' | 'ϖ' | '∂')
}

fn math_italic(c: char) -> char {
    if is_default_math_variable(c) {
        styled_char(c, TextStyle::It)
    } else {
        c
    }
}

// TeX appends a math-character italic correction when a bare variable
// participates directly in a row. Script attachment already owns the
// correction needed by scripted nuclei, so row packing must not add it
// again for Superscript/Subscript/SubSup nodes.
fn row_needs_math_italic_kern(node: &MathNode) -> bool {
    match node {
        MathNode::Atom(ch, class) => *class == AtomKind::Ord && is_default_math_variable(*ch),

        MathNode::Symbol(name) => {
            symbol_class(name) == AtomKind::Ord
                && symbol_char(name).ok().is_some_and(is_default_math_variable)
        }

        _ => false,
    }
}

fn symbol_char(name: &str) -> Result<char, Error> {
    single_glyph(name).ok_or_else(|| Error::Unsupported {
        what: format!("symbol \\{name}"),
    })
}

fn symbol_class(name: &str) -> AtomKind {
    symbol_atom_kind(name)
}

fn named_delim(n: &str) -> Result<char, Error> {
    let c = match n {
        "{" => '{',
        "}" => '}',
        "|" | "Vert" | "lVert" | "rVert" => '‖',
        "vert" | "lvert" | "rvert" => '|',
        "langle" => '⟨',
        "rangle" => '⟩',
        "lfloor" => '⌊',
        "rfloor" => '⌋',
        "lceil" => '⌈',
        "rceil" => '⌉',
        "backslash" => '\\',
        "uparrow" => '↑',
        "downarrow" => '↓',
        "Uparrow" => '⇑',
        "Downarrow" => '⇓',
        "updownarrow" => '↕',
        "Updownarrow" => '⇕',
        other => {
            return Err(Error::Unsupported {
                what: format!("delimiter {other}"),
            });
        }
    };
    Ok(c)
}

fn matrix_delims(s: MatrixStyle) -> (Option<char>, Option<char>) {
    match s {
        MatrixStyle::Pmatrix => (Some('('), Some(')')),
        MatrixStyle::Bmatrix => (Some('['), Some(']')),
        MatrixStyle::Vmatrix => (Some('|'), Some('|')),
        MatrixStyle::VVmatrix => (Some('‖'), Some('‖')),
        MatrixStyle::BBmatrix => (Some('{'), Some('}')),
        MatrixStyle::Cases => (Some('{'), None),
        _ => (None, None),
    }
}

#[cfg(test)]
mod tests {
    use super::variant_measure_is_better;
    use crate::Dim;

    #[test]
    fn variant_measure_prefers_tightest_satisfying_or_largest_short_candidate() {
        let target = Dim::one();
        let short = Dim::ratio(4, 5).expect("static ratio");
        let less_short = Dim::ratio(3, 4).expect("static ratio");
        let more_short = Dim::ratio(9, 10).expect("static ratio");
        let loose = Dim::ratio(6, 5).expect("static ratio");
        let tight = Dim::ratio(11, 10).expect("static ratio");
        let too_large = Dim::ratio(13, 10).expect("static ratio");

        assert!(variant_measure_is_better(&short, &more_short, &target));
        assert!(!variant_measure_is_better(&short, &less_short, &target));
        assert!(variant_measure_is_better(&short, &target, &target));
        assert!(!variant_measure_is_better(&target, &target, &target));
        assert!(variant_measure_is_better(&short, &loose, &target));
        assert!(variant_measure_is_better(&loose, &tight, &target));
        assert!(!variant_measure_is_better(&tight, &too_large, &target));
    }
}
