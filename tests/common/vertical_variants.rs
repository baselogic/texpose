use texpose::{Dim, MathFont};

/// Select a vertical OpenType MATH variant from raw `advanceMeasurement` data.
///
/// This reads the parser's MATH view directly so layout tests do not derive
/// their expected variant from TeXpose's internal typed construction records.
pub(crate) fn select_by_advance(font: &MathFont, ch: char, target: &Dim, scale: &Dim) -> u16 {
    let base = font.glyph(ch).expect("base vertical variant glyph");
    let mut selected_id = base.glyph_id;
    let mut selected_measure = base
        .height
        .checked_add(&base.depth)
        .expect("base variant extent")
        .checked_mul(scale)
        .expect("scaled base variant extent");

    let face = font.face();
    let Some(math) = face.tables().math else {
        return selected_id;
    };
    let Some(variants) = math.variants else {
        return selected_id;
    };
    let Some(construction) = variants
        .vertical_constructions
        .get(ttf_parser::GlyphId(base.glyph_id))
    else {
        return selected_id;
    };

    for index in 0..construction.variants.len() {
        let variant = construction
            .variants
            .get(index)
            .expect("variant index below MATH variant count");
        font.glyph_id(ch, variant.variant_glyph.0)
            .expect("MATH variant glyph metrics");
        let measure =
            Dim::from_font_units(i64::from(variant.advance_measurement), font.units_per_em())
                .expect("validated unitsPerEm")
                .checked_mul(scale)
                .expect("scaled MATH variant advance");

        let candidate_meets = measure >= *target;
        let selected_meets = selected_measure >= *target;
        let should_select = if candidate_meets {
            !selected_meets || measure < selected_measure
        } else {
            !selected_meets && measure > selected_measure
        };
        if should_select {
            selected_id = variant.variant_glyph.0;
            selected_measure = measure;
        }
    }

    selected_id
}
