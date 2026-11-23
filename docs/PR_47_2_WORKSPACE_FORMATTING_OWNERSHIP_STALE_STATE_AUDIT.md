# PR #47.2 — Workspace Formatting, Integration Ownership, and Stale-State Audit

**Status:** Audit completed; workspace-wide format gate exposes pre-existing formatting debt; integration ownership reviewed; Path2D conditional-write race remains open.
**Base:** PR #47.1 branch `chore/pr-47-1-clippy-audit`.
**Scope:** Formatting coverage and review of binding persistence ownership/freshness contracts. No deformation algorithm or host integration implementation is introduced.

## 1. Workspace formatting audit

The root Cargo workspace declares six members:
- `smart-controller-core`
- `form-deform-core`
- `anicraft-plugin-sdk`
- `bone-query`
- `form-deform`
- `smart-controller`

The tracked tree contains 74 Rust source, test, and benchmark files. The previous CI command formatted only `single_handle_displacement.rs` and `evaluation_engine.rs`, leaving the rest of the workspace unchecked.

The CI gate is changed to:

```sh
cargo fmt --all -- --check
```

This asks Cargo/rustfmt to check every workspace member without rewriting files. The first fresh run [#45167900555](https://github.com/Mujshanan/forma-deform/actions/runs/45167900555) failed at this gate and identified 71 of the 74 tracked Rust files as not rustfmt-clean. PR #47.2 contains no Rust source changes, so these failures are inherited from its PR #61 base; they are not regressions introduced by this audit. Three files passed. The job stopped at formatting, so downstream CI steps did not run. Do not mass-format all 71 files as an incidental side effect of an integration review; treat a workspace formatting sweep as a separately reviewed, formatting-only change.

## 2. Integration ownership review

The current ownership boundary is coherent and should be preserved:

- FDM owns binding schemas, canonicalization, structural validation, freshness checks, and validated capability types.
- The host owns the authoritative document, payload placement, transactions, dirty state, save/load orchestration, and undo/redo.
- The host adapter is the only persistence bridge; FDM does not mutate host document state.
- Path2D feature bindings and generic point-array cage bindings use distinct versioned payload envelopes and adapter slots. They share topology/certification primitives but are not forced into one document schema.
- No PR #40 `FdmDocument.cage_bindings` field is copied into this integration line. Unification of storage remains an explicit, versioned host-schema decision.
- Smart Controller evaluation and pin-deformation evaluation remain independent under CCC-01.5. This audit does not introduce composition, scheduling, caching, apply, or bake behavior.

## 3. Stale-state review

### 3.1 Generic point-array cage binding

`CageBindingHostAdapter::write_cage_binding_payload` receives both the expected stored binding revision and the expected source snapshot stamp. `persist_cage_binding` validates against a freshly certified cage and passes those expectations to the host write. The adapter contract explicitly requires atomic conditional replacement. This is the stronger compare-and-swap path.

### 3.2 Path2D feature binding — unresolved stale-write race

`BindingHostAdapter::write_binding_payload` currently receives only the target and serialized payload. `persist_binding` obtains a current context, validates the binding against that snapshot, serializes it, and then calls the write method. The write contract does not carry the observed source stamp or expected binding revision, and it exposes no conditional-write result.

Consequently, a source edit or competing binding write between validation and the host commit cannot be rejected by the FDM adapter contract itself. A host implementation might add out-of-band safeguards, but the current trait does not require or express them. This is a time-of-check/time-of-use gap, not evidence that the existing in-memory test adapter races.

**Review finding — must resolve before PR #47 is considered fully integration-safe:** strengthen the Path2D host write contract to atomically compare the expected source snapshot stamp and expected binding revision (including an explicit absent-payload expectation), then add deterministic conflict tests for source changes and competing writes. Keep the host responsible for the actual atomic document transaction. Do not silently rebind stale data.

### 3.3 Existing freshness protections retained

- Source target identity/generation, geometry revision, topology revision, coordinate space, payload schema, and fingerprint are validated.
- Cage topology identity/revision and freshly certified embedding are checked before binding use.
- Generic point binding validates stable point-ID coverage and reconstruction against current source points.
- Loading rejects stale payloads rather than silently rebinding them.
- Evaluation is derived and non-mutating; authored source geometry remains authoritative.

## 4. Review disposition

- Workspace-wide formatting: failed on CI run [#45167900555](https://github.com/Mujshanan/forma-deform/actions/runs/45167900555); 71/74 Rust files need formatting, and 3/74 passed. This is baseline debt inherited from PR #61 because PR #47.2 changes no Rust source.
- Ownership: accepted as a boundary; preserve separate payload families and host-owned transactions.
- Stale-state: generic point-array CAS boundary is explicit; Path2D conditional-write semantics remain an unresolved review item.
- No merge, no PR status transition, and no changes to PR #37 or PR #40.
