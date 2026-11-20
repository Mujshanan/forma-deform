# AniCraft WebAssembly Plugin ABI v1

**Status:** Normative guest/host contract  
**ABI version:** `1`  
**Wire encoding:** UTF-8 JSON  
**Maximum payload:** 16 MiB (16,777,216 bytes)  
**Guest target:** `wasm32-unknown-unknown`  
**SDK implementation:** `crates/anicraft-plugin-sdk`

This document is the source of truth for the binary boundary between an AniCraft plugin module and the AniCraft plugin runtime. Plugin crates must use `anicraft-plugin-sdk`; they must not independently declare host imports, reproduce export macros, or invent pointer/length conventions.

## 1. Versioning and compatibility

Every module exports `ac_abi_version() -> i32`. ABI v1 returns exactly `1`.

- The host must read and validate the ABI version before calling any other plugin export.
- A host that does not support the returned version must reject the module cleanly. It must not guess or attempt a nearby version.
- ABI version is a major contract identifier, not the plugin's manifest version or the SDK crate's Cargo version.
- ABI v1 signatures, meanings, status codes, payload encoding, ownership rules, and required imports are immutable.
- A backward-compatible v1 extension may add an optional export or optional manifest metadata. Hosts must ignore unknown optional exports and unknown optional metadata.
- A v1 change must not add a required import, change an existing signature, change an existing status meaning, increase a required payload size, or change ownership semantics.
- Any incompatible change requires ABI version 2 and a separately documented contract. A v2 host may support v1 and v2 side-by-side; it must not reinterpret v1 modules as v2.
- SDK releases may fix implementation defects while preserving the v1 contract. A plugin's manifest `version` is independent.

The host should feature-detect optional exports by export name. In particular, `ac_dealloc` is an optional v1 lifecycle extension. When absent, the host must reclaim the entire module instance after the operation; it must not retain a long-lived instance and repeatedly allocate without a deallocation path.

## 2. Module kinds and required exports

All functions use the WebAssembly core ABI and are exported with unmangled names.

### 2.1 Common exports

Every plugin kind must export:

| Export | Wasm signature | Meaning |
|---|---|---|
| `ac_abi_version` | `() -> i32` | Returns `1`. |
| `ac_manifest` | `() -> i64` | Returns a packed pointer/length pair for the UTF-8 manifest JSON. |

The packed `i64` is a bit field: low 32 bits are the unsigned linear-memory pointer; high 32 bits are the byte length. Treat it as an unsigned 64-bit bit pattern even though the Wasm function type is `i64`. Manifest memory is static, borrowed, and valid for the lifetime of the module instance. The host must copy it before destroying the instance and must never call `ac_dealloc` for it.

### 2.2 Command plugins

A command plugin must additionally export:

| Export | Wasm signature | Meaning |
|---|---|---|
| `ac_alloc` | `(size: i32) -> i32` | Allocates a guest-owned input buffer of exactly `size` bytes. Returns its pointer, or `-1` on failure. |
| `ac_run` | `(ptr: i32, len: i32) -> i32` | Reads one UTF-8 JSON parameter object and invokes the command handler. |
| `ac_dealloc` | `(ptr: i32, allocation_len: i32) -> ()` | Optional v1 extension; releases a buffer previously returned by `ac_alloc`. |

### 2.3 Exporter plugins

An exporter plugin must additionally export:

| Export | Wasm signature | Meaning |
|---|---|---|
| `ac_alloc` | `(size: i32) -> i32` | Allocates a guest-owned options buffer. |
| `ac_export` | `(ptr: i32, len: i32) -> i64` | Reads options JSON and returns packed pointer/length for the output bytes; `-1` means failure. |
| `ac_dealloc` | `(ptr: i32, allocation_len: i32) -> ()` | Optional v1 extension; releases an input or output buffer. |

The host must copy the exporter output before releasing it. Exported bytes are opaque file data, not ABI JSON; their format and any endianness are defined by the exporter and the file format it produces.

### 2.4 Ease plugins

An ease plugin must additionally export:

| Export | Wasm signature | Meaning |
|---|---|---|
| `ac_ease` | `(index: i32, t: f64) -> f64` | Returns the sampled easing value. |

Ease plugins do not import the AniCraft host API and do not need allocation exports.

A module exports the entry points for its declared kind only. A host must not call an entry point belonging to another kind.

## 3. Host imports

Command and exporter plugins import exactly these functions from the module namespace `anicraft`:

| Import | Wasm signature | Meaning |
|---|---|---|
| `anicraft.call` | `(ptr: i32, len: i32) -> i32` | Synchronously submits a UTF-8 JSON request. Returns the response byte length; a negative value indicates transport failure. |
| `anicraft.response` | `(ptr: i32, cap: i32) -> i32` | Copies the response for the immediately preceding call into guest memory. Returns copied byte count; a negative value indicates transport failure. |

The guest SDK declares `len` and `cap` as `usize`; on `wasm32` their ABI representation is `i32`.

The host must:
1. Validate the request pointer and length against the current guest memory and payload limit.
2. Copy or fully consume the request bytes before `anicraft.call` returns. It must not retain a guest pointer.
3. Produce one response byte sequence for that call.
4. Make that response available to the immediately following `anicraft.response`.
5. Reject response sizes above the payload limit.
6. Return the exact response length from `call`, and copy no more than `cap` bytes in `response`.

`anicraft.response` is single-use call state. It is replaced by the next `anicraft.call`. The SDK requires the host to return the full response in one copy; partial response delivery is not supported.

Ease modules must have no imports. WASI imports, clocks, filesystem, ambient randomness, network access, and other host modules are outside ABI v1. The runtime should reject unexpected imports rather than silently provide them.

## 4. Serialization and wire format

- Request and response payloads are UTF-8 JSON text, without a byte-order mark.
- JSON objects, arrays, strings, booleans, numbers, and null follow RFC 8259.
- Non-finite floating-point values (`NaN`, positive infinity, negative infinity) are not valid JSON and must be rejected before serialization.
- A command's `ac_run` input is the parameter object itself, not an additional envelope.
- SDK host requests use an operation object, for example `{"op":"document"}` or `{"op":"command","id":"bone.info","params":{}}`.
- Host responses use `{"ok":true,"result":...}` for success and `{"ok":false,"error":"..."}` for failure. The `error` field is a human-readable diagnostic, not a stable machine code.
- JSON object key order is not significant. Consumers must not rely on property order or whitespace.
- Integer identifiers must remain integers in the JSON model; consumers must not silently round values outside the exact integer range of their numeric representation.

Binary exporter results are not JSON encoded by the ABI. They are a pointer/length byte slice and must be interpreted only according to the exporter's declared file format.

## 5. Payload and memory limits

The maximum size of every ABI payload is **16 MiB (16,777,216 bytes)**. This applies to:
- manifest JSON;
- command input JSON;
- host request JSON;
- host response JSON;
- exporter options JSON;
- exporter output bytes.

The limit is inclusive. A payload of exactly 16 MiB is allowed; a larger payload must be rejected before copying, parsing, or allocating it. The host should also impose a reasonable total linear-memory ceiling and execution/fuel budget; the 16 MiB per-payload limit is not a total-memory or CPU limit.

All pointers are offsets into the module's current WebAssembly linear memory, not native process pointers. The host must bounds-check every pointer/length pair against the current memory size before reading or writing.

## 6. Memory ownership and deallocation

Ownership is explicit:

| Data | Owner | Lifetime / release |
|---|---|---|
| Manifest bytes from `ac_manifest` | Plugin module | Static for the instance lifetime; host copies; never deallocate. |
| Input buffer from `ac_alloc` | Plugin guest allocator | Host writes before calling `ac_run`/`ac_export`; guest reads synchronously; host calls `ac_dealloc` with the original allocation length when available. |
| Exporter output from `ac_export` | Plugin guest allocator | Host copies bytes before calling `ac_dealloc` with the exact output length. |
| Request bytes passed to `anicraft.call` | Plugin guest | Borrowed only for the duration of the synchronous import; host copies before return. |
| Response bytes copied by `anicraft.response` | Plugin guest | Written into guest-owned memory supplied by the SDK; guest owns and releases it normally. |

Rules:

- The host must not write outside the allocation returned by `ac_alloc`.
- The host must pass the original allocation size to `ac_dealloc`, not a shorter used length.
- Every allocation may be deallocated exactly once. Duplicate frees, fabricated pointers, or mismatched lengths are ABI violations.
- The host must not retain guest pointers after an export or host import returns.
- The host must copy exporter output before deallocation.
- If `ac_dealloc` is absent, the host must destroy the module instance after the operation to reclaim its memory. It must not repeatedly invoke the module and leak allocations.
- `ac_manifest` memory is borrowed static memory and is never passed to `ac_dealloc`.

## 7. Reentrancy, concurrency, and execution

ABI v1 is synchronous and single-flight per module instance.

- The host must not invoke another plugin export on the same instance while an export is active.
- A host import must not synchronously or asynchronously re-enter the same plugin instance.
- The host must not call `anicraft.response` except for the response belonging to the immediately preceding `anicraft.call`.
- The host must not concurrently invoke exports on one instance or share its linear memory between concurrent invocations.
- A plugin may make sequential host calls during one command. Each call/response pair must complete before the next call begins.
- No ABI v1 guarantee is made for threads, shared memory, asynchronous imports, callbacks, or retained pointers.

The host should enforce an execution timeout or fuel budget. A timed-out or interrupted instance must be considered unusable and destroyed.

## 8. Panic and trap behavior

A plugin must not unwind a Rust panic across an exported C ABI boundary. ABI v1 treats an uncaught panic, WebAssembly trap, out-of-fuel termination, or memory trap as a failed invocation.

- The host must catch the WebAssembly trap at the instance boundary, report a plugin-invocation failure, and discard the affected instance.
- The host must not assume that plugin state, pending response state, or allocations remain valid after a trap.
- A trap is not converted into an `ac_run` status code; it terminates the call before a status can be returned.
- Plugins should return ordinary validation and operational failures as `Err(String)`, not panic.
- ABI v1 does not promise panic recovery or continuation after a panic.

## 9. Deterministic status and failure codes

`ac_run` returns these stable values:

| Code | Name | Meaning |
|---:|---|---|
| `0` | `STATUS_OK` | Handler completed successfully. |
| `1` | `STATUS_INVALID_ARGUMENT` | Negative length, or a null/non-positive pointer with a non-empty input. |
| `2` | `STATUS_INVALID_JSON` | Input is not valid UTF-8 JSON. |
| `3` | `STATUS_HANDLER_ERROR` | Plugin handler returned `Err`. |
| `4` | `STATUS_PAYLOAD_TOO_LARGE` | Input exceeds 16 MiB. |

Additional conventions:

- `ac_alloc` returns `-1` for invalid size or allocation failure.
- `ac_manifest` returns `-1` if the manifest exceeds the payload limit.
- `ac_export` returns `-1` for invalid input, invalid JSON, handler failure, oversized output, or allocation failure; otherwise it returns a packed pointer/length pair.
- `anicraft.call` and `anicraft.response` return a negative value for transport failure.
- A WebAssembly trap is reported by the host as an invocation failure, not as one of the `ac_run` codes.

Human-readable logs and host error strings are diagnostic only. They must not be parsed as machine-readable status codes.

## 10. Pointer/length packing and endianness

The ABI is a WebAssembly 32-bit ABI:

- Linear-memory pointers and lengths are 32-bit values.
- Exported pointer/length pairs use an `i64` numeric bit pattern: `(length << 32) | pointer`.
- The low 32 bits contain the pointer; the high 32 bits contain the length.
- Hosts must use unsigned extraction/masking when unpacking the `i64`; the signedness of the Wasm `i64` return does not change its bits.
- Wasm scalar values are passed according to the WebAssembly core calling convention. When scalar values are represented as bytes in linear memory, WebAssembly is little-endian.
- JSON payloads are UTF-8 text and therefore have no integer byte order.
- Exporter output is opaque bytes; its own file format defines its byte order.

Do not serialize Rust structs, enums, pointers, or native memory layouts directly across this ABI. Use the documented scalar signatures and JSON DTOs.

## 11. Conformance checklist

A conforming host/runtime must:

- [ ] Check `ac_abi_version() == 1` before other calls.
- [ ] Validate the module kind and required exports.
- [ ] Reject unexpected imports and validate required `anicraft` imports.
- [ ] Bounds-check all guest-memory accesses and enforce the 16 MiB payload limit.
- [ ] Copy borrowed guest bytes before returning from host imports.
- [ ] Implement the synchronous `call` then `response` protocol exactly.
- [ ] Never re-enter or concurrently invoke one module instance.
- [ ] Call optional `ac_dealloc` with the exact original allocation length when available, or destroy the instance after use.
- [ ] Copy exporter output before deallocation.
- [ ] Treat traps as failed invocations and discard the instance.
- [ ] Unpack pointer/length pairs using the specified low-pointer/high-length layout.

A conforming plugin/SDK must:

- [ ] Export the required entry points for its plugin kind and report ABI version 1.
- [ ] Use the SDK for host imports, serialization, allocation, and exports.
- [ ] Reject oversized payloads and invalid JSON.
- [ ] Return documented `ac_run` status codes.
- [ ] Never unwind across the ABI boundary or retain host pointers.
- [ ] Use UTF-8 JSON DTOs rather than native struct layouts.
