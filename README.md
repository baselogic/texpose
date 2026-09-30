# TeXpose

TeXpose is a portable Rust engine for parsing and laying out TeX/LaTeX mathematical notation.

TeXpose is an independent hard fork of LaTeX-Rust 2.0.1. It is not synchronized with the upstream project.

## Status

TeXpose is in early development. The inherited codebase currently contains math parsing, OpenType MATH font metrics, TeX-style box layout, and legacy SVG, PNG, and egui rendering backends. The public API and repository structure are intentionally unstable while the hard-fork cutover is completed.

The target architecture is backend-neutral: TeXpose should produce typed notation and layout data that applications can render through their native graphics stack. Planned domains may extend beyond mathematics to chemistry, SI units, and other scientific notation.

## Provenance

The initial codebase derives from LaTeX-Rust by Jeffrey S Carr. Source provenance and licensing attribution are preserved in this repository.

## License

TeXpose is distributed under MIT OR Apache-2.0.

The embedded STIX Two Math font remains licensed under the SIL Open Font License 1.1. See `fonts/stix-two-math/OFL.txt`.
