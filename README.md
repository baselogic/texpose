# TeXpose

TeXpose is a Rust library for TeX/LaTeX mathematical parsing and backend-neutral layout. It is an independent fork of LaTeX-Rust 2.0.1, not synchronized with upstream.

The public output is `MathLayout`: a flat display list in root-em-normalized coordinates, with the exact math font face retained for glyph resolution. Applications handle rasterization, drawing, windows, GPU/device lifetime and pixel snapping. TeXpose produces no pixels.

## Supported scope

- Rust 1.76+, Edition 2021. The crate is not published (`publish = false`).
- Use `parse` or `parse_with_options`; parse errors have UTF-8 byte spans.
- Use `MathFont` with caller-provided OTF/TTF or indexed TTC/OTC bytes. Each formula uses one static math face; variable fonts are rejected.
- Literal text uses the same face without general text shaping, bidi, kerning or fallback.
- Style-relative `em`/`mu` and physical TeX `pt`/`bp` are distinct. MATH Device/VariationIndex corrections are not applied.
- Invalid input produces typed errors; specified recoverable font failures produce layout diagnostics.

See [API](docs/API.md), [syntax](docs/SYNTAX.md), [layout](docs/LAYOUT.md), [compatibility](docs/COMPATIBILITY.md) and [verification](docs/VERIFICATION.md).

## Local checks

```sh
cargo fmt --check
cargo test
cargo clippy --all-targets -- -D warnings
```

The oracle environment, stress gates, MSRV matrix, fuzz targets, and historical candidate evidence are defined in [verification](docs/VERIFICATION.md). A passing historical candidate does not certify a later revision.

## License

MIT OR Apache-2.0. The original project is LaTeX-Rust by Jeffrey S Carr. Font fixtures retain their own licensing and provenance.
