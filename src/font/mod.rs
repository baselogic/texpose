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
        Dim::from_font_units_nonzero(i64::from(font.face.ascender()), font.units_per_em_nonzero())
    }

    /// Depth below baseline from `hhea` descender, in em (non-negative).
    #[must_use]
    pub fn descender(&self) -> Dim {
        let font = self.operation_view();
        let d = i64::from(font.face.descender());
        Dim::from_font_units_nonzero(-d, font.units_per_em_nonzero())
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
    /// Lengths are font units.
    pub fn horizontal_assembly_parts(&self, glyph_id: u16) -> Vec<(u16, u16, u16, u16, bool)> {
        self.operation_view().horizontal_assembly_parts(glyph_id)
    }

    /// Horizontal MATH variants of `glyph_id`, including the base glyph first.
    pub fn horizontal_variants(&self, glyph_id: u16) -> Vec<u16> {
        self.operation_view().horizontal_variants(glyph_id)
    }

    /// Vertical MATH variants of `glyph_id`, including the base glyph first.
    pub fn vertical_variants(&self, glyph_id: u16) -> Vec<u16> {
        self.operation_view().vertical_variants(glyph_id)
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
        let upem = self.units_per_em_nonzero();
        Ok(GlyphMetrics {
            ch,
            glyph_id: gid.0,
            advance_fu,
            advance: Dim::from_font_units_nonzero(i64::from(advance_fu), upem),
            height: Dim::from_font_units_nonzero(height_fu, upem),
            depth: Dim::from_font_units_nonzero(depth_fu, upem),
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
            Some(v) => Dim::from_font_units_nonzero(
                i64::from(math_value_design_units(v)),
                self.units_per_em_nonzero(),
            ),
            None => Dim::zero(),
        }
    }

    pub(crate) fn top_accent_attachment(&self, glyph_id: u16) -> Option<Dim> {
        let math = self.math_table();
        let info = math.glyph_info?;
        let table = info.top_accent_attachments?;
        let v = table.get(ttf_parser::GlyphId(glyph_id))?;
        Some(Dim::from_font_units_nonzero(
            i64::from(math_value_design_units(v)),
            self.units_per_em_nonzero(),
        ))
    }

    pub(crate) fn horizontal_assembly_parts(
        &self,
        glyph_id: u16,
    ) -> Vec<(u16, u16, u16, u16, bool)> {
        let mut out = Vec::new();
        let math = self.math_table();
        let Some(variants) = math.variants else {
            return out;
        };
        let Some(cons) = variants
            .horizontal_constructions
            .get(ttf_parser::GlyphId(glyph_id))
        else {
            return out;
        };
        let Some(assembly) = cons.assembly else {
            return out;
        };
        for i in 0..assembly.parts.len() {
            if let Some(p) = assembly.parts.get(i) {
                out.push((
                    p.glyph_id.0,
                    p.start_connector_length,
                    p.end_connector_length,
                    p.full_advance,
                    p.part_flags.extender(),
                ));
            }
        }
        out
    }

    pub(crate) fn horizontal_variants(&self, glyph_id: u16) -> Vec<u16> {
        let mut out = vec![glyph_id];
        let math = self.math_table();
        let Some(variants) = math.variants else {
            return out;
        };
        let Some(cons) = variants
            .horizontal_constructions
            .get(ttf_parser::GlyphId(glyph_id))
        else {
            return out;
        };
        for i in 0..cons.variants.len() {
            if let Some(v) = cons.variants.get(i) {
                out.push(v.variant_glyph.0);
            }
        }
        out
    }

    pub(crate) fn vertical_variants(&self, glyph_id: u16) -> Vec<u16> {
        let mut out = vec![glyph_id];
        let math = self.math_table();
        let Some(variants) = math.variants else {
            return out;
        };
        let Some(cons) = variants
            .vertical_constructions
            .get(ttf_parser::GlyphId(glyph_id))
        else {
            return out;
        };
        for i in 0..cons.variants.len() {
            if let Some(v) = cons.variants.get(i) {
                out.push(v.variant_glyph.0);
            }
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

/// Return the design-unit component of an OpenType `MathValueRecord`.
///
/// TeXpose deliberately ignores PPEM-dependent Device-table corrections in
/// the stable core so mathematical layout is independent of display DPI and
/// pixel-grid hinting.
pub(crate) fn math_value_design_units(value: ttf_parser::math::MathValue<'_>) -> i16 {
    value.value
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
