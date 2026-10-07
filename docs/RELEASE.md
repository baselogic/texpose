# Release gates

TeXpose currently has `publish = false`; this document defines candidate verification and evidence, not publication authority.

## Normal dependency inventory

The production manifest has one direct normal dependency:

| Crate | Requirement | Default features | Enabled features | Purpose |
| --- | --- | --- | --- | --- |
| `ttf-parser` | `0.25` | disabled | `std`, `opentype-layout` | OpenType face parsing, metrics, MATH and layout tables |

The root crate intentionally has no normal dependency on the fuzz package or oracle tooling. Because the library does not commit a root `Cargo.lock`, an exact resolved patch version is build evidence rather than a permanent source invariant. Every candidate therefore retains the `cargo tree -e normal`, `cargo tree -e dev`, and `cargo tree -e features` outputs from the dependency gate. Any new normal dependency or transitive normal node requires explicit review and an update to this inventory/rationale.

## Candidate evidence

Before treating a revision as a release candidate, require green evidence for:

```text
Core CI: stable Windows/Linux/macOS
MSRV: Rust 1.76 Windows/Linux/macOS
Canonical oracle: stix/libertinus/fira on the pinned reference runner
Stress oracle: stix/libertinus/fira on the pinned reference runner
Dependency trees: normal/dev/features
```

The oracle JSON artifacts identify the font hash, face index, corpus census, positioned mismatches, and reference-environment fingerprint used by that run. The dependency artifact identifies the resolved graph. These artifacts describe the candidate actually verified; they must not be reused after a semantic source, manifest, toolchain, profile, or oracle-baseline change.

Publishing, tagging, signing, pushing, or creating a public release is outside this gate and requires separate explicit authority.
