use std::sync::{Arc, OnceLock};

use crate::test_support::{FontError, MathFont};

pub(crate) const STIX_TWO_MATH_OTF: &[u8] =
    include_bytes!("../../../tests/fixtures/fonts/stix-two-math/STIXTwoMath-Regular.otf");
pub(crate) const STIX_TWO_MATH_FACE_INDEX: u32 = 0;

fn stix_two_math_shared_bytes() -> Arc<[u8]> {
    static BYTES: OnceLock<Arc<[u8]>> = OnceLock::new();
    Arc::clone(BYTES.get_or_init(|| Arc::from(STIX_TWO_MATH_OTF)))
}

pub(crate) fn stix_two_math() -> Result<MathFont, FontError> {
    MathFont::from_shared_bytes(stix_two_math_shared_bytes(), STIX_TWO_MATH_FACE_INDEX)
}
