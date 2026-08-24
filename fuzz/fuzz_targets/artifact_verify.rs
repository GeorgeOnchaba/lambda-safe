#![no_main]

use lambda_safe_emit::verify_artifacts;
use lambda_safe_model::{HardManifest, LambdaCertificate};
use lambda_safe_syntax::deserialize_json_strict;
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let midpoint = data.len() / 2;
    let (manifest_bytes, certificate_bytes) = data.split_at(midpoint);
    let (Ok(manifest_source), Ok(certificate_source)) = (
        std::str::from_utf8(manifest_bytes),
        std::str::from_utf8(certificate_bytes),
    ) else {
        return;
    };
    let (Ok(manifest), Ok(certificate)) = (
        deserialize_json_strict::<HardManifest>(manifest_source),
        deserialize_json_strict::<LambdaCertificate>(certificate_source),
    ) else {
        return;
    };
    let _ = verify_artifacts(&manifest, &certificate);
});
