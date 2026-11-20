#![no_main]
use libfuzzer_sys::fuzz_target;
use serde_json::Value;

fuzz_target!(|data: &[u8]| {
    // Match ABI v1 plugin-parameter cap before invoking the JSON parser.
    if data.len() > 64 * 1024 { return; }
    if let Ok(value) = serde_json::from_slice::<Value>(data) {
        // Exercise nested object/array traversal without trusting host-supplied lengths.
        let mut stack = vec![&value];
        let mut visited = 0usize;
        while let Some(item) = stack.pop() {
            visited += 1;
            if visited > 4096 { break; }
            match item {
                Value::Array(values) => stack.extend(values.iter().take(256)),
                Value::Object(values) => stack.extend(values.values().take(256)),
                _ => {}
            }
        }
    }
});
