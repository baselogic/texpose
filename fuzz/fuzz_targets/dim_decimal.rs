#![no_main]

use libfuzzer_sys::fuzz_target;
use texpose::Dim;
use texpose_fuzz::{assert_matches, reference_decimal};

const MAX_INPUT_BYTES: usize = 512;

fuzz_target!(|data: &[u8]| {
    if data.len() > MAX_INPUT_BYTES {
        return;
    }
    let Ok(text) = core::str::from_utf8(data) else {
        return;
    };

    if let Ok(value) = Dim::parse(text) {
        let expected = reference_decimal(text)
            .expect("every decimal accepted by Dim must have reference semantics");
        assert_matches(&value, &expected);
    }
});
