# Native Smart Controller host contracts

**Status:** M2 contract baseline — identity/input contracts defined; neutral capture, durable binding, and target mutation remain out of scope.  
**Host:** AniCraft  
**Core:** `smart-controller-core`  
**Contract revision:** 1

This document defines the boundary between the host-independent controller core and the AniCraft adapter. It turns existing data structures and verified read-only commands into normative requirements. It does not authorize neutral capture, binding controls, persistence, playback evaluation, or target writes.

## 1. Boundary and ownership

- `smart-controller-core` owns canonical input definitions, typed values, immutable input snapshots, controller evaluation, composition, target mappings, and validation.
- AniCraft owns the live document, bone/element/pin identity resolution, current frame, and all document mutation/undo transactions.
- `plugins/smart-controller` is the adapter. It may query documented read-only host commands and convert their results into core types. It must not infer host identity from names, array positions, UI selection order, or geometry coordinates.
- A successful calculation or capability assessment is not permission to mutate the document.
- Smart Controller evaluation remains independent of Form Deform pin evaluation (CCC-01.5).

## 2. Contract A — target identity and capability discovery

### 2.1 Canonical target identity

The canonical core target remains a typed `TargetMapping`. For geometry deformation, the host locator is the pair:

- `element_id`: numeric AniCraft `ElementId`, serialized as a decimal string in the canonical mapping.
- `pin_id`: numeric AniCraft `PinId`, serialized as a decimal string in the canonical mapping.

Neither a layer name, artwork name, pin coordinate, pin-array index, nor current UI selection index is a stable target identifier.

AniCraft's `ElementId` and `PinId` are unique within a document, not globally. The inspected AniCraft `Document` model has no stable document UUID. Therefore `geometry:{element_id}:{pin_id}` is only a locator within the active document context. It must not be treated as a globally unique or cross-document persistent key. Durable binding persistence is blocked until the host exposes a stable document identity (or an equivalent collision-safe document scope) and defines its lifecycle across save, duplicate, import, and reopen.

### 2.2 Read-only discovery command

The adapter queries:

```json
{"element": 17, "frame": 24}
```

through the documented `warp.info` command. The optional frame is the zero-based AniCraft frame; omit it only when intentionally querying the current playhead outside a deterministic snapshot.

The response schema relevant to capability discovery is:

```json
{
  "element": 17,
  "layer": 4,
  "warped": true,
  "bitmap": false,
  "bent": false,
  "pins": [
    {"pin": 12, "at": [120.0, 80.0], "rest": [100.0, 70.0]}
  ]
}
```

When no live warp exists, AniCraft returns `warped: false` and does not return a pin list. The pin identifier field is **`pin`**, not `id`. The adapter must validate the actual host response shape and must never use array position as identity.

### 2.3 Resolution states

Every target lookup must produce one of these explicit states:

| State | Meaning | UI consequence |
|---|---|---|
| `Supported` | Exact element and pin IDs match; live warp exists; artwork is not a bitmap | May be described as a geometry-pin target, but application is still preview/undo-only |
| `Unsupported` | Host confirms bitmap artwork, no live warp, or requested pin does not exist | Do not offer binding or capture controls |
| `Unavailable` | Host query failed, response is malformed, or returned element identity differs | Show unavailable/error state; do not silently retry against another element |

A resolved geometry pin has a capability ceiling of `CommandPreviewOnly`, not direct mutation. `warp.pin_move` is a mutating command. It may only be invoked after AniCraft exposes and the integration verifies the host command-dialog preview/undo transaction. This contract does not add that transaction.

Other current mappings retain their existing limits:

- `SymbolFrame`: calculation-only; no verified displayed-frame setter.
- `FrameRange`: calculation-only; no verified action-layer frame setter.
- `Property`: calculation-only; no verified generic property setter.
- `GeometryDeform`: exact pin discovery is possible through `warp.info`; mutation remains preview/undo-only.

A selected artwork element is not automatically a supported deformation target. Selection inspection in the M2 Properties panel is identity context only.

## 3. Contract B — deterministic bone-input snapshot

### 3.1 Core representation

The adapter must produce the existing host-independent core types:

- `InputDefinition` identifies an input, source, dimension, and authored fallback policy.
- `InputSample` contains a typed value or an explicit validity state (`Missing`, `Invalid`, or `Unavailable`).
- `InputSnapshot` is an immutable collection for one evaluation invocation and rejects duplicate input IDs.
- `TypedValue::Angle` is expressed in radians and carries explicit wrapped/unwrapped semantics.

The adapter must not add a second input model or let host SDK types enter `smart-controller-core`.

### 3.2 Explicit sampling context

Every bone input query in a deterministic snapshot must use:

- an explicit canonical bone ID;
- an explicit bone-ID → AniCraft armature-layer-ID mapping;
- an explicit zero-based frame;
- an explicit `BoneAnglePolicy`;
- explicit wrapped/unwrapped angle semantics.

The adapter queries `bone.info` once per required armature layer per invocation, passing both `layer` and `frame`. It verifies that the response's `frame` equals the requested frame and that the response contains the exact mapped armature `layer` before using any bone data. A response from another frame or layer is unavailable, not a usable stale sample.

The relevant response is:

```json
{
  "frame": 24,
  "armatures": [
    {
      "layer": 4,
      "bones": [
        {"id": 42, "name": "Head", "angle": 30.0, "swing": -5.0}
      ]
    }
  ]
}
```

Bone IDs are matched numerically by `id`; names are presentation only. A bone missing from the explicitly mapped layer response is invalid. The adapter must not search a different layer as a fallback.

### 3.3 Angle semantics

AniCraft's read-only `bone.info` reports:

- `angle`: the constrained keyed pose angle, in degrees. This is the authored/keyed pose after the bone's joint limits are applied.
- `swing`: the difference, in degrees, between the evaluated spring pose and the constrained keyed pose. It is secondary motion, not part of the keyed pose.

The adapter's policy is explicit:

- `KeyedPoseOnly`: use `angle`.
- `KeyedPosePlusSpringSwing`: use `angle + swing`.

Missing `angle` or (when requested) `swing` is an invalid sample. The adapter must not silently substitute zero for a missing field. It converts the selected degree value to radians and applies the caller-authored wrapped/unwrapped flag; wrapping is never inferred from the host value.

### 3.4 Snapshot freshness and failures

- A sample with `age_ticks: Some(0)` means it was read or supplied during this same invocation. It makes no claim about wall-clock freshness after the invocation.
- The adapter does not read a wall clock or reuse samples across invocations.
- A failed query or mismatched frame produces `Unavailable`.
- A missing bone or malformed angle produces `Invalid`.
- A missing manual value produces `Missing`.
- The core applies the input's authored `FallbackPolicy`; the adapter does not inject a default numeric value.
- Controller-output inputs are resolved by the controller dependency graph and are not queried from AniCraft.
- Only Manual and BoneTransform Rotation inputs are currently provided by this host adapter. Translation, scale, and legacy scalar inputs fail closed until their units and host semantics are specified.

## 4. Explicit non-goals and gates

These contracts do not implement:

- neutral/reference pose capture;
- binding creation, editing, or persistence;
- document UUID generation or cross-document identity;
- generic target adapters;
- host mutation, undo, or preview transactions;
- timeline playback/scrubbing evaluation;
- implicit composition with Form Deform pin evaluation.

Before neutral capture or binding UI can be implemented, the host must provide stable document scope for persisted locators, and the product must define whether neutral values are captured from keyed pose only or from keyed pose plus spring swing. Capture must use the same explicit frame and angle policy as the input snapshot contract. Binding controls remain absent until those semantics and persistence/undo ownership are approved.

## 5. Conformance tests

The canonical `collect_host_input_snapshot` path is frame-pinned by this contract. The older V1 Slider command remains a separate current-playhead, on-demand preview path; it is not to be represented as a frame-pinned canonical snapshot until it is migrated explicitly.

The adapter conformance suite must cover:

1. Exact `warp.info` schema: `pins[].pin` identity, not `pins[].id` or array index.
2. Correct element match, live warp, non-bitmap, and exact pin match.
3. Distinct Unsupported and Unavailable outcomes.
4. Explicit frame passed to `bone.info` and exact response-frame verification.
5. Numeric bone-ID match and exact response-layer verification against the explicitly mapped armature layer.
6. Keyed angle only versus keyed angle plus spring swing.
7. Missing spring swing, malformed angles, missing bone, failed host query, and frame mismatch fail closed.
8. No implicit zero, no cross-invocation cache, and no host mutation.

CI/build and AniCraft in-app smoke tests are required before claiming host conformance. Static source inspection or the presence of tests is not a passing test result.
