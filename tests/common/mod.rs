use texpose::{Error, MathFont};

pub(crate) const STIX_TWO_MATH_OTF: &[u8] =
    include_bytes!("../fixtures/fonts/stix-two-math/STIXTwoMath-Regular.otf");

pub(crate) fn stix_two_math() -> Result<MathFont, Error> {
    MathFont::from_bytes(STIX_TWO_MATH_OTF)
}
