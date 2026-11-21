# CCC-01.7 — Compatibility Verification

- **Milestone:** CCC-01.7 — Compatibility Verification
- **Status:** Verification changes implemented; automated CI pending
- **Compatibility authority:** `docs/CODE_COMPATIBILITY.md` (CCC-COMP-001)
- **Source baseline:** `6b53aac51ddc15249c5100745726027fd9c75907`
- **Verification branch:** `test/ccc-01-7-compatibility-verification`
- **Scope:** Verify the CCC-01.3 through CCC-01.6 consolidation without changing runtime semantics.

## 1. Verification approach

This milestone adds characterization coverage for compatibility-sensitive numeric behavior and records what the existing tests do and do not prove. It does not redefine the baseline, approve breaking changes, or resolve deferred FDM v2 decisions.

A passing automated suite demonstrates the behavior exercised by those tests. It does not prove a real AniCraft host session, a complete ABI security audit, or compatibility with every historical external consumer.

## 2. New regression coverage

### Pin deformation

Added tests in `crates/form-deform-core/src/lib.rs` for:

- Non-finite values in every component of the deformation specification.
- Overflow while calculating the angular delta.
- Overflow while calculating the resulting absolute position.

Existing tests continue to cover neutral position, positive/negative angle response, and repeated non-accumulating evaluation.

### Controller mapping

Added tests in `crates/smart-controller-core/src/mapping.rs` for:

- Clamping and reversed rotation endpoints.
- Non-finite inputs and degenerate rotation ranges.
- Inclusive and exclusive frame endpoints.
- Frame clamping and degenerate frame intervals.
- The current `transform_value` behavior where `additive=true` and `false` produce the same mapped value. This is a characterization test, not endorsement of additive composition semantics.

### Public API and binding persistence

Existing `binding_api_compatibility.rs` tests verify canonical, legacy, and root import paths resolve to the same public types, and verify schema-v1 serialization/deserialization and representative evaluation.

Existing binding tests cover duplicate IDs, stable-ID CRUD, disabled bindings, deterministic evaluation, malformed numeric values, and rejection of unsupported schema versions.

## 3. Compatibility evidence matrix

| Contract area | Evidence available | Status / limitation |
|---|---|---|
| Pin evaluator | Core unit tests, including new numeric overflow and non-finite cases | Automated |
| Controller normalization/easing/modes/anchors | Binding implementation unit tests and mapping tests | Automated; no baseline-vs-head differential harness |
| Public controller API | Canonical/legacy/root compile-time compatibility integration test | Automated |
| BindingSet schema v1 | Round-trip and representative evaluation tests; unsupported version test | Automated; no archived corpus of historical production files |
| Shared Vec2 | Public re-export tests and x/y serde-shape test | Automated |
| Independent integration | Independent output and failure-isolation tests | Automated; composition intentionally not tested or defined |
| Adapter host response parsing | Plugin helper tests and fixtures | Partial; command-level host-call behavior not fully exercised |
| Form Deform apply/undo | Apply remains explicit in adapter source | Not proven by a live host; undo semantics need AniCraft runtime verification |
| Smart Controller preview-only | Adapter source and preview calculation tests | Source-level / automated core; no live host mutation-observation test |
| Bone Query read-only | Diagnostic implementation and parsing tests | Source-level; no live host mutation-observation test |
| AniCraft SDK / ABI v1 | Existing native test stand-ins and build coverage | Partial; malformed pointer/length, allocation bounds, and full unsafe-boundary audit remain outstanding |
| TypeScript geometry/timeline/document | Existing Bun test suite and build | Automated suite; no separate archived v1 document fixture corpus identified in this milestone |
| WASM artifacts | Release build for all three plugin crates | Automated build only; not a host-load test |
| Formatting | `cargo fmt --all -- --check` added to CI | Automated when CI completes |

## 4. Runtime verification still required

The following must not be represented as proven by GitHub Actions:

1. Load each generated WASM plugin in the supported AniCraft host build.
2. Verify Bone Query reads host state and performs no mutation.
3. Verify Form Deform calculation-only mode performs no mutation.
4. Verify Form Deform apply mode moves the selected pin and the host undo command restores the prior state.
5. Verify Smart Controller V1 emits preview diagnostics without timeline writes or document mutation.
6. Exercise malformed or missing host responses in the real host/SDK boundary where feasible.

Record host version, plugin artifact hashes, test project, exact steps, and observed outcomes in a separate runtime test report before declaring these checks passed.

## 5. Deferred security and architecture work

- SDK unsafe pointer/length/allocation audit remains separate from CCC-01.7.
- Historical external consumer compatibility has not been established.
- No new binding/deformation composition behavior is introduced.
- Geometry ownership, coordinate-space semantics, multi-source composition, animation ownership, baking, and runtime representation remain deferred FDM v2 decisions.

## 6. Exit decision

CCC-01.7 can report automated compatibility checks as passed only after the workflow on this branch completes successfully. It must continue to report host-runtime and SDK security verification as outstanding until supported evidence exists.

CCC-01.7 does not authorize merging any stacked CCC-01 pull request. CCC-01.8 should use this evidence matrix to identify remaining readiness gaps rather than treating a green CI run as complete FDM v2 readiness.
