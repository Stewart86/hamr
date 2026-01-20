#![no_main]

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    // Try to parse arbitrary bytes as JSON for ResultItem
    // This should never panic - only return Ok or Err
    if let Ok(s) = std::str::from_utf8(data) {
        let _ = serde_json::from_str::<hamr_types::ResultItem>(s);
    }
});
