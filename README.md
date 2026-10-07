# TeXpose

TeXpose is a portable Rust engine for parsing and laying out TeX/LaTeX mathematical notation.

TeXpose is an independent hard fork of LaTeX-Rust 2.0.1. It is not synchronized with the upstream project.

## Status

TeXpose is in early development. The current core contains math parsing, OpenType MATH font metrics, exact TeX-style internal geometry, and a flat backend-neutral public display list. Rendering, rasterization, windows, surfaces, GPU/device lifetime, pixel snapping, and application UI are consumer responsibilities. The public API and repository structure remain intentionally unstable while the hard-fork cutover is completed.

The architecture is backend-neutral: TeXpose produces typed notation plus `MathLayout`, a flat root-em-normalized display list that applications consume through their native graphics stack. Planned domains may extend beyond mathematics to chemistry, SI units, and other scientific notation.

## Provenance

The initial codebase derives from LaTeX-Rust by Jeffrey S Carr. Source provenance and licensing attribution are preserved in this repository.

## License

TeXpose is distributed under MIT OR Apache-2.0.

Verification fonts live under `tests/fixtures/fonts/` and are not embedded in the production crate. Each fixture records its upstream source, SHA-256, and font license in its own directory.

## Font input contract

The first stable core accepts caller-provided non-variable OpenType OTF/TTF faces and TTC/OTC collections. The ownership-oriented constructor is `MathFont::from_shared_bytes(Arc<[u8]>, face_index)`, and clones of `MathFont` share that immutable allocation. The `from_bytes` / `from_bytes_at_index` compatibility constructors copy borrowed slices into the same shared representation. Each layout operation parses one temporary OpenType face from the retained bytes and reuses it throughout that operation. Functional OpenType variable fonts are rejected with `FontError::VariableFontUnsupported`. Construction also validates the physical `MATH` table and mandatory `MathConstants`, preserving distinct typed failures for absence and malformed data. MATH `MathValueRecord` layout uses design-unit values and deliberately ignores PPEM-dependent Device corrections; see `docs/MATH_COVERAGE.md` and `docs/LAYOUT.md`.

## Fuzzing

Roadmap Phase H fuzz targets live in `fuzz/`. The maintained input domains,
independent oracles, corpus policy, resource bounds, and reproduction commands
are documented in `docs/FUZZING.md`. The fuzz package is separate from the
production crate, so its oracle dependencies do not enter TeXpose's normal
dependency graph.
