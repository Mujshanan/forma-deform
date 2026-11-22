# AniCraft Host Project-Storage Capability Audit

**Audited:** 2026-11-22  
**Host repository inspected:** `Mujshanan/anicraft`, branch `main`  
**Consumer repository:** `Mujshanan/forma-deform`, branch `feat/smart-controller-phase6a-mapping`  
**Scope:** Determine whether the existing AniCraft plug-in contract can persist the separately versioned Smart Controller advanced-mapping extension without modifying AniCraft.

## Executive result

<badge>Host extension required</badge>

The current AniCraft command plug-in API does **not** provide durable document metadata storage. A plug-in cannot save files or access arbitrary filesystem paths, and the ABI exposes only `command`, `document`, `elements`, and `log` requests. The host's native `.anicraft` format serializes a concrete `Document` model; it has no generic plug-in metadata/extensions collection.

Therefore, the extension implemented in Forma Deform can be serialized and loaded by its core API, but an ordinary installed command plug-in cannot yet save it into an AniCraft project. Do not implement a sidecar file or claim project-level persistence through the current SDK.

## Evidence

### 1. Native project model is a closed, typed document structure

In `Mujshanan/anicraft/crates/doc/src/lib.rs`, `Document` is a concrete `Serialize + Deserialize` struct. Its persisted fields are stage, scenes, library, bitmaps, sounds, library tree, document class, publish profile, swatches, aids, eases and next ID. There is no generic metadata map, extension bag, or plug-in-owned payload field.

The deserialization helper `DocumentFile` enumerates the same known document fields and applies defaults. It does not declare an extension field. Consequently, an arbitrary JSON property added by a plug-in to the document payload is not a supported storage contract and would not round-trip as document-owned data.

Source: https://github.com/Mujshanan/anicraft/blob/main/crates/doc/src/lib.rs

### 2. Native format is a versioned JSON envelope

In `Mujshanan/anicraft/crates/engine/src/format.rs`:

- The native format is a JSON envelope containing `format`, `version`, `generator`, `paths`, and `document`.
- `to_json` serializes the typed `Document` into that envelope.
- `from_json` decodes the envelope into the typed document, validates the format/version and sanitizes the result.
- `save` writes the resulting JSON atomically through a temporary file and rename.

The current format version is 4. The writer does not expose a generic extension payload slot. Adding a supported persisted field therefore belongs in the host document/format model and needs a deliberate backward-compatibility policy.

Source: https://github.com/Mujshanan/anicraft/blob/main/crates/engine/src/format.rs

### 3. Plug-in ABI has no file or arbitrary metadata operation

The current guest SDK exposes `command`, `document`, `selected_elements`, and `log`. The host dispatch in `crates/engine/src/cmds_plugin.rs` accepts the ABI v1 operations `command`, `document`, `elements`, and `log`.

The command plug-in policy explicitly rejects the `file` namespace with the message that plug-ins cannot open, save, import, export or publish files. The public plug-in guide likewise says plug-ins cannot open/save files or touch the filesystem, and that plug-ins belong to AniCraft on the computer rather than to a document.

Sources:
- https://github.com/Mujshanan/anicraft/blob/main/examples/plugins/sdk/src/lib.rs
- https://github.com/Mujshanan/anicraft/blob/main/crates/engine/src/cmds_plugin.rs
- https://github.com/Mujshanan/anicraft/blob/main/docs/guide/plugins.md

### 4. Existing command path can be extended, but it is not storage by itself

Command plug-ins may call only registered commands that pass `plugin_may_call`. The current policy permits the `doc` namespace, but this does not create a generic metadata API: a specific host command must be implemented, registered, validated and tested. Existing `document()` is a read-only compact view, not a serialization or metadata write operation.

A future host extension could expose narrowly scoped commands such as:

- `doc.smart_controller_extensions_get`
- `doc.smart_controller_extensions_set`

These are design candidates only; they do not exist today. The setter must validate extension ID/version, payload size, JSON structure and host-level ownership rules, and must participate in the existing document edit/undo transaction. The getter should return only the registered Smart Controller extension data, not the entire native document.

## Capability matrix

| Capability | Current status | Evidence / consequence |
|---|---|---|
| Core extension JSON encode/decode | Supported in Forma Deform | Implemented and CI-tested on the Phase 6A branch |
| In-memory runtime registry reconstruction | Supported in Forma Deform | Extension loader builds the existing runtime registry |
| Arbitrary plug-in file I/O | Not supported | Sandbox and command policy prohibit file access |
| Generic document metadata bag | Not present in inspected model | `Document` and `DocumentFile` enumerate known fields |
| Plug-in-owned project extension storage | Not supported | No SDK or host command contract for it |
| Registered host commands | Supported | A first-party AniCraft change can add explicit, narrow commands |
| Document undo/redo participation | Host-supported for command runs | New extension updates should use the normal document edit transaction |
| Durable save/reopen | Requires host model/format work | Must be verified with round-trip and older-file tests |
| Live per-frame controller evaluation | Not established by this audit | Remains separate from project storage and preview-only limits |

## Recommended host integration contract

Do not change Smart Controller canonical schema v1. Keep the extension envelope separate from `Controller`, `MappingConfig` and the canonical `SchemaEnvelope`.

The smallest viable host-side implementation is:

1. Add an optional, versioned Smart Controller extension payload to AniCraft's typed `Document` model. Use an explicit extension identity and payload/version boundary; do not create a generic unvalidated arbitrary JSON bag unless AniCraft independently wants that platform capability.
2. Add the corresponding defaulted field to `DocumentFile` and include it in the typed serialization/deserialization path.
3. Define a host command contract to get/set the extension. The setter must validate the payload, mutate the document through the normal command transaction, support undo/redo, and reject malformed/unsupported versions without silently dropping data.
4. Register and explicitly allow the new command IDs for command plug-ins.
5. Expose only the extension payload through the plug-in ABI's existing command request. No new filesystem access or generic file operation is required.
6. Add host tests proving create → set → save → close/reopen → get equality; update → undo → get previous payload; redo → get updated payload; malformed payload rejection; unknown extension/version behavior; and legacy files without the field.
7. Decide how older AniCraft builds behave when opening a file containing the new field. A build that cannot preserve the extension must not silently rewrite and discard it. Prefer a format-version bump or explicit preservation/unsupported-version behavior consistent with AniCraft's native-format policy.

The exact host data shape, command names and format-version strategy must be reviewed in the AniCraft repository before implementation. This audit does not authorize changes to that repository.

## Forma Deform boundary after host support exists

Once the host contract is implemented and verified, the Forma Deform adapter can:

1. Call the host getter and decode `AdvancedMappingExtension`.
2. Validate it against the canonical Smart Controller document and rebuild `AdvancedMappingRegistry`.
3. Run the existing `AdvancedControllerMapper` path.
4. On an explicit user save/authoring action, encode and pass the extension to the host setter.
5. Keep old documents with no extension on the existing V1 mapper path.

No change is required to the pure grid/triangulated mapping algorithms, the evaluator pipeline, contribution composition, or the separation from Form Deform pin evaluation.

## Exit decision

**The audit is complete; the storage capability gate is not met by the current host SDK.** The next implementation must be a separately reviewed first-party AniCraft host extension. Until that host work lands, Forma Deform should keep the persistence contract and runtime registry available for tests/integration, but must not claim that advanced mappings persist inside actual AniCraft project files.
