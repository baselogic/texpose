#![no_main]

use libfuzzer_sys::fuzz_target;
use texpose::parse_with_options;
use texpose_fuzz::{assert_parse_error, h2_parse_options, H2_MAX_SOURCE_BYTES};

fuzz_target!(|data: &[u8]| {
    if data.len() > H2_MAX_SOURCE_BYTES {
        return;
    }
    let Ok(source) = std::str::from_utf8(data) else {
        return;
    };

    let options = h2_parse_options();
    let first = parse_with_options(source, &options).map(|(node, _)| node);
    let second = parse_with_options(source, &options).map(|(node, _)| node);
    assert_eq!(first, second, "parse result must be deterministic");

    if let Err(error) = &first {
        assert_parse_error(source, error, &options);
    }
});
