# TeXpose

TeXpose is a portable Rust engine for parsing and laying out TeX/LaTeX mathematical notation.

TeXpose is an independent hard fork of LaTeX-Rust 2.0.1. It is not synchronized with the upstream project.

## Status

TeXpose is in early development. The current core contains math parsing, OpenType MATH font metrics, and TeX-style box layout. Rendering, rasterization, windows, surfaces, GPU/device lifetime, pixel snapping, and application UI are consumer responsibilities. The public API and repository structure remain intentionally unstable while the hard-fork cutover is completed.

The target architecture is backend-neutral: TeXpose produces typed notation and layout data that applications can consume through their native graphics stack. Planned domains may extend beyond mathematics to chemistry, SI units, and other scientific notation.

## Provenance

The initial codebase derives from LaTeX-Rust by Jeffrey S Carr. Source provenance and licensing attribution are preserved in this repository.

## License

TeXpose is distributed under MIT OR Apache-2.0.

Verification fonts live under `tests/fixtures/fonts/` and are not embedded in the production crate. Each fixture records its upstream source, SHA-256, and font license in its own directory.
