//! OpenType math font metrics. Integer font units → [`Dim`](crate::Dim).

use core::num::NonZeroU16;
use std::cell::OnceCell;
use std::sync::Arc;

use ttf_parser::{Face, FaceParsingError, RawFace, Tag};

use crate::dim::Dim;
use crate::error::{Error, FontError};

/// Horizontal glyph metrics in font units and em.
///
/// # Examples
///
/// ```no_run
/// use texpose::{Error, MathFont};
///
/// # fn font_bytes() -> &'static [u8] { unimplemented!() }
/// # fn main() -> Result<(), Error> {
/// let font = MathFont::from_bytes(font_bytes())?;
/// let g = font.glyph('x')?;
/// assert_eq!(g.ch, 'x');
/// assert!(!g.advance.is_zero());
/// # Ok(())
/// # }
/// ```
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GlyphMetrics {
    /// Character requested.
    pub ch: char,
    /// OpenType glyph id.
    pub glyph_id: u16,
    /// Horizontal advance, font units.
    pub advance_fu: u16,
    /// Advance in em.
    pub advance: Dim,
    /// Height above baseline in em (`max(y_max, 0)`).
    pub height: Dim,
    /// Depth below baseline in em (`max(-y_min, 0)`).
    pub depth: Dim,
}

/// One ready-made OpenType MATH glyph variant in normalized em units.
///
/// `advance` is the `MathGlyphVariantRecord.advanceMeasurement` in the
/// construction's growth direction. It is intentionally distinct from the
/// glyph's horizontal advance and ink bounding box.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct MathVariant {
    /// Glyph ID of the prepared variant.
    pub(crate) glyph_id: u16,
    /// Growth-direction `advanceMeasurement`, normalized to root-em units.
    pub(crate) advance: Dim,
}

/// One OpenType MATH glyph-assembly part in normalized em units.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct AssemblyPart {
    /// Glyph ID of this assembly part.
    pub(crate) glyph_id: u16,
    /// Connector length at the start of the part in the growth direction.
    pub(crate) start_connector: Dim,
    /// Connector length at the end of the part in the growth direction.
    pub(crate) end_connector: Dim,
    /// Full advance of this part in the growth direction.
    pub(crate) full_advance: Dim,
    /// Whether OpenType permits this part to be skipped or repeated.
    pub(crate) extender: bool,
}

/// Maximum number of materialized parts in one OpenType MATH glyph assembly.
///
/// The bound applies both while decoding font-controlled assembly data and to
/// the final repeated assembly selected by the layout solver.
pub(crate) const MAX_ASSEMBLY_PARTS: usize = 1024;

/// Font-boundary rejection while decoding a MATH glyph assembly.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum MathAssemblyDataError {
    /// The font-controlled source part count exceeds the layout allocation cap.
    PartBudgetExceeded,
    /// A glyph-assembly part sets a reserved OpenType flag bit.
    ReservedPartFlags,
    /// Raw MATH data declares an assembly/construction that cannot be parsed.
    MalformedAssemblyData,
}

#[derive(Clone, Copy)]
enum MathAssemblyAxis {
    Horizontal,
    Vertical,
}

fn assembly_extender_flag(
    flags: ttf_parser::math::PartFlags,
) -> Result<bool, MathAssemblyDataError> {
    if flags.0 & !0x0001 != 0 {
        return Err(MathAssemblyDataError::ReservedPartFlags);
    }
    Ok(flags.extender())
}

/// One OpenType MATH glyph assembly in normalized em units.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct GlyphAssembly {
    /// Italic correction of the complete assembly.
    pub(crate) italic_correction: Dim,
    /// Minimum legal overlap between neighboring part connectors.
    pub(crate) min_connector_overlap: Dim,
    /// Ordered assembly parts in the construction growth direction.
    pub(crate) parts: Vec<AssemblyPart>,
}

/// Loaded math font backed by shared caller-owned bytes.
///
/// `MathFont` owns no self-referential parser state. Clones share the same
/// immutable byte allocation, and each layout operation creates one temporary
/// parsed OpenType face from those bytes.
///
/// # Examples
///
/// ```no_run
/// use std::sync::Arc;
/// use texpose::{Error, MathFont};
///
/// # fn font_bytes() -> &'static [u8] { unimplemented!() }
/// # fn main() -> Result<(), Error> {
/// let raw: Arc<[u8]> = Arc::from(font_bytes());
/// let font = MathFont::from_shared_bytes(raw, 0)?;
/// assert!(font.units_per_em() > 0);
/// # Ok(())
/// # }
/// ```
#[derive(Clone)]
pub struct MathFont {
    inner: Arc<MathFontInner>,
}

struct MathFontInner {
    raw: Arc<[u8]>,
    face_index: u32,
}

pub(crate) struct MathFontView<'a> {
    face: Face<'a>,
    math_gsub_plan: OnceCell<MathGsubPlan>,
}

impl MathFont {
    /// Parse one standalone OpenType face from a borrowed byte slice.
    ///
    /// This compatibility constructor copies the supplied slice into shared
    /// storage. Collections require [`Self::from_bytes_at_index`] so face
    /// selection is explicit. Variable fonts are rejected by the first stable
    /// core.
    pub fn from_bytes(raw: &[u8]) -> Result<Self, FontError> {
        if ttf_parser::fonts_in_collection(raw).is_some() {
            return Err(FontError::CollectionFaceIndexRequired);
        }
        Self::from_shared_bytes(Arc::from(raw), 0)
    }

    /// Parse an OpenType face at `face_index` from a borrowed byte slice.
    ///
    /// This compatibility constructor copies the supplied slice into shared
    /// storage. `face_index` is `0` for standalone OTF/TTF data and selects a
    /// face for TTC/OTC collections.
    pub fn from_bytes_at_index(raw: &[u8], face_index: u32) -> Result<Self, FontError> {
        Self::from_shared_bytes(Arc::from(raw), face_index)
    }

    /// Parse an OpenType face from shared caller-owned bytes.
    ///
    /// The byte allocation is retained by reference counting without copying.
    /// `face_index` is `0` for standalone OTF/TTF data and selects a face for
    /// TTC/OTC collections. The selected face must be static; functional
    /// OpenType variation axes return [`FontError::VariableFontUnsupported`].
    pub fn from_shared_bytes(raw: Arc<[u8]>, face_index: u32) -> Result<Self, FontError> {
        {
            let raw_face = RawFace::parse(raw.as_ref(), face_index).map_err(map_face_error)?;
            if raw_face_has_variable_axes(&raw_face)? {
                return Err(FontError::VariableFontUnsupported);
            }
            validate_math_contract(&raw_face)?;
        }

        {
            let face = Face::parse(raw.as_ref(), face_index).map_err(map_face_error)?;
            NonZeroU16::new(face.units_per_em()).ok_or(FontError::InvalidFace)?;
            let math = face.tables().math.ok_or(FontError::MalformedMathTable)?;
            math.constants.ok_or(FontError::MalformedMathConstants)?;
        }

        Ok(Self {
            inner: Arc::new(MathFontInner { raw, face_index }),
        })
    }

    fn parse_face(&self) -> Face<'_> {
        Face::parse(self.bytes(), self.face_index())
            .expect("MathFont bytes and face index were validated at construction")
    }

    pub(crate) fn operation_view(&self) -> MathFontView<'_> {
        MathFontView {
            face: self.parse_face(),
            math_gsub_plan: OnceCell::new(),
        }
    }

    /// Parse and return an OpenType face view over the retained bytes.
    ///
    /// Native consumers may need glyph outlines and bounding boxes that this
    /// crate does not otherwise expose. The returned view borrows `self`; no
    /// OpenType face is stored self-referentially inside [`MathFont`].
    ///
    /// The crate re-exports [`ttf_parser`] so consumers can name the exact parser
    /// version used by TeXpose.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use texpose::{ttf_parser, Error, MathFont};
    ///
    /// # fn font_bytes() -> &'static [u8] { unimplemented!() }
    /// # fn main() -> Result<(), Error> {
    /// let font = MathFont::from_bytes(font_bytes())?;
    /// let metrics = font.glyph('x')?;
    /// let id = ttf_parser::GlyphId(metrics.glyph_id);
    /// assert!(font.face().glyph_bounding_box(id).is_some());
    /// # Ok(())
    /// # }
    /// ```
    #[must_use]
    pub fn face(&self) -> Face<'_> {
        self.parse_face()
    }

    /// OpenType bytes retained by this font.
    #[must_use]
    pub fn bytes(&self) -> &[u8] {
        self.inner.raw.as_ref()
    }

    /// Clone the shared OpenType byte allocation.
    #[must_use]
    pub fn shared_bytes(&self) -> Arc<[u8]> {
        Arc::clone(&self.inner.raw)
    }

    /// Face index used to parse this font.
    ///
    /// Standalone OTF/TTF faces use index `0`; TTC/OTC collections require an
    /// explicitly selected index.
    #[must_use]
    pub fn face_index(&self) -> u32 {
        self.inner.face_index
    }

    /// `unitsPerEm` from the `head` table.
    #[must_use]
    pub fn units_per_em(&self) -> u16 {
        self.face().units_per_em()
    }

    /// `hhea` ascender in font units.
    #[must_use]
    pub fn ascender_fu(&self) -> i16 {
        self.face().ascender()
    }

    /// `hhea` descender in font units (typically negative).
    #[must_use]
    pub fn descender_fu(&self) -> i16 {
        self.face().descender()
    }

    /// Ascender in em.
    #[must_use]
    pub fn ascender(&self) -> Dim {
        let font = self.operation_view();
        font.font_units(i64::from(font.face.ascender()))
    }

    /// Depth below baseline from `hhea` descender, in em (non-negative).
    #[must_use]
    pub fn descender(&self) -> Dim {
        let font = self.operation_view();
        let d = i64::from(font.face.descender());
        font.font_units(-d)
    }

    /// Metrics for `ch`, or [`FontError::MissingGlyph`].
    pub fn glyph(&self, ch: char) -> Result<GlyphMetrics, Error> {
        self.operation_view().glyph(ch)
    }

    /// Metrics for OpenType glyph id `gid`, tagged with `ch` for the box payload.
    pub fn glyph_id(&self, ch: char, gid: u16) -> Result<GlyphMetrics, Error> {
        self.operation_view().glyph_id(ch, gid)
    }

    /// MATH italic correction for `glyph_id`, or zero.
    pub fn italic_correction(&self, glyph_id: u16) -> Dim {
        self.operation_view().italic_correction(glyph_id)
    }

    /// MATH top-accent attachment (em from glyph left), if present.
    pub fn top_accent_attachment(&self, glyph_id: u16) -> Option<Dim> {
        self.operation_view().top_accent_attachment(glyph_id)
    }

    /// Horizontal glyph-assembly parts: `(gid, start_connector, end_connector, advance, extender)`.
    /// Lengths are raw font units. Layout itself consumes the internal typed
    /// [`Dim`] representation instead of this compatibility view. Assemblies
    /// above the internal part budget return an empty compatibility view.
    pub fn horizontal_assembly_parts(&self, glyph_id: u16) -> Vec<(u16, u16, u16, u16, bool)> {
        self.operation_view().horizontal_assembly_parts_fu(glyph_id)
    }

    /// Horizontal MATH variant glyph IDs, including the base glyph first.
    ///
    /// The layout engine consumes the corresponding typed variant records so
    /// `advanceMeasurement` remains attached to each glyph ID.
    pub fn horizontal_variants(&self, glyph_id: u16) -> Vec<u16> {
        let font = self.operation_view();
        let mut out = vec![glyph_id];
        out.extend(
            font.horizontal_variant_records(glyph_id)
                .into_iter()
                .map(|variant| variant.glyph_id),
        );
        out
    }

    /// Vertical MATH variant glyph IDs, including the base glyph first.
    ///
    /// The layout engine consumes the corresponding typed variant records so
    /// `advanceMeasurement` remains attached to each glyph ID.
    pub fn vertical_variants(&self, glyph_id: u16) -> Vec<u16> {
        let font = self.operation_view();
        let mut out = vec![glyph_id];
        out.extend(
            font.vertical_variant_records(glyph_id)
                .into_iter()
                .map(|variant| variant.glyph_id),
        );
        out
    }

    /// SHA-256 hex of arbitrary font bytes.
    #[must_use]
    pub fn sha256_hex(bytes: &[u8]) -> String {
        let d = crate::hash::sha256(bytes);
        let mut s = String::with_capacity(64);
        for b in d {
            s.push_str(&hex_byte(b));
        }
        s
    }
}

impl<'a> MathFontView<'a> {
    pub(crate) fn face(&self) -> &Face<'a> {
        &self.face
    }

    pub(crate) fn math_table(&self) -> ttf_parser::math::Table<'a> {
        self.face
            .tables()
            .math
            .expect("MathFont validates the MATH table at construction")
    }

    pub(crate) fn math_constants(&self) -> ttf_parser::math::Constants<'a> {
        self.math_table()
            .constants
            .expect("MathFont validates MathConstants at construction")
    }

    pub(crate) fn units_per_em_nonzero(&self) -> NonZeroU16 {
        NonZeroU16::new(self.face.units_per_em())
            .expect("MathFont unitsPerEm was validated at construction")
    }

    /// Convert validated OpenType design units into the engine's exact em unit.
    ///
    /// `MathFont` construction proves `unitsPerEm != 0`; all MATH-table design
    /// unit conversion goes through this method so unit provenance stays local
    /// to the validated face.
    pub(crate) fn font_units(&self, units: i64) -> Dim {
        Dim::from_font_units_nonzero(units, self.units_per_em_nonzero())
    }

    /// Convert the design-unit component of a MATH value into exact em units.
    ///
    /// Device/PPEM corrections remain deliberately ignored by project policy.
    pub(crate) fn math_value(&self, value: ttf_parser::math::MathValue<'_>) -> Dim {
        self.font_units(i64::from(value.value))
    }

    pub(crate) fn glyph_index(&self, ch: char) -> Option<u16> {
        self.face.glyph_index(ch).map(|glyph_id| glyph_id.0)
    }

    pub(crate) fn glyph(&self, ch: char) -> Result<GlyphMetrics, Error> {
        let gid = self.glyph_index(ch).ok_or(FontError::MissingGlyph { ch })?;
        self.glyph_id(ch, gid)
    }

    pub(crate) fn glyph_id(&self, ch: char, gid: u16) -> Result<GlyphMetrics, Error> {
        let gid = ttf_parser::GlyphId(gid);
        let advance_fu = self
            .face
            .glyph_hor_advance(gid)
            .ok_or(FontError::MissingGlyph { ch })?;
        let mut height_fu = 0i64;
        let mut depth_fu = 0i64;
        if let Some(bbox) = self.face.glyph_bounding_box(gid) {
            height_fu = i64::from(bbox.y_max).max(0);
            depth_fu = i64::from(-bbox.y_min).max(0);
        }
        Ok(GlyphMetrics {
            ch,
            glyph_id: gid.0,
            advance_fu,
            advance: self.font_units(i64::from(advance_fu)),
            height: self.font_units(height_fu),
            depth: self.font_units(depth_fu),
        })
    }

    pub(crate) fn italic_correction(&self, glyph_id: u16) -> Dim {
        let math = self.math_table();
        let Some(info) = math.glyph_info else {
            return Dim::zero();
        };
        let Some(table) = info.italic_corrections else {
            return Dim::zero();
        };
        match table.get(ttf_parser::GlyphId(glyph_id)) {
            Some(value) => self.math_value(value),
            None => Dim::zero(),
        }
    }

    pub(crate) fn top_accent_attachment(&self, glyph_id: u16) -> Option<Dim> {
        let math = self.math_table();
        let info = math.glyph_info?;
        let table = info.top_accent_attachments?;
        let value = table.get(ttf_parser::GlyphId(glyph_id))?;
        Some(self.math_value(value))
    }

    fn variant_records(
        &self,
        construction: ttf_parser::math::GlyphConstruction<'_>,
    ) -> Vec<MathVariant> {
        let mut out = Vec::with_capacity(usize::from(construction.variants.len()));
        for index in 0..construction.variants.len() {
            let variant = construction
                .variants
                .get(index)
                .expect("parsed MATH variant index is within its validated array");
            out.push(MathVariant {
                glyph_id: variant.variant_glyph.0,
                advance: self.font_units(i64::from(variant.advance_measurement)),
            });
        }
        out
    }

    pub(crate) fn horizontal_variant_records(&self, glyph_id: u16) -> Vec<MathVariant> {
        let Some(variants) = self.math_table().variants else {
            return Vec::new();
        };
        let Some(construction) = variants
            .horizontal_constructions
            .get(ttf_parser::GlyphId(glyph_id))
        else {
            return Vec::new();
        };
        self.variant_records(construction)
    }

    pub(crate) fn vertical_variant_records(&self, glyph_id: u16) -> Vec<MathVariant> {
        let Some(variants) = self.math_table().variants else {
            return Vec::new();
        };
        let Some(construction) = variants
            .vertical_constructions
            .get(ttf_parser::GlyphId(glyph_id))
        else {
            return Vec::new();
        };
        self.variant_records(construction)
    }

    fn glyph_assembly(
        &self,
        construction: ttf_parser::math::GlyphConstruction<'_>,
        min_connector_overlap: u16,
    ) -> Result<Option<GlyphAssembly>, MathAssemblyDataError> {
        let Some(assembly) = construction.assembly else {
            return Ok(None);
        };
        let part_count = usize::from(assembly.parts.len());
        if part_count > MAX_ASSEMBLY_PARTS {
            return Err(MathAssemblyDataError::PartBudgetExceeded);
        }

        let mut parts = Vec::with_capacity(part_count);
        for index in 0..assembly.parts.len() {
            let part = assembly
                .parts
                .get(index)
                .expect("parsed MATH assembly index is within its validated array");
            parts.push(AssemblyPart {
                glyph_id: part.glyph_id.0,
                start_connector: self.font_units(i64::from(part.start_connector_length)),
                end_connector: self.font_units(i64::from(part.end_connector_length)),
                full_advance: self.font_units(i64::from(part.full_advance)),
                extender: assembly_extender_flag(part.part_flags)?,
            });
        }
        Ok(Some(GlyphAssembly {
            italic_correction: self.math_value(assembly.italics_correction),
            min_connector_overlap: self.font_units(i64::from(min_connector_overlap)),
            parts,
        }))
    }

    fn glyph_assembly_for_axis(
        &self,
        glyph_id: u16,
        axis: MathAssemblyAxis,
    ) -> Result<Option<GlyphAssembly>, MathAssemblyDataError> {
        let raw_math = self
            .face
            .raw_face()
            .table(Tag::from_bytes(b"MATH"))
            .ok_or(MathAssemblyDataError::MalformedAssemblyData)?;
        let declared = raw_math_assembly_declared(raw_math, glyph_id, axis)?;
        if !declared {
            return Ok(None);
        }

        let variants = self
            .math_table()
            .variants
            .ok_or(MathAssemblyDataError::MalformedAssemblyData)?;
        let construction = match axis {
            MathAssemblyAxis::Horizontal => variants
                .horizontal_constructions
                .get(ttf_parser::GlyphId(glyph_id)),
            MathAssemblyAxis::Vertical => variants
                .vertical_constructions
                .get(ttf_parser::GlyphId(glyph_id)),
        }
        .ok_or(MathAssemblyDataError::MalformedAssemblyData)?;
        if construction.assembly.is_none() {
            return Err(MathAssemblyDataError::MalformedAssemblyData);
        }
        self.glyph_assembly(construction, variants.min_connector_overlap)
    }

    pub(crate) fn horizontal_assembly(
        &self,
        glyph_id: u16,
    ) -> Result<Option<GlyphAssembly>, MathAssemblyDataError> {
        self.glyph_assembly_for_axis(glyph_id, MathAssemblyAxis::Horizontal)
    }

    pub(crate) fn vertical_assembly(
        &self,
        glyph_id: u16,
    ) -> Result<Option<GlyphAssembly>, MathAssemblyDataError> {
        self.glyph_assembly_for_axis(glyph_id, MathAssemblyAxis::Vertical)
    }

    // Compatibility view for callers that explicitly require raw font-unit
    // assembly data. Layout must not consume this untyped adapter.
    pub(crate) fn horizontal_assembly_parts_fu(
        &self,
        glyph_id: u16,
    ) -> Vec<(u16, u16, u16, u16, bool)> {
        let mut out = Vec::new();
        let Some(variants) = self.math_table().variants else {
            return out;
        };
        let Some(construction) = variants
            .horizontal_constructions
            .get(ttf_parser::GlyphId(glyph_id))
        else {
            return out;
        };
        let Some(assembly) = construction.assembly else {
            return out;
        };
        let part_count = usize::from(assembly.parts.len());
        if part_count > MAX_ASSEMBLY_PARTS {
            return out;
        }
        out.reserve(part_count);
        for index in 0..assembly.parts.len() {
            let part = assembly
                .parts
                .get(index)
                .expect("parsed MATH assembly index is within its validated array");
            let Ok(extender) = assembly_extender_flag(part.part_flags) else {
                return Vec::new();
            };
            out.push((
                part.glyph_id.0,
                part.start_connector_length,
                part.end_connector_length,
                part.full_advance,
                extender,
            ));
        }
        out
    }

    #[inline]
    pub(crate) fn math_gsub_glyph_id(
        &self,
        glyph_id: u16,
        script_level: u8,
        context: MathGsubContext,
    ) -> u16 {
        if !matches!(script_level, 1 | 2) && context == MathGsubContext::None {
            return glyph_id;
        }

        let plan = self
            .math_gsub_plan
            .get_or_init(|| MathGsubPlan::from_face(&self.face));
        apply_math_gsub_plan(&self.face, plan, glyph_id, script_level, context)
    }
}

fn raw_math_assembly_declared(
    math: &[u8],
    glyph_id: u16,
    axis: MathAssemblyAxis,
) -> Result<bool, MathAssemblyDataError> {
    let variants_offset = read_be_u16(math, 8)?;
    if variants_offset == 0 {
        return Ok(false);
    }
    let variants = math
        .get(usize::from(variants_offset)..)
        .ok_or(MathAssemblyDataError::MalformedAssemblyData)?;
    if variants.len() < 10 {
        return Err(MathAssemblyDataError::MalformedAssemblyData);
    }

    let vertical_count = usize::from(read_be_u16(variants, 6)?);
    let horizontal_count = usize::from(read_be_u16(variants, 8)?);
    let (coverage_offset, construction_start, construction_count) = match axis {
        MathAssemblyAxis::Vertical => (read_be_u16(variants, 2)?, 10usize, vertical_count),
        MathAssemblyAxis::Horizontal => {
            let vertical_bytes = vertical_count
                .checked_mul(2)
                .ok_or(MathAssemblyDataError::MalformedAssemblyData)?;
            let start = 10usize
                .checked_add(vertical_bytes)
                .ok_or(MathAssemblyDataError::MalformedAssemblyData)?;
            (read_be_u16(variants, 4)?, start, horizontal_count)
        }
    };
    if construction_count == 0 {
        return Ok(false);
    }
    if coverage_offset == 0 {
        return Err(MathAssemblyDataError::MalformedAssemblyData);
    }

    let Some(coverage_index) = coverage_index(variants, coverage_offset, glyph_id)? else {
        return Ok(false);
    };
    if coverage_index >= construction_count {
        return Err(MathAssemblyDataError::MalformedAssemblyData);
    }

    let entry_offset = construction_start
        .checked_add(
            coverage_index
                .checked_mul(2)
                .ok_or(MathAssemblyDataError::MalformedAssemblyData)?,
        )
        .ok_or(MathAssemblyDataError::MalformedAssemblyData)?;
    let construction_offset = read_be_u16(variants, entry_offset)?;
    if construction_offset == 0 {
        return Err(MathAssemblyDataError::MalformedAssemblyData);
    }
    let construction_start = usize::from(construction_offset);
    let construction = variants
        .get(construction_start..)
        .ok_or(MathAssemblyDataError::MalformedAssemblyData)?;
    let assembly_offset = read_be_u16(construction, 0)?;
    if assembly_offset == 0 {
        return Ok(false);
    }

    let assembly_start = construction_start
        .checked_add(usize::from(assembly_offset))
        .ok_or(MathAssemblyDataError::MalformedAssemblyData)?;
    let assembly_header_end = assembly_start
        .checked_add(6)
        .ok_or(MathAssemblyDataError::MalformedAssemblyData)?;
    if assembly_header_end > variants.len() {
        return Err(MathAssemblyDataError::MalformedAssemblyData);
    }
    Ok(true)
}

fn coverage_index(
    table: &[u8],
    offset: u16,
    glyph_id: u16,
) -> Result<Option<usize>, MathAssemblyDataError> {
    let coverage = table
        .get(usize::from(offset)..)
        .ok_or(MathAssemblyDataError::MalformedAssemblyData)?;
    match read_be_u16(coverage, 0)? {
        1 => {
            let count = usize::from(read_be_u16(coverage, 2)?);
            let bytes = count
                .checked_mul(2)
                .and_then(|value| value.checked_add(4))
                .ok_or(MathAssemblyDataError::MalformedAssemblyData)?;
            if coverage.len() < bytes {
                return Err(MathAssemblyDataError::MalformedAssemblyData);
            }
            for index in 0..count {
                let entry = read_be_u16(coverage, 4 + index * 2)?;
                if entry == glyph_id {
                    return Ok(Some(index));
                }
            }
            Ok(None)
        }
        2 => {
            let count = usize::from(read_be_u16(coverage, 2)?);
            let bytes = count
                .checked_mul(6)
                .and_then(|value| value.checked_add(4))
                .ok_or(MathAssemblyDataError::MalformedAssemblyData)?;
            if coverage.len() < bytes {
                return Err(MathAssemblyDataError::MalformedAssemblyData);
            }
            let mut previous_end: Option<u16> = None;
            for index in 0..count {
                let record = 4 + index * 6;
                let start = read_be_u16(coverage, record)?;
                let end = read_be_u16(coverage, record + 2)?;
                let start_index = usize::from(read_be_u16(coverage, record + 4)?);
                if start > end || previous_end.is_some_and(|previous| start <= previous) {
                    return Err(MathAssemblyDataError::MalformedAssemblyData);
                }
                if glyph_id >= start && glyph_id <= end {
                    let delta = usize::from(glyph_id - start);
                    return start_index
                        .checked_add(delta)
                        .map(Some)
                        .ok_or(MathAssemblyDataError::MalformedAssemblyData);
                }
                previous_end = Some(end);
            }
            Ok(None)
        }
        _ => Err(MathAssemblyDataError::MalformedAssemblyData),
    }
}

fn read_be_u16(bytes: &[u8], offset: usize) -> Result<u16, MathAssemblyDataError> {
    let end = offset
        .checked_add(2)
        .ok_or(MathAssemblyDataError::MalformedAssemblyData)?;
    let raw = bytes
        .get(offset..end)
        .ok_or(MathAssemblyDataError::MalformedAssemblyData)?;
    Ok(u16::from_be_bytes([raw[0], raw[1]]))
}

const MATH_HEADER_LEN: usize = 10;
const MATH_CONSTANTS_LEN: usize = 214;

fn map_face_error(error: FaceParsingError) -> FontError {
    match error {
        FaceParsingError::FaceIndexOutOfBounds => FontError::FaceIndexOutOfBounds,
        _ => FontError::InvalidFace,
    }
}

fn validate_math_contract(raw_face: &RawFace<'_>) -> Result<(), FontError> {
    let math = raw_math_table(raw_face)?;
    if math.len() < MATH_HEADER_LEN {
        return Err(FontError::MalformedMathTable);
    }

    let major = u16::from_be_bytes([math[0], math[1]]);
    let minor = u16::from_be_bytes([math[2], math[3]]);
    if (major, minor) != (1, 0) {
        return Err(FontError::MalformedMathTable);
    }

    let constants_offset = usize::from(u16::from_be_bytes([math[4], math[5]]));
    if constants_offset == 0 {
        return Err(FontError::MissingMathConstants);
    }
    if constants_offset < MATH_HEADER_LEN {
        return Err(FontError::MalformedMathConstants);
    }
    let constants_end = constants_offset
        .checked_add(MATH_CONSTANTS_LEN)
        .ok_or(FontError::MalformedMathConstants)?;
    if constants_end > math.len() {
        return Err(FontError::MalformedMathConstants);
    }

    let parsed = ttf_parser::math::Table::parse(math).ok_or(FontError::MalformedMathTable)?;
    parsed.constants.ok_or(FontError::MalformedMathConstants)?;
    Ok(())
}

fn raw_math_table<'a>(raw_face: &RawFace<'a>) -> Result<&'a [u8], FontError> {
    let math_tag = Tag::from_bytes(b"MATH");
    let mut found = None;
    for index in 0..raw_face.table_records.len() {
        let record = raw_face
            .table_records
            .get(index)
            .ok_or(FontError::MalformedMathTable)?;
        if record.tag == math_tag {
            if found.is_some() {
                return Err(FontError::MalformedMathTable);
            }
            found = Some(record);
        }
    }

    let record = found.ok_or(FontError::MissingMathTable)?;
    let offset = usize::try_from(record.offset).map_err(|_| FontError::MalformedMathTable)?;
    let length = usize::try_from(record.length).map_err(|_| FontError::MalformedMathTable)?;
    let end = offset
        .checked_add(length)
        .ok_or(FontError::MalformedMathTable)?;
    raw_face
        .data
        .get(offset..end)
        .ok_or(FontError::MalformedMathTable)
}

fn raw_face_has_variable_axes(raw_face: &RawFace<'_>) -> Result<bool, FontError> {
    let Some(fvar) = raw_face.table(Tag::from_bytes(b"fvar")) else {
        return Ok(false);
    };

    // OpenType fvar 1.0 header through axisSize. A functional variable font
    // has axisCount > 0; axisCount == 0 is explicitly treated as non-variable.
    if fvar.len() < 16 {
        return Err(FontError::InvalidFace);
    }

    let major = u16::from_be_bytes([fvar[0], fvar[1]]);
    let axes_offset = usize::from(u16::from_be_bytes([fvar[4], fvar[5]]));
    let reserved = u16::from_be_bytes([fvar[6], fvar[7]]);
    let axis_count = usize::from(u16::from_be_bytes([fvar[8], fvar[9]]));
    let axis_size = usize::from(u16::from_be_bytes([fvar[10], fvar[11]]));

    if major != 1 || reserved != 2 || axes_offset < 16 || axis_size < 20 {
        return Err(FontError::InvalidFace);
    }
    if axis_count == 0 {
        return Ok(false);
    }

    let axes_len = axis_count
        .checked_mul(axis_size)
        .ok_or(FontError::InvalidFace)?;
    let axes_end = axes_offset
        .checked_add(axes_len)
        .ok_or(FontError::InvalidFace)?;
    if axes_end > fvar.len() {
        return Err(FontError::InvalidFace);
    }

    Ok(true)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum MathGsubContext {
    None,
    FlattenedAccent,
    DotlessAccentBase,
}

const MATH_GSUB_SSTY: u8 = 1 << 0;
const MATH_GSUB_FLAC: u8 = 1 << 1;
const MATH_GSUB_DTLS: u8 = 1 << 2;

#[derive(Default)]
struct MathGsubPlan {
    lookup_features: Vec<u8>,
}

impl MathGsubPlan {
    fn from_face(face: &Face<'_>) -> Self {
        let Some(gsub) = face.tables().gsub else {
            return Self::default();
        };
        let Some(math_script) = gsub.scripts.find(Tag::from_bytes(b"math")) else {
            return Self::default();
        };
        let Some(language) = math_script.default_language else {
            return Self::default();
        };

        let lookup_count = usize::from(gsub.lookups.len());
        let mut lookup_features = Vec::new();
        if let Some(feature_index) = language.required_feature {
            mark_math_gsub_feature(&gsub, feature_index, lookup_count, &mut lookup_features);
        }
        for feature_index in language.feature_indices {
            mark_math_gsub_feature(&gsub, feature_index, lookup_count, &mut lookup_features);
        }

        if lookup_features.iter().all(|features| *features == 0) {
            return Self::default();
        }

        Self { lookup_features }
    }
}

fn mark_math_gsub_feature(
    gsub: &ttf_parser::opentype_layout::LayoutTable<'_>,
    feature_index: u16,
    lookup_count: usize,
    lookup_features: &mut Vec<u8>,
) {
    let Some(feature) = gsub.features.get(feature_index) else {
        return;
    };
    let feature_flag = if feature.tag == Tag::from_bytes(b"ssty") {
        MATH_GSUB_SSTY
    } else if feature.tag == Tag::from_bytes(b"flac") {
        MATH_GSUB_FLAC
    } else if feature.tag == Tag::from_bytes(b"dtls") {
        MATH_GSUB_DTLS
    } else {
        return;
    };

    if lookup_features.is_empty() {
        lookup_features.resize(lookup_count, 0);
    }
    for lookup_index in feature.lookup_indices {
        if let Some(flags) = lookup_features.get_mut(usize::from(lookup_index)) {
            *flags |= feature_flag;
        }
    }
}

fn apply_math_gsub_plan(
    face: &Face<'_>,
    plan: &MathGsubPlan,
    glyph_id: u16,
    script_level: u8,
    context: MathGsubContext,
) -> u16 {
    if plan.lookup_features.is_empty() {
        return glyph_id;
    }

    let Some(gsub) = face.tables().gsub else {
        return glyph_id;
    };
    let context_flag = match context {
        MathGsubContext::None => 0,
        MathGsubContext::FlattenedAccent => MATH_GSUB_FLAC,
        MathGsubContext::DotlessAccentBase => MATH_GSUB_DTLS,
    };

    let mut glyph = ttf_parser::GlyphId(glyph_id);
    for lookup_index in 0..gsub.lookups.len() {
        let features = plan
            .lookup_features
            .get(usize::from(lookup_index))
            .copied()
            .unwrap_or(0);
        let ssty = matches!(script_level, 1 | 2) && features & MATH_GSUB_SSTY != 0;
        let contextual = context_flag != 0 && features & context_flag != 0;
        if !ssty && !contextual {
            continue;
        }

        let Some(lookup) = gsub.lookups.get(lookup_index) else {
            continue;
        };
        for subtable in lookup
            .subtables
            .into_iter::<ttf_parser::gsub::SubstitutionSubtable<'_>>()
        {
            let substituted = match subtable {
                ttf_parser::gsub::SubstitutionSubtable::Single(single) => {
                    single_substitution_glyph(single, glyph)
                }
                ttf_parser::gsub::SubstitutionSubtable::Alternate(alternate) if ssty => {
                    ssty_alternate_glyph(alternate, glyph, script_level)
                }
                _ => None,
            };
            if let Some(next) = substituted {
                glyph = next;
                break;
            }
        }
    }

    glyph.0
}

fn single_substitution_glyph(
    single: ttf_parser::gsub::SingleSubstitution<'_>,
    glyph: ttf_parser::GlyphId,
) -> Option<ttf_parser::GlyphId> {
    match single {
        ttf_parser::gsub::SingleSubstitution::Format1 { coverage, delta } => {
            coverage.get(glyph)?;
            Some(ttf_parser::GlyphId(glyph.0.wrapping_add_signed(delta)))
        }
        ttf_parser::gsub::SingleSubstitution::Format2 {
            coverage,
            substitutes,
        } => {
            let coverage_index = coverage.get(glyph)?;
            substitutes.get(coverage_index)
        }
    }
}

fn ssty_alternate_glyph(
    alternate: ttf_parser::gsub::AlternateSubstitution<'_>,
    glyph: ttf_parser::GlyphId,
    script_level: u8,
) -> Option<ttf_parser::GlyphId> {
    let alternate_index = match script_level {
        1 => 0,
        2 => 1,
        _ => return None,
    };
    let coverage_index = alternate.coverage.get(glyph)?;
    let set = alternate.alternate_sets.get(coverage_index)?;
    set.alternates.get(alternate_index)
}

fn hex_byte(b: u8) -> String {
    const H: &[u8; 16] = b"0123456789abcdef";
    let hi = H[(b >> 4) as usize];
    let lo = H[(b & 0xf) as usize];
    let mut out = String::with_capacity(2);
    out.push(hi as char);
    out.push(lo as char);
    out
}

#[cfg(test)]
mod tests {
    use super::{MathFont, MathFontView};

    const STIX_TWO_MATH: &[u8] =
        include_bytes!("../../tests/fixtures/fonts/stix-two-math/STIXTwoMath-Regular.otf");

    fn stix() -> MathFont {
        MathFont::from_bytes(STIX_TWO_MATH).expect("STIX Two Math fixture")
    }

    #[test]
    fn typed_variant_records_preserve_opentype_advance_measurement() {
        // E11's pinned access census records a horizontal construction at gid
        // 732 for this exact fixture. Keep the test tied to the pinned bytes so
        // an intentional fixture update must review the construction evidence.
        const GLYPH_ID: u16 = 732;

        let font = stix();
        let view = font.operation_view();
        let typed = view.horizontal_variant_records(GLYPH_ID);
        let variants = view.math_table().variants.expect("MATH variants");
        let raw = variants
            .horizontal_constructions
            .get(ttf_parser::GlyphId(GLYPH_ID))
            .expect("horizontal construction");

        assert!(
            !typed.is_empty(),
            "fixture needs prepared horizontal variants"
        );
        assert_eq!(typed.len(), usize::from(raw.variants.len()));
        for (index, typed_variant) in typed.iter().enumerate() {
            let raw_variant = raw
                .variants
                .get(u16::try_from(index).expect("variant index fits u16"))
                .expect("raw variant");
            assert_eq!(typed_variant.glyph_id, raw_variant.variant_glyph.0);
            assert_eq!(
                typed_variant.advance,
                crate::Dim::from_font_units(
                    i64::from(raw_variant.advance_measurement),
                    font.units_per_em(),
                )
                .expect("validated unitsPerEm")
            );
        }
    }

    #[test]
    fn typed_vertical_variant_records_preserve_opentype_advance_measurement() {
        let font = stix();
        let view = font.operation_view();
        let glyph_id = view.glyph_index('(').expect("parenthesis cmap glyph");
        let typed = view.vertical_variant_records(glyph_id);
        let variants = view.math_table().variants.expect("MATH variants");
        let raw = variants
            .vertical_constructions
            .get(ttf_parser::GlyphId(glyph_id))
            .expect("vertical parenthesis construction");

        assert!(
            !typed.is_empty(),
            "fixture needs prepared vertical variants"
        );
        assert_eq!(typed.len(), usize::from(raw.variants.len()));
        for (index, typed_variant) in typed.iter().enumerate() {
            let raw_variant = raw
                .variants
                .get(u16::try_from(index).expect("variant index fits u16"))
                .expect("raw variant");
            assert_eq!(typed_variant.glyph_id, raw_variant.variant_glyph.0);
            assert_eq!(
                typed_variant.advance,
                crate::Dim::from_font_units(
                    i64::from(raw_variant.advance_measurement),
                    font.units_per_em(),
                )
                .expect("validated unitsPerEm")
            );
        }
    }

    #[test]
    fn typed_horizontal_assembly_preserves_all_design_unit_fields() {
        // E11's pinned access census records a horizontal assembly at gid 746.
        const GLYPH_ID: u16 = 746;

        let font = stix();
        let view = font.operation_view();
        let typed = view
            .horizontal_assembly(GLYPH_ID)
            .expect("assembly is within the part budget")
            .expect("typed horizontal assembly");
        let variants = view.math_table().variants.expect("MATH variants");
        let raw = variants
            .horizontal_constructions
            .get(ttf_parser::GlyphId(GLYPH_ID))
            .and_then(|construction| construction.assembly)
            .expect("raw horizontal assembly");

        assert_eq!(
            typed.italic_correction,
            crate::Dim::from_font_units(
                i64::from(raw.italics_correction.value),
                font.units_per_em(),
            )
            .expect("validated unitsPerEm")
        );
        assert_eq!(
            typed.min_connector_overlap,
            crate::Dim::from_font_units(
                i64::from(variants.min_connector_overlap),
                font.units_per_em(),
            )
            .expect("validated unitsPerEm")
        );
        assert_eq!(typed.parts.len(), usize::from(raw.parts.len()));
        for (index, typed_part) in typed.parts.iter().enumerate() {
            let raw_part = raw
                .parts
                .get(u16::try_from(index).expect("assembly index fits u16"))
                .expect("raw assembly part");
            assert_eq!(typed_part.glyph_id, raw_part.glyph_id.0);
            assert_eq!(
                typed_part.start_connector,
                crate::Dim::from_font_units(
                    i64::from(raw_part.start_connector_length),
                    font.units_per_em(),
                )
                .expect("validated unitsPerEm")
            );
            assert_eq!(
                typed_part.end_connector,
                crate::Dim::from_font_units(
                    i64::from(raw_part.end_connector_length),
                    font.units_per_em(),
                )
                .expect("validated unitsPerEm")
            );
            assert_eq!(
                typed_part.full_advance,
                crate::Dim::from_font_units(i64::from(raw_part.full_advance), font.units_per_em())
                    .expect("validated unitsPerEm")
            );
            assert_eq!(typed_part.extender, raw_part.part_flags.extender());
        }
    }

    #[test]
    fn typed_vertical_assembly_preserves_min_overlap_and_part_fields() {
        let font = stix();
        let view = font.operation_view();
        let glyph_id = view.glyph_index('|').expect("fixture vertical-bar glyph");
        let typed = view
            .vertical_assembly(glyph_id)
            .expect("assembly is within the part budget")
            .expect("fixture vertical assembly");
        let variants = view.math_table().variants.expect("MATH variants");
        let raw = variants
            .vertical_constructions
            .get(ttf_parser::GlyphId(glyph_id))
            .and_then(|construction| construction.assembly)
            .expect("raw vertical assembly");

        assert_eq!(
            typed.min_connector_overlap,
            crate::Dim::from_font_units(
                i64::from(variants.min_connector_overlap),
                font.units_per_em(),
            )
            .expect("validated unitsPerEm")
        );
        assert_eq!(typed.parts.len(), usize::from(raw.parts.len()));
        assert!(typed.parts.iter().any(|part| part.extender));
        for (index, typed_part) in typed.parts.iter().enumerate() {
            let raw_part = raw
                .parts
                .get(u16::try_from(index).expect("assembly index fits u16"))
                .expect("raw vertical assembly part");
            assert_eq!(typed_part.glyph_id, raw_part.glyph_id.0);
            assert_eq!(
                typed_part.full_advance,
                crate::Dim::from_font_units(i64::from(raw_part.full_advance), font.units_per_em())
                    .expect("validated unitsPerEm")
            );
        }
    }

    #[test]
    fn public_variant_id_views_project_the_typed_records_without_reordering() {
        let font = stix();
        let view = font.operation_view();

        let horizontal_base = 732;
        let mut expected_horizontal = vec![horizontal_base];
        expected_horizontal.extend(
            view.horizontal_variant_records(horizontal_base)
                .into_iter()
                .map(|variant| variant.glyph_id),
        );
        assert_eq!(
            font.horizontal_variants(horizontal_base),
            expected_horizontal
        );

        let vertical_base = view.glyph_index('(').expect("parenthesis cmap glyph");
        let mut expected_vertical = vec![vertical_base];
        expected_vertical.extend(
            view.vertical_variant_records(vertical_base)
                .into_iter()
                .map(|variant| variant.glyph_id),
        );
        assert_eq!(font.vertical_variants(vertical_base), expected_vertical);
    }

    #[test]
    fn public_raw_assembly_view_remains_an_exact_compatibility_projection() {
        const GLYPH_ID: u16 = 746;

        let font = stix();
        let view = font.operation_view();
        let variants = view.math_table().variants.expect("MATH variants");
        let raw = variants
            .horizontal_constructions
            .get(ttf_parser::GlyphId(GLYPH_ID))
            .and_then(|construction| construction.assembly)
            .expect("raw horizontal assembly");
        let actual = font.horizontal_assembly_parts(GLYPH_ID);

        assert_eq!(actual.len(), usize::from(raw.parts.len()));
        for (index, actual_part) in actual.iter().enumerate() {
            let raw_part = raw
                .parts
                .get(u16::try_from(index).expect("assembly index fits u16"))
                .expect("raw assembly part");
            assert_eq!(
                *actual_part,
                (
                    raw_part.glyph_id.0,
                    raw_part.start_connector_length,
                    raw_part.end_connector_length,
                    raw_part.full_advance,
                    raw_part.part_flags.extender(),
                )
            );
        }
    }

    #[test]
    fn raw_assembly_presence_distinguishes_absence_from_malformed_data() {
        // MATH header + one vertical construction. The construction covers gid
        // 42 and declares an assembly directly after its four-byte header.
        let mut math = vec![0u8; 38];
        math[0..4].copy_from_slice(&[0, 1, 0, 0]);
        math[8..10].copy_from_slice(&10u16.to_be_bytes());
        let variants = 10usize;
        math[variants + 2..variants + 4].copy_from_slice(&12u16.to_be_bytes());
        math[variants + 6..variants + 8].copy_from_slice(&1u16.to_be_bytes());
        math[variants + 10..variants + 12].copy_from_slice(&18u16.to_be_bytes());
        let coverage = variants + 12;
        math[coverage..coverage + 2].copy_from_slice(&1u16.to_be_bytes());
        math[coverage + 2..coverage + 4].copy_from_slice(&1u16.to_be_bytes());
        math[coverage + 4..coverage + 6].copy_from_slice(&42u16.to_be_bytes());
        let construction = variants + 18;
        math[construction..construction + 2].copy_from_slice(&4u16.to_be_bytes());

        assert_eq!(
            super::raw_math_assembly_declared(&math, 42, super::MathAssemblyAxis::Vertical),
            Ok(true)
        );
        assert_eq!(
            super::raw_math_assembly_declared(&math, 43, super::MathAssemblyAxis::Vertical),
            Ok(false)
        );

        math[construction..construction + 2].copy_from_slice(&0u16.to_be_bytes());
        assert_eq!(
            super::raw_math_assembly_declared(&math, 42, super::MathAssemblyAxis::Vertical),
            Ok(false)
        );

        math[construction..construction + 2].copy_from_slice(&30u16.to_be_bytes());
        assert_eq!(
            super::raw_math_assembly_declared(&math, 42, super::MathAssemblyAxis::Vertical),
            Err(super::MathAssemblyDataError::MalformedAssemblyData)
        );
    }

    #[test]
    fn raw_coverage_format_two_maps_ranges_and_rejects_overlap() {
        let mut coverage = Vec::new();
        coverage.extend_from_slice(&2u16.to_be_bytes());
        coverage.extend_from_slice(&2u16.to_be_bytes());
        coverage.extend_from_slice(&10u16.to_be_bytes());
        coverage.extend_from_slice(&12u16.to_be_bytes());
        coverage.extend_from_slice(&0u16.to_be_bytes());
        coverage.extend_from_slice(&20u16.to_be_bytes());
        coverage.extend_from_slice(&21u16.to_be_bytes());
        coverage.extend_from_slice(&3u16.to_be_bytes());

        assert_eq!(super::coverage_index(&coverage, 0, 11), Ok(Some(1)));
        assert_eq!(super::coverage_index(&coverage, 0, 20), Ok(Some(3)));
        assert_eq!(super::coverage_index(&coverage, 0, 19), Ok(None));

        coverage[10..12].copy_from_slice(&12u16.to_be_bytes());
        assert_eq!(
            super::coverage_index(&coverage, 0, 20),
            Err(super::MathAssemblyDataError::MalformedAssemblyData)
        );
    }

    #[test]
    fn reserved_assembly_part_flags_are_rejected_before_typed_projection() {
        assert_eq!(
            super::assembly_extender_flag(ttf_parser::math::PartFlags(0x0002)),
            Err(super::MathAssemblyDataError::ReservedPartFlags)
        );
        assert_eq!(
            super::assembly_extender_flag(ttf_parser::math::PartFlags(0x0001)),
            Ok(true)
        );
        assert_eq!(
            super::assembly_extender_flag(ttf_parser::math::PartFlags(0)),
            Ok(false)
        );
    }

    #[test]
    fn font_unit_conversion_uses_the_validated_face_denominator() {
        let font = stix();
        let view: MathFontView<'_> = font.operation_view();
        assert_eq!(
            view.font_units(1),
            crate::Dim::from_font_units(1, font.units_per_em())
                .expect("validated nonzero unitsPerEm")
        );
        let axis = view.math_constants().axis_height();
        assert_eq!(
            view.math_value(axis),
            crate::Dim::from_font_units(i64::from(axis.value), font.units_per_em())
                .expect("validated nonzero unitsPerEm")
        );
    }
}
