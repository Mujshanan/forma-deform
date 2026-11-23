# CQ2 formatting-only review

**Baseline:** `975045cbcd43bd9b1e2195041d06a057d514721d`  
**Review branch:** `quality/cq2-formatting-patch`  
**Toolchain:** stable Rust `1.100.0`; rustfmt `1.11.0`  
**Check command:** `cargo fmt --all -- --check`

## Initial check result

The first isolated check completed with a formatting failure. It identified formatting diffs in **47 Rust source and test files** across the six workspace members. The failure is not a compilation/test failure: rustfmt found files whose current text does not match the selected stable formatter output.

The check did not write any changes to the repository. No application source files were modified by the check.

The patch-generation workflow is configured to run `cargo fmt --all` only in its ephemeral GitHub Actions checkout, produce a unified binary-safe patch and file/stat manifests, and upload those as an artifact. It does not commit or push formatted source. The resulting patch must be reviewed before it is applied as a formatting-only change.

## Scope

The 47 files cover:
- Smart Controller core implementation and tests.
- Form Deform core.
- AniCraft plugin SDK.
- Bone Query plugin.
- Form Deform plugin.
- Smart Controller plugin and integration tests.

The three decomposition targets are among the files needing formatting:
- `crates/smart-controller-core/src/canonical.rs`
- `crates/smart-controller-core/src/evaluator.rs`
- `crates/smart-controller-core/src/composition.rs`

That overlap is why formatting must be landed and reviewed separately before structural edits. It prevents formatting noise from obscuring the later semantic diff.

## Review policy

1. Apply only the output of stable `cargo fmt --all`; do not combine import cleanup, naming, comments, helper extraction, lint fixes, or logic changes.
2. Review the patch for generated/non-Rust files and unexpected scope before accepting it.
3. Run `cargo fmt --all -- --check` again after applying the patch; it must pass.
4. Run the existing Rust workspace tests and all three release WASM builds after applying the formatting patch. Formatting should be semantics-preserving, but verification is still required.
5. Keep the formatting patch in its own commit/PR. Do not merge it together with the canonical-core decomposition work.

## Status

- [x] Stable rustfmt check executed in GitHub Actions.
- [x] Formatting differences identified.
- [ ] Generated patch artifact retrieved and inspected.
- [ ] Formatting-only patch approved/applied.
- [ ] Post-format check, tests, and WASM builds verified.

No source formatting changes have been committed by this review.
