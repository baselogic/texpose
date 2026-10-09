# Math font verification fixtures

These fonts are committed **only for tests, benchmarks, and the LuaLaTeX oracle**.
They are not a default font for TeXpose and no font is embedded by a production
`lib` build. Applications provide the OpenType MATH font bytes (including the
TTC/OTC face index) used for layout and then draw using that same face.

The directory is intentionally at repository root so production source, test
helpers, benchmarks, and oracle tools use one canonical filesystem location.
Every fixture retains its existing exact bytes, SHA-256, original license, and
`SOURCE.md` provenance in its own subdirectory. The `fonts/**` package include
allows source archives to compile their tests; it does **not** enable any font
feature or embed the fixtures into the compiled library. TeXpose is currently
`publish = false`; including fixtures in a future distributable crate archive
is a separate release-size/licensing decision.

Current fixtures: STIX Two Math v2.13, Libertinus Math v7.051, Fira Math
v0.3.4, and DejaVu Math TeX Gyre 2.37. Windows Cambria Math is a
system-owned, diagnostic-only oracle source; it is not committed here.

Keep the contractual oracle profiles (`stix`, `libertinus`, `fira`) and their
pinned SHA-256, corpus censuses, and approved deviations unchanged when only
moving files. A new font is exploratory until its capabilities and complete
reference evidence have been reviewed. A MATH table alone does not guarantee
coverage of every test equation.
