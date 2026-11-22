# M1 — Host Conformance and Acceptance Test Matrix

**Status:** acceptance specification; host-side tests are blocked pending separately authorized AniCraft implementation.
**Related contracts:** `M1_TARGET_GEOMETRY_CONTRACT.md` and `M1_EVALUATION_LIFECYCLE_CONTRACT.md`.

This matrix is the release gate for the FDM v2 host boundary. A passing Forma Deform unit-test suite does not prove AniCraft host conformance. Every host test must run against the actual document model, command transaction and save/load implementation.

## 1. FDM-side automated tests

| ID | Test | Expected result | Current evidence |
|---|---|---|---|
| FDM-M1-001 | Opaque IDs reject empty values and preserve exact strings | Invalid identifiers fail; IDs are never numerically reinterpreted | Implemented in PR #20 |
| FDM-M1-002 | Target generation participates in identity | Different generations never compare as the same target | Implemented in PR #20 |
| FDM-M1-003 | Snapshot rejects zero revisions/generation | Construction fails closed | Implemented in PR #20 |
| FDM-M1-004 | Path2D rejects malformed command sequences/non-finite coordinates | Invalid payload cannot be fingerprinted | Implemented in PR #20 |
| FDM-M1-005 | Canonical fingerprint is deterministic | Golden SHA-256 vector remains stable for algorithm v1 | Implemented in PR #20 |
| FDM-M1-006 | Coordinate interpretation affects fingerprint | Unit, scale, axes, origin or coordinate-space ID change the digest | Implemented in PR #20 |
| FDM-M1-007 | Snapshot verifies supplied fingerprint | Payload/fingerprint mismatch is rejected | Implemented in PR #20 |
| FDM-M1-008 | Freshness detects target mismatch, revision change/regression and fingerprint mismatch | Each condition has a distinct outcome | Implemented in PR #20 |
| FDM-M1-009 | Evaluation request validates source snapshots, model/time stamps and external dependencies | Missing, duplicate or invalid dependency prevents request construction | Implemented and CI-verified in PR #30, run #690 |
| FDM-M1-010 | Evaluated result binds to the exact request | Target, every source stamp, model, time, dependencies and graph stamp must match | Implemented and CI-verified in PR #30, run #690 |
| FDM-M1-011 | Evaluated result preserves source coordinate semantics | Output coordinate descriptor must exactly match the source descriptor in v1 | Implemented and CI-verified in PR #30, run #690 |
| FDM-M1-012 | Composition graph identity is explicit and versioned | Result records the exact graph stamp; graph structure/cycle validation remains a separate core task | Graph-stamp binding implemented and CI-verified in PR #30, run #690 |
| FDM-M1-013 | Persistence round-trip and named migrations | Authored IDs/settings survive; unknown versions fail closed | Not implemented; contract specified |
| FDM-M1-014 | Result fingerprint is deterministic for canonical output payload | Equivalent payload/coordinate values match; semantic payload changes differ. Result schema/operation are separately versioned and validated, not included in this Path2D digest | Implemented and CI-verified in PR #30, run #690 |

## 2. AniCraft host conformance tests

| ID | Host capability | Acceptance test |
|---|---|---|
| HOST-M1-001 | Persistent document identity | Save/reopen and rename preserve document ID; duplicate/Save As create a new ID |
| HOST-M1-002 | Target incarnation | Delete/recreate with reused numeric ID produces a new generation; old references never resolve |
| HOST-M1-003 | Import remapping | Imported targets receive destination-owned identities and no source-document references leak |
| HOST-M1-004 | Revision ownership | Every source-path mutation path advances geometry/topology epochs according to the M1 rules |
| HOST-M1-005 | Undo/redo revisions | Undo and redo advance epochs; revisions never move backward |
| HOST-M1-006 | Canonical source stage | Unwarped targets snapshot `Shape.path`; native Asset Warp targets snapshot `Warp.rest` |
| HOST-M1-007 | Atomic snapshot capture | Identity, payload, coordinate descriptor, revisions and fingerprint represent one document state |
| HOST-M1-008 | Snapshot immutability | Later edits cannot mutate a previously returned snapshot |
| HOST-M1-009 | Coordinate metadata | Unit, scale, axes, origin and local-rest semantics are verified against actual host transforms |
| HOST-M1-010 | Transform/hierarchy dependencies | Transform and hierarchy edits advance their own revisions without changing source-path revisions |
| HOST-M1-011 | Time sampling | Explicit frame/subframe sampling is repeatable and does not mutate the global playhead |
| HOST-M1-012 | Dependency invalidation | Any changed dependency invalidates cached results that consume it |
| HOST-M1-013 | Document persistence | Authored FDM extension survives save/reopen and supported document migrations |
| HOST-M1-014 | Duplicate/import persistence | Internal FDM IDs remain consistent while host references are remapped correctly |
| HOST-M1-015 | Atomic apply | Stale or invalid result is rejected before the first mutation |
| HOST-M1-016 | Multi-target/range transaction | Any preflight or commit failure leaves the entire document unchanged |
| HOST-M1-017 | Undo/redo | Apply and bake each form one coherent undo step and redo restores all output |
| HOST-M1-018 | Non-destructive bake | Provenance is retained and source restoration exactly recovers pre-bake authored state |
| HOST-M1-019 | Resource limits | Oversized payloads, excessive targets and excessive frame ranges fail before mutation |
| HOST-M1-020 | Failure diagnostics | Missing, stale, unsupported and integrity-failure cases return stable actionable diagnostics |

## 3. Evidence rules

- A source-code inspection may establish that a command or field exists; it does not prove runtime behavior.
- A Rust/Bun/WASM CI pass proves only the tests and builds that actually ran.
- Host persistence requires real save/load tests against the host document format.
- Undo, redo and rollback require tests against AniCraft's actual transaction/history implementation.
- Timeline sampling requires repeatability tests at explicit frames, including nested/parent-dependent targets.
- GUI smoke tests supplement automated host tests; they do not replace them.
- Every test report records the AniCraft commit, Forma Deform commit, host build, test IDs, results and skipped cases.

## 4. Current gate summary

- PR #20 supplies the first FDM-side source identity/snapshot implementation and its automated tests.
- The remaining FDM lifecycle result/persistence tests are not yet implemented by PR #20.
- All HOST-M1 tests remain blocked until a separately authorized AniCraft implementation exists.
- FD-M2 through FD-M5 Smart Controller plug-in work is not evidence that this FDM v2 host contract is implemented. In particular, numeric `warp.info` element/pin checks do not satisfy persistent target-generation and Path2D revision/snapshot requirements.

