#![no_main]

use libfuzzer_sys::fuzz_target;
use texpose::{tokenize_spanned, ParseErrorKind};
use texpose_fuzz::{assert_source_span, assert_token_spans, H2_MAX_SOURCE_BYTES};

fuzz_target!(|data: &[u8]| {
    if data.len() > H2_MAX_SOURCE_BYTES {
        return;
    }
    let Ok(source) = std::str::from_utf8(data) else {
        return;
    };

    let first = tokenize_spanned(source);
    let second = tokenize_spanned(source);
    assert_eq!(first, second, "tokenization must be deterministic");

    match first {
        Ok(tokens) => assert_token_spans(source, &tokens),
        Err(error) => {
            assert_source_span(source, error.span());
            assert_eq!(error.kind(), ParseErrorKind::TrailingBackslash);
            assert_eq!(error.span().end, source.len());
            assert_eq!(error.span().start + 1, error.span().end);
        }
    }
});
