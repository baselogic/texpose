use texpose::{FontError, MathFont};

pub(crate) const STIX_TWO_MATH_OTF: &[u8] =
    include_bytes!("../fixtures/fonts/stix-two-math/STIXTwoMath-Regular.otf");
pub(crate) const STIX_TWO_MATH_FACE_INDEX: u32 = 0;

pub(crate) fn stix_two_math() -> Result<MathFont, FontError> {
    MathFont::from_bytes_at_index(STIX_TWO_MATH_OTF, STIX_TWO_MATH_FACE_INDEX)
}
