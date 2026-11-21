# Shared Primitives — CCC-01.3

- **Milestone:** CCC-01.3 — Shared Primitives
- **Status:** Implemented; CI passed; pending review
- **Baseline:** `6b53aac51ddc15249c5100745726027fd9c75907`
- **Compatibility contract:** `docs/CODE_COMPATIBILITY.md` (CCC-COMP-001)

## 1. Objective

Consolidate duplicated foundational Rust value types without changing the behavior of deformation evaluation, controller evaluation, plugin adapters, or persisted binding data.

## 2. Shared crate

The workspace now has a host-independent `forma-geometry` crate. It owns the canonical Rust `Vec2` value type.

The type remains a two-component `f64` structure with public `x` and `y` fields. It intentionally does not encode whether a value is a point, displacement, or direction, nor does it assign a coordinate space or unit. Those semantics remain the responsibility of the API carrying the value.

Serde support is opt-in through the crate's `serde` feature. Its representation remains the existing object shape with `x` and `y`, and unknown fields remain denied when deserializing.

## 3. Compatibility-preserving re-exports

- `form_deform_core::Vec2` remains available as a public re-export.
- `smart_controller_core::legacy::Vec2` remains available as a public re-export.
- The existing root re-export from `smart_controller_core` continues to expose `Vec2`.
- Existing struct-literal construction remains valid.
- Controller serialization continues to use the same `x`/`y` representation.
- No plugin-facing API or AniCraft ABI is changed.

The controller crate depends directly on `forma-geometry`; it does not depend on `form-deform-core`. The shared primitive therefore does not create a dependency from controller logic into deformation logic.

## 4. Identity decision

No shared ID newtype is introduced in this milestone. Existing binding, controller, symbol-instance, element, pin, and bone identities are distinct semantic domains represented by strings in existing public and serialized contracts. Replacing those fields with wrappers would create unnecessary source and serialization migration risk before a canonical identity model is designed.

The distinction between these identity domains remains explicit. Array positions are not promoted to stable identities.

## 5. Coordinate and geometry boundary

`Vec2` is a value primitive, not a coordinate-space model or geometry ownership decision. The current stage-space meaning of pin deformation remains unchanged. The new crate does not add transforms, coordinate conversion, geometry topology, point/vector semantic wrappers, or FDM v2 document types.

## 6. Verification required

- Compile and test the full Rust workspace.
- Verify existing controller binding serialization fixtures and round trips.
- Build all three plugin targets for `wasm32-unknown-unknown`.
- Confirm the old public `Vec2` import paths compile.
- Confirm the dependency graph remains acyclic and no host SDK dependency enters `forma-geometry`.

## 7. Verification results

GitHub Actions pull-request run [44897850860](https://github.com/Mujshanan/forma-deform/actions/runs/44897850860) completed successfully on the implementation commit. It passed TypeScript typecheck/tests/build, `cargo test --workspace`, and release WASM builds for Bone Query, Form Deform, and Smart Controller.

The workflow does not run `cargo fmt --all -- --check`; formatting was not independently verified by CI.

## 8. Deferred

- Typed identity wrappers and identity conversion rules.
- Point-versus-displacement semantic types.
- Coordinate-space types and explicit transform APIs.
- Geometry ownership and topology.
- Shared TypeScript/Rust geometry representation.
