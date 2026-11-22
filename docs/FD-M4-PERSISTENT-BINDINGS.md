# FD-M4 — Persistent Bindings

**Status:** first core payload slice implemented; AniCraft document-storage integration is not yet implemented.
**Depends on:** FD-M1 canonical mapper, FD-M2 read-only preview, and the M3 typed authoring builder.
**Ownership:** Forma Deform owns the versioned binding payload and its validation/migration rules. AniCraft owns the document extension field, file serialization boundary, dirty state, undo/redo, commands, and invalidation.

## 1. Why this requires host support

The audited AniCraft plug-in contract starts each WASM command invocation fresh, prohibits plug-in file access, and does not expose arbitrary document metadata. Plug-ins are installed on the host and do not travel with a document. Therefore durable bindings cannot be implemented safely inside the current command plug-in.

M4 requires a small first-party AniCraft extension. This document specifies the contract; it does not claim that the host API already exists and does not authorize guessing an SDK function or document field.

## 2. Persisted payload

The core now defines `PersistentBindingDocument` in `smart-controller-core::persistent`.

The envelope contains:
- `schema.name = "forma-deform-bindings"`
- `schema.version = 1`
- a monotonic collection `revision` (distinct from AniCraft's document revision)
- a nested canonical Smart Controller document (`smart-controller`, schema version 1) containing stable input, controller, binding, and target identities

The payload is independent of AniCraft storage APIs. It serializes as strict JSON for tests and host boundary transport; the plug-in itself does not read or write files.

Unknown fields and unsupported schema versions fail closed. The first persisted version is deliberately explicit; no implicit reinterpretation of an unknown future payload is allowed.

## 3. Core mutation semantics

The pure core payload supports:
- lookup by stable ControllerId;
- atomic upsert of a controller and its input definitions;
- atomic removal of a controller;
- validation of the complete controller graph before accepting a change;
- pruning inputs no longer referenced by any controller;
- preserving shared inputs only when their definitions are identical;
- a checked collection revision increment for each actual successful change.

A failed mutation leaves the original payload and revision unchanged. A controller cannot be removed while another controller still depends on it. The caller must update/remove dependents explicitly in the same host transaction.

The host must not perform partial writes while translating these operations into document commands.

## 4. Proposed AniCraft document boundary

The host-side implementation should add a versioned, optional Forma Deform extension to the authoritative document model. Exact Rust field names and command IDs are for the AniCraft PR to decide after inspecting the current document/file-format architecture.

Minimum contract:

1. **Read** the extension as an owned typed payload or a typed absence result.
2. **Add/update** one controller and its input definitions as one validated document command.
3. **Remove** a controller, rejecting unresolved dependents unless the same command explicitly updates/removes them.
4. **Inspect/list** persisted controllers and bindings without exposing mutable document internals.
5. Validate the full payload before commit and after file load.
6. Preserve unknown unrelated document extension data and reject malformed Forma Deform payloads with a recoverable, actionable load error (or a clearly defined quarantine policy); never silently drop authored bindings.
7. Include the extension in the native document save/load path. A save/load round trip must preserve stable IDs, numeric values, optional composition fields, and schema version.
8. Every persistent edit must be one AniCraft undo step; undo/redo restores the entire payload and collection revision atomically.
9. Any host cache/invalidation hook must be tied to AniCraft's document revision. Do not add a parallel cache or dirty-state system in Forma Deform.

The host extension must be versioned and covered by AniCraft document compatibility tests. The plug-in ABI should expose only typed command/query contracts that have been implemented and reviewed in AniCraft.

## 5. Migration policy

- Storage envelope v1 wraps canonical Smart Controller document v1.
- Canonical document validation remains the single source of truth for controller/input graph integrity.
- Unsupported envelope or nested canonical versions fail closed with an explicit version error.
- A future migration must be a named, tested transformation from a known prior version; it must not mutate the input payload in place.
- The first save of an M3 ephemeral dialog draft is an explicit authoring/save action. M3 settings are not silently auto-imported or persisted.
- Stable IDs are authored data. Never regenerate IDs on load, save, copy, or migration unless a named migration explicitly reports the identity change.

## 6. Security and resource limits

The AniCraft host must bound extension payload size, controller count, input count, dependency edges, binding count, string lengths, and numeric values before allocation/commit. The core validates semantic correctness; the host enforces document-level resource limits and command authorization. No plug-in-supplied path, URL, filesystem location, or arbitrary executable content is stored.

## 7. Acceptance criteria

- [x] Define a strict versioned persistence envelope in the host-independent core.
- [x] Validate the nested canonical controller document before serialization/load.
- [x] Add atomic core upsert/remove operations and collection revision semantics.
- [x] Unit-test round trip, unknown fields/versions, invalid atomic updates, and input cleanup.
- [ ] Review and implement the document-owned extension in AniCraft.
- [ ] Add typed host commands/queries for list, inspect, upsert, and remove.
- [ ] Verify native save/load and document compatibility behavior.
- [ ] Verify undo/redo restores the full binding payload atomically.
- [ ] Verify host revision/invalidation behavior and payload limits.
- [ ] Exercise end-to-end persistence in AniCraft with the M3 authoring flow.

## 8. Guardrails

- No plug-in file I/O or sidecar storage.
- No new Forma Deform document model; the nested canonical document is only the payload schema.
- No direct mutation of AniCraft internals from WASM.
- No invented host SDK methods or command IDs.
- No live playback evaluator (M6) or apply/bake target writes (M5).
