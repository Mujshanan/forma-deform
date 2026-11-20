# AniCraft WebAssembly Plugin ABI v1

**ABI version:** `1` · **Guest target:** `wasm32-unknown-unknown` · **Wire format:** UTF-8 JSON  
**Guest SDK:** `crates/anicraft-plugin-sdk` · **Host:** AniCraft `anicraft_plugins` / wasmi

This document records the ABI contract consumed by this plugin workspace. The host implementation and its normative public reference live in the AniCraft repository at `docs/plugins.md` and `crates/plugins/src/runtime.rs`. Changes here and in the host contract must be reviewed together; this file must not silently redefine the runtime.

Plugins must use `anicraft-plugin-sdk`. They must not declare their own raw host imports, pointer packing, allocation helpers, or ABI export macros.

## 1. Versioning and compatibility

Every module exports `ac_abi_version() -> i32`, which returns exactly `1`.

- The host checks the version before reading the manifest or invoking kind-specific exports. Unsupported versions are rejected; the host must not guess.
- ABI version is independent of the plugin manifest version and the SDK Cargo version.
- ABI v1 signatures, required exports, import names/signatures, pointer packing, JSON envelopes, status conventions, and ownership rules are stable.
- Additive optional exports, optional manifest fields, and new request operations may be introduced compatibly. Unknown optional fields/exports are ignored; unknown operations receive an error response.
- Changing a required export/import, signature, existing field meaning, memory ownership, or response framing is breaking and requires ABI v2.
- The SDK emits optional `ac_dealloc(ptr, allocation_len)`. Existing ABI v1 hosts may ignore it; the current AniCraft runtime creates a fresh instance per run and reclaims allocations by dropping the instance.
- Do not add a required import in ABI v1. The host offers two optional imports; plugins may import either, both, or neither.

The ABI version is the compatibility gate. Plugin and SDK versions do not negotiate ABI features.

## 2. Required exports

Every plugin exports the WebAssembly linear memory as `memory`, plus:

| Export | Wasm signature | Meaning |
|---|---|---|
| `memory` | memory | Module linear memory; required for every kind. |
| `ac_abi_version` | `() -> i32` | Returns `1`. |
| `ac_manifest` | `() -> i64` | Packed pointer/length of UTF-8 manifest JSON. |

Kind-specific exports:

| Manifest kind | Required exports |
|---|---|
| `command` | `ac_alloc(i32) -> i32`, `ac_run(i32, i32) -> i32` |
| `frame-script` | `ac_alloc(i32) -> i32`, `ac_run(i32, i32) -> i32` |
| `exporter` | `ac_alloc(i32) -> i32`, `ac_export(i32, i32) -> i64` |
| `ease` | `ac_ease(i32, f64) -> f64` |

The SDK also exports optional `ac_dealloc(ptr: i32, allocation_len: i32) -> ()` for command and exporter modules. It is not required by the current host.

### Pointer/length returns

`ac_manifest` and successful `ac_export` return an `i64` bit pattern: low 32 bits are the unsigned linear-memory pointer; high 32 bits are the byte length. The value is `(length << 32) | pointer`. Unpack with unsigned masks and shifts; signed `i64` interpretation does not change the bits.

Manifest bytes are static, borrowed memory valid for the instance lifetime. The host copies them before dropping the instance and never deallocates them.

`ac_alloc(size)` returns a guest-owned buffer the host may write. **Zero means allocation failure.** The host passes the pointer and actual input length to `ac_run` or `ac_export`, and never writes past the allocated size.

## 3. Host imports

The host offers these optional imports from module namespace `anicraft`:

| Import | Wasm signature | Meaning |
|---|---|---|
| `anicraft.call` | `(req_ptr: i32, req_len: i32) -> i32` | Submit one JSON request; after handling it, the host resumes the guest with response byte length. |
| `anicraft.response` | `(dst_ptr: i32, dst_cap: i32) -> i32` | Copy up to `dst_cap` bytes of the last response; returns bytes copied. |

The SDK uses `usize` for lengths, represented as Wasm `i32` on `wasm32`.

Call/response is synchronous from the guest's perspective:
1. Guest calls `anicraft.call`; host copies request bytes out of guest memory.
2. Host handles the request and resumes the guest call with the response length.
3. Guest allocates a response buffer and calls `anicraft.response`.
4. Host copies the response and returns the copied byte count.

The runtime uses a resumable Wasm call to handle requests outside the guest invocation. It must not retain guest pointers. The response belongs to the immediately preceding call and is replaced by the next call. The SDK reads the reported response in one copy.

Responses use JSON envelopes:
- success: `{"ok":true,"result":...}`
- failure: `{"ok":false,"error":"..."}`

A refused request is an ordinary error response, not a trap. Ease plugins are sampled at load and do not use host imports.

## 4. Serialization

- Manifests, entry-point parameters, requests and responses are UTF-8 JSON without a byte-order mark (RFC 8259).
- Non-finite floating-point values are not valid JSON and must be rejected before serialization.
- `ac_run` receives the resolved parameter object itself; `ac_export` receives the resolved exporter options object itself.
- Host requests are operation objects such as `{"op":"document"}` or `{"op":"command","id":"bone.info","params":{}}`.
- Object key order and insignificant whitespace are not meaningful. Validate required fields and types.
- Do not serialize Rust structs, enums, pointers, or native memory layouts directly. Use JSON DTOs.
- Exporter output is opaque binary file data, not JSON; the output file format defines its representation.

## 5. Payload and runtime limits

The host enforces per-kind limits. These are ceilings, not targets.

| Limit | Command default | Frame script | Exporter |
|---|---:|---:|---:|
| Module size | 16 MiB | 16 MiB | 16 MiB |
| Linear memory per instance | 256 MiB | 256 MiB | 256 MiB |
| Manifest JSON | 64 KiB | 64 KiB | 64 KiB |
| Entry-point parameters | 64 KiB | 16 KiB | 64 KiB |
| One host request | 1 MiB | 64 KiB | 1 MiB |
| One host response | 8 MiB | 1 MiB | 32 MiB |
| Total request bytes per run | 64 MiB | 1 MiB | 64 MiB |
| Host calls per run | 100,000 | 1,000 | 1,000,000 |
| Export output | n/a | n/a | 64 MiB |
| Fuel / instructions | 1,000,000,000 | 20,000,000 | 50,000,000,000 |
| Wall-clock budget | 5 s | 100 ms | 10 min |
| Log lines retained | 200 | 50 | 200 |

Other defaults: 1,024 call frames; log lines are limited to 500 characters. Ease functions are sampled 257 times at load under the loading fuel budget, without host calls.

SDK general ceilings: 1 MiB per serialized host request; 32 MiB response buffer; 64 KiB entry-point parameters/allocation; 64 KiB manifest; 64 MiB exporter output. The host may impose lower per-kind limits. In particular, command responses are capped at 8 MiB although the shared SDK ceiling accommodates 32 MiB exporter responses.

Oversized host requests currently trap; oversized responses become error answers. Fuel, wall-clock, call-count, total-request, memory, and call-depth limits also apply. Command and frame-script runs get a fresh instance; exporters run in a bounded worker invocation.

## 6. Memory ownership and deallocation

ABI v1 uses instance-scoped allocation:

| Memory | Owner | Lifetime |
|---|---|---|
| `ac_manifest` bytes | Plugin module | Static for instance lifetime; host copies; never free. |
| `ac_alloc` input | Guest allocator | Host writes before entry-point call; plugin reads during it. |
| `ac_export` output | Guest allocator | Host copies before dropping exporter instance. |
| Bytes passed to `anicraft.call` | Guest | Borrowed for the import only; host copies before handling. |
| Destination passed to `anicraft.response` | Guest | Guest-owned buffer supplied for the copy. |

- Current AniCraft creates a fresh instance per run and drops it afterwards. Instance teardown is the authoritative reclamation mechanism for ABI v1 allocations.
- The host must not retain guest pointers after a call or instance lifetime. It copies manifest and exporter output bytes before dropping the instance.
- Optional `ac_dealloc(ptr, allocation_len)` permits early release for a host that chooses to use it. It must be called exactly once with the original allocation length, not the shorter used length. The current host does not call it; plugins must not rely on it.
- Invalid pointers, duplicate frees, and mismatched lengths are ABI violations. Static manifest memory must never be deallocated.

A future host that retains instances across runs must use the optional deallocator correctly or introduce a separately versioned lifetime protocol.

## 7. Reentrancy and concurrency

ABI v1 is synchronous and single-flight per instance.

- Plugins may make sequential host calls during one entry-point invocation.
- A host call is handled by the runtime, then the same guest invocation resumes. The host must not call another export on that instance.
- The host must not invoke exports concurrently on one instance or share one instance's linear memory across runs.
- Response state belongs to the immediately preceding `anicraft.call`.
- Threads, shared memory, asynchronous callbacks, reentrant exports, and retained guest pointers are not part of ABI v1.

## 8. Panic and trap behavior

A Rust panic must not unwind across an exported C ABI boundary. A panic, Wasm trap, out-of-bounds access, out-of-fuel condition, or wall-clock timeout fails the invocation. The host catches the Wasm failure at the runtime boundary, reports a failed run, and discards the instance. For command plugins, the document is restored to its pre-run state. Expected validation and operational failures must be returned as `Err(String)`, not panics. A trapped instance is not reusable.

## 9. Deterministic status codes

The host treats `ac_run == 0` as success and **any non-zero value as failure**. The SDK standardizes these values for guest code and tests:

| Code | SDK name | Meaning |
|---:|---|---|
| 0 | `STATUS_OK` | Handler completed successfully. |
| 1 | `STATUS_INVALID_ARGUMENT` | Invalid pointer/length combination or negative length. |
| 2 | `STATUS_INVALID_JSON` | Entry-point input is not valid UTF-8 JSON. |
| 3 | `STATUS_HANDLER_ERROR` | Handler returned `Err`; SDK logs the diagnostic. |
| 4 | `STATUS_PAYLOAD_TOO_LARGE` | Entry-point input exceeds the SDK parameter ceiling. |

The current host does not interpret non-zero values individually; it reports failure using the last plugin log line when available. Plugins may rely on the values in their own tests and tooling, but must not assume the host exposes distinct user-facing error categories.

Other conventions:
- `ac_alloc`: pointer on success; `0` on allocation failure.
- `ac_export`: packed output pointer/length on success; any negative value on failure.
- Host imports return a byte count on success; negative values indicate transport failure in the guest SDK contract.
- A trap does not return an `ac_run` status code.

## 10. Endianness and pointer representation

- Pointers are 32-bit offsets into WebAssembly linear memory.
- Wasm scalar values follow the WebAssembly core calling convention. Scalars stored in linear memory use little-endian byte order.
- Packed pointer/length values are numeric bit fields, not serialized Rust structs. Extract the low and high 32-bit halves with masks and shifts.
- JSON is UTF-8 text and has no integer byte order.
- Exporter output is opaque; its file format defines byte order.

Never rely on host-native pointer width, alignment, endianness, or Rust struct layout.

## 11. Conformance checklist

Plugin:
- [ ] Export `memory`, `ac_abi_version`, and `ac_manifest`.
- [ ] Export the required entry points for its manifest kind and report ABI version 1.
- [ ] Use SDK allocation, request, serialization, and export helpers.
- [ ] Stay within per-kind limits; use UTF-8 JSON DTOs.
- [ ] Return zero only for successful `ac_run`; use stable non-zero SDK status codes for failure.
- [ ] Never unwind across the ABI boundary or retain host pointers.
- [ ] Treat allocation as invocation-scoped; do not assume the host calls optional `ac_dealloc`.

Host:
- [ ] Check ABI version before reading the manifest.
- [ ] Require exported memory and kind-specific exports.
- [ ] Reject unsupported imports and validate imported function signatures.
- [ ] Enforce module, memory, manifest, parameter, request, response, output, fuel, time, call-count, and recursion limits.
- [ ] Bounds-check all pointer/length pairs against current linear memory.
- [ ] Copy borrowed request, manifest, and exporter output bytes before their memory expires.
- [ ] Preserve call/response sequencing and response envelopes.
- [ ] Avoid reentrant or concurrent calls on one instance.
- [ ] Treat non-zero `ac_run` returns and traps as failed invocations.
- [ ] Drop each instance after the run under the current ABI v1 lifecycle.
