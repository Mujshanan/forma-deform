#![allow(unsafe_code)] // Raw pointers and host imports are isolated in this audited ABI boundary.

//! Guest-side helpers for AniCraft WebAssembly plug-ins (ABI v1).
//!
//! The normative contract is docs/PLUGIN_ABI_V1.md. Keep this crate as the only guest-side
//! implementation of the ABI: plug-ins should not declare host imports or raw ABI exports.
//! The host imports are synchronous. A plug-in instance must not be re-entered while an export
//! or host call is active.
//!
//! Native builds provide inert host-import stand-ins so pure plug-in logic can be unit-tested.

use serde_json::{Value, json};

/// AniCraft host default limits. Per-kind limits may be lower (see PLUGIN_ABI_V1.md).
pub const MAX_MODULE_BYTES: usize = 16 * 1024 * 1024;
pub const MAX_LINEAR_MEMORY_BYTES: usize = 256 * 1024 * 1024;
pub const MAX_MANIFEST_BYTES: usize = 64 * 1024;
pub const MAX_PLUGIN_PARAMS_BYTES: usize = 64 * 1024;
pub const MAX_HOST_REQUEST_BYTES: usize = 1 * 1024 * 1024;
pub const MAX_HOST_RESPONSE_BYTES: usize = 32 * 1024 * 1024;
pub const MAX_EXPORT_OUTPUT_BYTES: usize = 64 * 1024 * 1024;
/// Stable status returned by command exports.
pub const STATUS_OK: i32 = 0;
pub const STATUS_INVALID_ARGUMENT: i32 = 1;
pub const STATUS_INVALID_JSON: i32 = 2;
pub const STATUS_HANDLER_ERROR: i32 = 3;
pub const STATUS_PAYLOAD_TOO_LARGE: i32 = 4;
pub const STATUS_ALLOCATION_FAILED: i32 = 5;

#[cfg(target_arch = "wasm32")]
#[link(wasm_import_module = "anicraft")]
unsafe extern "C" {
    /// Synchronously submits UTF-8 JSON bytes. Returns response byte length, or a negative
    /// value when the host transport failed.
    #[link_name = "call"]
    fn host_call(ptr: *const u8, len: usize) -> i32;
    /// Copies the response from the immediately preceding call. Returns copied byte count,
    /// or a negative value on transport failure.
    #[link_name = "response"]
    fn host_response(ptr: *mut u8, cap: usize) -> i32;
}

#[cfg(not(target_arch = "wasm32"))]
unsafe fn host_call(_: *const u8, _: usize) -> i32 { -1 }
#[cfg(not(target_arch = "wasm32"))]
unsafe fn host_response(_: *mut u8, _: usize) -> i32 { -1 }

/// Sends one request and returns its result or a diagnostic string.
///
/// Calls are synchronous and single-flight. The host must copy request bytes before returning
/// from `anicraft.call`; the response remains available only until the next host call.
pub fn request(req: &Value) -> Result<Value, String> {
    let bytes = serde_json::to_vec(req).map_err(|e| format!("request serialization failed: {e}"))?;
    if bytes.len() > MAX_HOST_REQUEST_BYTES {
        return Err(format!("request exceeds ABI v1 host request limit ({MAX_HOST_REQUEST_BYTES} bytes)"));
    }

    // SAFETY: bytes remains alive and immutable for the full synchronous host call.
    let response_len = unsafe { host_call(bytes.as_ptr(), bytes.len()) };
    if response_len < 0 {
        return Err("AniCraft host call failed".into());
    }
    let response_len = response_len as usize;
    if response_len > MAX_HOST_RESPONSE_BYTES {
        return Err("AniCraft response exceeds ABI v1 host response limit".into());
    }

    let mut response = vec![0u8; response_len];
    // SAFETY: the host may write at most response.len() bytes into this live allocation.
    let copied = unsafe { host_response(response.as_mut_ptr(), response.len()) };
    if copied < 0 || copied as usize > response.len() {
        return Err("AniCraft host response copy failed or exceeded its capacity".into());
    }
    response.truncate(copied as usize);

    let value: Value = serde_json::from_slice(&response)
        .map_err(|e| format!("AniCraft returned invalid UTF-8 JSON: {e}"))?;
    if value["ok"] == true {
        Ok(value["result"].clone())
    } else {
        Err(value["error"].as_str().unwrap_or("the host refused the request").to_owned())
    }
}

/// Runs an AniCraft command by id.
pub fn command(id: &str, params: Value) -> Result<Value, String> {
    request(&json!({"op": "command", "id": id, "params": params}))
        .map_err(|e| format!("{id}: {e}"))
}

/// The compact document view: stage, scene, frame, layers and selection.
pub fn document() -> Result<Value, String> {
    request(&json!({"op": "document"}))
}

/// Selected elements on the active layer at the playhead.
pub fn selected_elements(paths: bool) -> Result<Vec<Value>, String> {
    let value = request(&json!({"op": "elements", "selected": true, "paths": paths}))?;
    Ok(value["elements"].as_array().cloned().unwrap_or_default())
}

/// A line in the run report.
pub fn log(message: &str) {
    let _ = request(&json!({"op": "log", "message": message}));
}

/// An exporter's movie metadata.
pub fn movie() -> Result<Value, String> {
    request(&json!({"op": "movie"}))
}

/// Evaluated export frame.
pub fn frame(index: u64) -> Result<Value, String> {
    request(&json!({"op": "frame", "index": index}))
}

/// Deterministic SplitMix64 generator. Plug-ins have no ambient randomness source.
pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Rng { Rng(seed ^ 0x9e37_79b9_7f4a_7c15) }
    pub fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9e37_79b9_7f4a_7c15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
        z ^ (z >> 31)
    }
    pub fn unit(&mut self) -> f64 { (self.next_u64() >> 11) as f64 / (1u64 << 53) as f64 }
    pub fn signed(&mut self) -> f64 { self.unit() * 2.0 - 1.0 }
}

/// Exports ABI v1 command entry points. Handler signature:
/// `fn(&serde_json::Value) -> Result<(), String>`.
#[macro_export]
macro_rules! plugin {
    ($manifest:expr, $run:path) => {
        #[unsafe(no_mangle)]
        pub extern "C" fn ac_abi_version() -> i32 { 1 }
        #[unsafe(no_mangle)]
        pub extern "C" fn ac_manifest() -> i64 { $crate::manifest_ptr($manifest) }
        #[unsafe(no_mangle)]
        pub extern "C" fn ac_alloc(size: i32) -> i32 { $crate::alloc(size) }
        #[unsafe(no_mangle)]
        pub extern "C" fn ac_dealloc(ptr: i32, len: i32) { $crate::dealloc(ptr, len) }
        #[unsafe(no_mangle)]
        pub extern "C" fn ac_run(ptr: i32, len: i32) -> i32 { $crate::run(ptr, len, $run) }
    };
}

/// Exports ABI v1 exporter entry points. Handler signature:
/// `fn(&serde_json::Value) -> Result<Vec<u8>, String>`.
#[macro_export]
macro_rules! exporter {
    ($manifest:expr, $export:path) => {
        #[unsafe(no_mangle)]
        pub extern "C" fn ac_abi_version() -> i32 { 1 }
        #[unsafe(no_mangle)]
        pub extern "C" fn ac_manifest() -> i64 { $crate::manifest_ptr($manifest) }
        #[unsafe(no_mangle)]
        pub extern "C" fn ac_alloc(size: i32) -> i32 { $crate::alloc(size) }
        #[unsafe(no_mangle)]
        pub extern "C" fn ac_dealloc(ptr: i32, len: i32) { $crate::dealloc(ptr, len) }
        #[unsafe(no_mangle)]
        pub extern "C" fn ac_export(ptr: i32, len: i32) -> i64 {
            $crate::export(ptr, len, $export)
        }
    };
}

/// Exports ABI v1 ease entry points. Ease plug-ins do not import the AniCraft host API.
#[macro_export]
macro_rules! ease {
    ($manifest:expr, $ease:path) => {
        #[unsafe(no_mangle)]
        pub extern "C" fn ac_abi_version() -> i32 { 1 }
        #[unsafe(no_mangle)]
        pub extern "C" fn ac_manifest() -> i64 { $crate::manifest_ptr($manifest) }
        #[unsafe(no_mangle)]
        pub extern "C" fn ac_ease(index: i32, t: f64) -> f64 {
            $ease(index.max(0) as usize, t)
        }
    };
}

#[doc(hidden)]
pub fn manifest_ptr(manifest: &'static str) -> i64 {
    if manifest.len() > MAX_MANIFEST_BYTES { return -1; }
    pack_ptr_len(manifest.as_ptr() as usize, manifest.len())
}

fn pack_ptr_len(ptr: usize, len: usize) -> i64 {
    (((len as u64) << 32) | (ptr as u32 as u64)) as i64
}

#[doc(hidden)]
pub fn alloc(size: i32) -> i32 {
    if size < 0 || size as usize > MAX_PLUGIN_PARAMS_BYTES { return 0; }
    let mut bytes = Vec::<u8>::new();
    if bytes.try_reserve_exact(size as usize).is_err() { return 0; }
    bytes.resize(size as usize, 0);
    let raw = Box::into_raw(bytes.into_boxed_slice());
    let ptr = raw as *mut u8 as usize;
    if ptr == 0 || ptr > i32::MAX as usize {
        // SAFETY: raw is the allocation just created above and has not been transferred.
        unsafe { drop(Box::from_raw(raw)); }
        return 0;
    }
    ptr as i32
}

/// Releases exactly the slice previously returned by `ac_alloc` or `ac_export`.
/// The host must pass the original allocation length, not the number of bytes it used.
#[doc(hidden)]
pub fn dealloc(ptr: i32, len: i32) {
    if ptr <= 0 || len < 0 || len as usize > MAX_EXPORT_OUTPUT_BYTES { return; }
    // SAFETY: the ABI contract requires the host to return the original pointer and allocation
    // length exactly once. Invalid or duplicate deallocation is a host ABI violation.
    unsafe {
        let slice = std::ptr::slice_from_raw_parts_mut(ptr as usize as *mut u8, len as usize);
        drop(Box::from_raw(slice));
    }
}

#[doc(hidden)]
pub fn run(ptr: i32, len: i32, handler: fn(&Value) -> Result<(), String>) -> i32 {
    if len < 0 || (len > 0 && ptr <= 0) { return STATUS_INVALID_ARGUMENT; }
    if len as usize > MAX_PLUGIN_PARAMS_BYTES { return STATUS_PAYLOAD_TOO_LARGE; }
    let bytes: &[u8] = if len == 0 { &[] } else {
        // SAFETY: the host must pass a live guest allocation from ac_alloc with at least len bytes.
        unsafe { std::slice::from_raw_parts(ptr as usize as *const u8, len as usize) }
    };
    run_bytes(bytes, handler)
}

fn run_bytes(bytes: &[u8], handler: fn(&Value) -> Result<(), String>) -> i32 {
    if bytes.len() > MAX_PLUGIN_PARAMS_BYTES { return STATUS_PAYLOAD_TOO_LARGE; }
    let params: Value = match serde_json::from_slice(bytes) {
        Ok(value) => value,
        Err(_) => return STATUS_INVALID_JSON,
    };
    match handler(&params) {
        Ok(()) => STATUS_OK,
        Err(error) => {
            log(&error);
            STATUS_HANDLER_ERROR
        }
    }
}

#[doc(hidden)]
pub fn export(ptr: i32, len: i32, handler: fn(&Value) -> Result<Vec<u8>, String>) -> i64 {
    if len < 0 || len as usize > MAX_PLUGIN_PARAMS_BYTES || (len > 0 && ptr <= 0) { return -1; }
    let bytes: &[u8] = if len == 0 { &[] } else {
        // SAFETY: the host must pass a live guest allocation from ac_alloc with at least len bytes.
        unsafe { std::slice::from_raw_parts(ptr as usize as *const u8, len as usize) }
    };
    let options: Value = match serde_json::from_slice(bytes) {
        Ok(value) => value,
        Err(_) => return -1,
    };
    match handler(&options) {
        Ok(output) if output.len() <= MAX_EXPORT_OUTPUT_BYTES => {
            let boxed = output.into_boxed_slice();
            let len = boxed.len();
            let raw = Box::into_raw(boxed);
            let ptr = raw as *mut u8 as usize;
            if ptr == 0 || ptr > i32::MAX as usize {
                // SAFETY: raw is the output allocation just created above.
                unsafe { drop(Box::from_raw(raw)); }
                return -1;
            }
            pack_ptr_len(ptr, len)
        }
        Ok(_) => -1,
        Err(error) => {
            log(&error);
            -1
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn host_and_payload_limits_match_the_abi_contract() {
        assert_eq!(MAX_MODULE_BYTES, 16 * 1024 * 1024);
        assert_eq!(MAX_MANIFEST_BYTES, 64 * 1024);
        assert_eq!(MAX_PLUGIN_PARAMS_BYTES, 64 * 1024);
        assert_eq!(MAX_HOST_REQUEST_BYTES, 1 * 1024 * 1024);
        assert_eq!(MAX_HOST_RESPONSE_BYTES, 32 * 1024 * 1024);
        assert_eq!(MAX_EXPORT_OUTPUT_BYTES, 64 * 1024 * 1024);
    }

    #[test]
    fn packed_pointer_length_uses_low_pointer_high_length() {
        assert_eq!(pack_ptr_len(0x1234, 0x5678) as u64, 0x0000_5678_0000_1234);
    }

    #[test]
    fn allocation_rejects_negative_and_oversized_requests() {
        assert_eq!(alloc(-1), 0);
        assert_eq!(alloc((MAX_PLUGIN_PARAMS_BYTES + 1) as i32), 0);
    }

    #[test]
    fn run_returns_stable_invalid_input_and_json_codes() {
        assert_eq!(run(0, -1, |_| Ok(())), STATUS_INVALID_ARGUMENT);
        assert_eq!(run(0, 1, |_| Ok(())), STATUS_INVALID_ARGUMENT);
        assert_eq!(run_bytes(&[0xff], |_| Ok(())), STATUS_INVALID_JSON);
        assert_eq!(run(1, (MAX_PLUGIN_PARAMS_BYTES + 1) as i32, |_| Ok(())), STATUS_PAYLOAD_TOO_LARGE);
    }

    #[test]
    fn handler_error_has_stable_status_code() {
        assert_eq!(run_bytes(br#"{}"#, |_| Err("no".into())), STATUS_HANDLER_ERROR);
        assert_eq!(run_bytes(br#"{}"#, |_| Ok(())), STATUS_OK);
    }
}
