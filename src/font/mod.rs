//! OpenType math font metrics. Integer font units → [`Dim`](crate::Dim).

use core::num::NonZeroU16;

use ttf_parser::Face;

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

#[derive(Clone, Copy)]
struct ScriptAlternateEntry {
    glyph_id: u16,
    alternates: [Option<u16>; 2],
}

/// Loaded math face.
///
/// # Examples
///
/// ```no_run
/// use texpose::{Error, MathFont};
///
/// # fn font_bytes() -> &'static [u8] { unimplemented!() }
/// # fn main() -> Result<(), Error> {
/// let font = MathFont::from_bytes(font_bytes())?;
/// assert!(font.units_per_em() > 0);
/// # Ok(())
/// # }
/// ```
pub struct MathFont {
    raw: &'static [u8],
    face: Face<'static>,
    units_per_em: NonZeroU16,
    ascender_fu: i16,
    descender_fu: i16,
    script_alternates: Vec<ScriptAlternateEntry>,
}

impl MathFont {
    /// Parse OpenType bytes from a caller-provided static buffer.
    ///
    /// The current font representation borrows that buffer for its full lifetime.
    pub fn from_bytes(raw: &'static [u8]) -> Result<Self, Error> {
        let face = Face::parse(raw, 0).map_err(|_| FontError::InvalidFace)?;
        let units_per_em = NonZeroU16::new(face.units_per_em()).ok_or(FontError::InvalidFace)?;
        let ascender_fu = face.ascender();
        let descender_fu = face.descender();

        let mut script_alternates = Vec::new();
        for glyph_id in 0..face.number_of_glyphs() {
            let alternates = [
                ssty_alternate_glyph_id(&face, glyph_id, 1),
                ssty_alternate_glyph_id(&face, glyph_id, 2),
            ];

            if alternates.iter().any(Option::is_some) {
                script_alternates.push(ScriptAlternateEntry {
                    glyph_id,
                    alternates,
                });
            }
        }

        Ok(Self {
            raw,
            face,
            units_per_em,
            ascender_fu,
            descender_fu,
            script_alternates,
        })
    }

    /// The parsed OpenType face.
    ///
    /// A native consumer may need glyph outlines and bounding boxes, which this
    /// crate does not otherwise expose. Reaching the face here
    /// rather than re-parsing [`Self::bytes`] guarantees that the glyph ids in
    /// [`BoxContent::Glyph`](crate::BoxContent::Glyph) are resolved against the
    /// same face, parsed by the same version of `ttf-parser`, that produced
    /// them. The crate re-exports [`ttf_parser`] so that a
    /// consumer can name this type without pinning the version itself.
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
    pub fn face(&self) -> &Face<'static> {
        &self.face
    }

    pub(crate) fn script_alternate_glyph_id(&self, glyph_id: u16, script_level: u8) -> Option<u16> {
        let alternate_index = match script_level {
            1 => 0,
            2 => 1,
            _ => return None,
        };

        let index = self
            .script_alternates
            .binary_search_by_key(&glyph_id, |entry| entry.glyph_id)
            .ok()?;

        self.script_alternates[index].alternates[alternate_index]
    }

    /// OpenType bytes this face was parsed from.
    #[must_use]
    pub fn bytes(&self) -> &'static [u8] {
        self.raw
    }

    /// `unitsPerEm` from the `head` table.
    #[must_use]
    pub fn units_per_em(&self) -> u16 {
        self.units_per_em.get()
    }

    pub(crate) fn units_per_em_nonzero(&self) -> NonZeroU16 {
        self.units_per_em
    }

    /// `hhea` ascender in font units.
    #[must_use]
    pub fn ascender_fu(&self) -> i16 {
        self.ascender_fu
    }

    /// `hhea` descender in font units (typically negative).
    #[must_use]
    pub fn descender_fu(&self) -> i16 {
        self.descender_fu
    }

    /// Ascender in em.
    #[must_use]
    pub fn ascender(&self) -> Dim {
        Dim::from_font_units_nonzero(i64::from(self.ascender_fu), self.units_per_em)
    }

    /// Depth below baseline from `hhea` descender, in em (non-negative).
    #[must_use]
    pub fn descender(&self) -> Dim {
        let d = i64::from(self.descender_fu);
        Dim::from_font_units_nonzero(-d, self.units_per_em)
    }

    /// Metrics for `ch`, or [`FontError::MissingGlyph`].
    pub fn glyph(&self, ch: char) -> Result<GlyphMetrics, Error> {
        let face = self.face();
        let gid = face.glyph_index(ch).ok_or(FontError::MissingGlyph { ch })?;
        let advance_fu = face
            .glyph_hor_advance(gid)
            .ok_or(FontError::MissingGlyph { ch })?;
        let mut height_fu = 0i64;
        let mut depth_fu = 0i64;
        if let Some(bbox) = face.glyph_bounding_box(gid) {
            height_fu = i64::from(bbox.y_max).max(0);
            depth_fu = i64::from(-bbox.y_min).max(0);
        }
        let upem = self.units_per_em;
        Ok(GlyphMetrics {
            ch,
            glyph_id: gid.0,
            advance_fu,
            advance: Dim::from_font_units_nonzero(i64::from(advance_fu), upem),
            height: Dim::from_font_units_nonzero(height_fu, upem),
            depth: Dim::from_font_units_nonzero(depth_fu, upem),
        })
    }

    /// Metrics for OpenType glyph id `gid`, tagged with `ch` for the box payload.
    pub fn glyph_id(&self, ch: char, gid: u16) -> Result<GlyphMetrics, Error> {
        let face = self.face();
        let gid = ttf_parser::GlyphId(gid);
        let advance_fu = face
            .glyph_hor_advance(gid)
            .ok_or(FontError::MissingGlyph { ch })?;
        let mut height_fu = 0i64;
        let mut depth_fu = 0i64;
        if let Some(bbox) = face.glyph_bounding_box(gid) {
            height_fu = i64::from(bbox.y_max).max(0);
            depth_fu = i64::from(-bbox.y_min).max(0);
        }
        let upem = self.units_per_em;
        Ok(GlyphMetrics {
            ch,
            glyph_id: gid.0,
            advance_fu,
            advance: Dim::from_font_units_nonzero(i64::from(advance_fu), upem),
            height: Dim::from_font_units_nonzero(height_fu, upem),
            depth: Dim::from_font_units_nonzero(depth_fu, upem),
        })
    }

    /// MATH italic correction for `glyph_id`, or zero.
    pub fn italic_correction(&self, glyph_id: u16) -> Dim {
        let face = self.face();
        let Some(math) = face.tables().math else {
            return Dim::zero();
        };
        let Some(info) = math.glyph_info else {
            return Dim::zero();
        };
        let Some(table) = info.italic_corrections else {
            return Dim::zero();
        };
        match table.get(ttf_parser::GlyphId(glyph_id)) {
            Some(v) => Dim::from_font_units_nonzero(i64::from(v.value), self.units_per_em),
            None => Dim::zero(),
        }
    }

    /// MATH top-accent attachment (em from glyph left), if present.
    pub fn top_accent_attachment(&self, glyph_id: u16) -> Option<Dim> {
        let face = self.face();
        let math = face.tables().math?;
        let info = math.glyph_info?;
        let table = info.top_accent_attachments?;
        let v = table.get(ttf_parser::GlyphId(glyph_id))?;
        Some(Dim::from_font_units_nonzero(
            i64::from(v.value),
            self.units_per_em,
        ))
    }

    /// Horizontal glyph-assembly parts: `(gid, start_connector, end_connector, advance, extender)`.
    /// Lengths are font units.
    pub fn horizontal_assembly_parts(&self, glyph_id: u16) -> Vec<(u16, u16, u16, u16, bool)> {
        let mut out = Vec::new();
        let face = self.face();
        let Some(math) = face.tables().math else {
            return out;
        };
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

    /// Horizontal MATH variants of `glyph_id`, including the base glyph first.
    pub fn horizontal_variants(&self, glyph_id: u16) -> Vec<u16> {
        let mut out = vec![glyph_id];
        let face = self.face();
        let Some(math) = face.tables().math else {
            return out;
        };
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

    /// Vertical MATH variants of `glyph_id`, including the base glyph first.
    pub fn vertical_variants(&self, glyph_id: u16) -> Vec<u16> {
        let mut out = vec![glyph_id];
        let face = self.face();
        let Some(math) = face.tables().math else {
            return out;
        };
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

fn ssty_alternate_glyph_id(face: &Face<'_>, glyph_id: u16, script_level: u8) -> Option<u16> {
    let alternate_index = match script_level {
        1 => 0,
        2 => 1,
        _ => return None,
    };

    let gsub = face.tables().gsub?;
    let feature = gsub.features.find(ttf_parser::Tag::from_bytes(b"ssty"))?;

    for lookup_index in feature.lookup_indices {
        let Some(lookup) = gsub.lookups.get(lookup_index) else {
            continue;
        };

        for subtable in lookup
            .subtables
            .into_iter::<ttf_parser::gsub::SubstitutionSubtable<'_>>()
        {
            let ttf_parser::gsub::SubstitutionSubtable::Alternate(alternate) = subtable else {
                continue;
            };

            let Some(coverage_index) = alternate.coverage.get(ttf_parser::GlyphId(glyph_id)) else {
                continue;
            };

            let Some(set) = alternate.alternate_sets.get(coverage_index) else {
                continue;
            };

            if let Some(selected) = set.alternates.get(alternate_index) {
                return Some(selected.0);
            }
        }
    }

    None
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
