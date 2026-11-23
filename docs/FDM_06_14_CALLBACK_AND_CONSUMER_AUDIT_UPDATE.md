# FDM-06.14 — Callback and Consumer Evidence Update

**Disposition: evidence improved; S0 remains OPEN.** Static audit against AniCraft commit `2df7775907d1458ce653e7e6314ec8a5d7d565f5` and tree `80071fb3f714944e49608668871d4cf53a6159a5`. This is not compiler-backed proof.

## 1. Callback audit

The original locator marked 36 of 131 `mutates: false` entries unresolved. All 36 now have a source registration/callback location and first-order effect class in the attached evidence bundle.

The 36 include read-only queries; transient selection/navigation/scene/playhead/editing/view/envelope/transform context changes; history restoration (`edit.undo`, `edit.redo`); and external filesystem writes (`desktop.integrate`, export commands, `file.save_as`).

This resolves the callback-location gap, not the complete effect-graph gap. Every one of the 36 still requires helper/transitive effect verification. The other 95 false-flag callbacks also need a full transitive review. A lexical token scan is not proof of purity or effect containment. The false-flag execution branch also calls settlement operations including `settle_floating`, which can mutate document state.

## 2. Workspace consumer scan

A targeted repository-wide lexical scan, excluding `crates/engine/**`, found **762 candidate API call-site lines across 94 files**:

| API surface | Candidate lines |
|---|---:|
| `Session.doc` access | 382 |
| `CommandSpec` / `RunFn` references | 9 |
| `.execute(...)` calls | 68 |
| gesture begin/end calls | 98 |
| `replace_document(...)` calls | 1 |
| `undo` / `redo` calls | 10 |
| public nested Document mutator calls | 208 |

The row-level inventory covers app `src/` (502 candidate lines), `crates/doc` (159), `crates/render` (48), `crates/xfl` (22), `crates/cli` (19), and `crates/gpu` (12). It includes both test and non-test code; path filtering cannot identify inline `#[cfg(test)]` modules. These are candidate references, not 762 unique semantic consumers.

The migration boundary is therefore broader than GUI command handlers: document construction, import/export and library code also use public mutators. The scan does not establish the full call graph, borrow mutability, or compile impact. Compiler-backed workspace verification remains required.

## 3. Host CI evidence

AniCraft `.github/workflows/ci.yml` defines Linux workspace tests + Clippy and cargo-deny. `.github/workflows/platforms.yml` defines macOS and Windows build/test/Clippy, but its PR path filters skip docs-only changes.

A source-neutral audit CSV was added to AniCraft baseline PR #1 at head `c1c034df9f0308174b4708fafe00d05259226bca`, based on pinned source SHA `2df7775907d1458ce653e7e6314ec8a5d7d565f5`. It changes no Rust source, Cargo manifest, lockfile or workflow. The latest Actions API query still reports zero workflow runs and zero status contexts for that host PR head. No host test, Clippy, cargo-deny, macOS or Windows pass is claimed.

The GitHub connector exposes no workflow-dispatch action, and local `rustc`/`cargo` are unavailable. Host CI evidence cannot be manufactured from source inspection.

## 4. S0 reassessment

| Gate | Current result |
|---|---|
| Pinned source integrity | PASS |
| Registry completeness (52 providers / 395 IDs) | PASS at registry-enumeration level |
| All 131 false-flag callback locations | PASS at location/first-order triage level |
| Full callback effect graph | OPEN |
| False-flag transaction safety | FAIL / OPEN |
| Non-command mutation inventory | PARTIAL |
| Compiler-backed consumer proof | OPEN |
| Host tests, Clippy, cargo-deny | OPEN |
| macOS/Windows CI | OPEN |
| S0 closure | **NOT CLOSED** |

## 5. Required remaining evidence

1. Complete transitive effect review for every command callback, including helpers, plugin/macro nested calls, settlement and filesystem/preferences effects.
2. Classify every candidate consumer as production/test, read-only/mutable, owner boundary and migration target; verify call graph with a Rust compiler or equivalent semantic index.
3. Obtain host CI for the exact pinned source tree: tests, Clippy, cargo-deny, macOS and Windows.
4. Reassess S0 only after these artifacts are attached and independently reproducible.

## 6. Non-authorization

No runtime implementation, FDM evaluator, host integration, adapter bypass, asynchronous evaluation, apply/bake, Asset Warp change, Smart Controller/FDM composition, merge or release is authorized. FDM-06.6 remains unimplemented and unverified. S0 remains OPEN.
