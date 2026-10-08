use std::sync::Arc;

use crate::test_support::{layout, parse, MathFont, MathStyle};

const STIX: &[u8] =
    include_bytes!("../../../tests/fixtures/fonts/stix-two-math/STIXTwoMath-Regular.otf");

#[test]
fn shared_font_storage_outlives_the_callers_handle_and_clones_without_copying() {
    let caller_bytes: Arc<[u8]> = Arc::from(STIX.to_vec());
    let font = MathFont::from_shared_bytes(Arc::clone(&caller_bytes), 0).expect("shared STIX font");
    let clone = font.clone();

    let font_bytes = font.shared_bytes();
    let clone_bytes = clone.shared_bytes();
    assert!(Arc::ptr_eq(&caller_bytes, &font_bytes));
    assert!(Arc::ptr_eq(&font_bytes, &clone_bytes));
    assert_eq!(font.bytes().as_ptr(), caller_bytes.as_ptr());
    assert_eq!(font.face_index(), 0);

    drop(caller_bytes);
    drop(font);

    let ast = parse(r"\frac{x_1}{y^2}").expect("math expression");
    let laid = layout(&ast, &clone, MathStyle::Display).expect("layout from cloned font");
    assert!(!laid.width.is_zero());
    assert!(clone.face().tables().math.is_some());
}

#[test]
fn slice_compatibility_constructor_copies_nonstatic_input_into_owned_storage() {
    let owned = STIX.to_vec();
    let font = MathFont::from_bytes(&owned).expect("copied STIX font");
    assert_eq!(font.bytes(), owned.as_slice());

    drop(owned);

    let glyph = font
        .glyph('x')
        .expect("glyph after source buffer is dropped");
    assert!(glyph.advance_fu > 0);
}
