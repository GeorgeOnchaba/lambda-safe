#![no_main]

use lambda_safe_syntax::parse_json_projection;
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    if let Ok(source) = std::str::from_utf8(data) {
        let _ = parse_json_projection(source);
    }
});
