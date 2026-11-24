// Copyright: Cardly contributors
// License: GNU AGPL, version 3 or later; https://www.gnu.org/licenses/agpl-3.0.html

use std::env;
use std::fs;
use std::path::PathBuf;

fn main() {
    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap_or_else(|_| ".".into()));
    let descriptors = env::var("DESCRIPTORS_BIN")
        .ok()
        .map(PathBuf::from)
        .filter(|p| p.exists())
        .unwrap_or_else(|| {
            let upstream = manifest_dir.join("../anki-upstream/out/rslib/proto/descriptors.bin");
            if upstream.exists() {
                upstream
            } else {
                manifest_dir.join("descriptors.bin")
            }
        });

    println!("cargo:rerun-if-env-changed=DESCRIPTORS_BIN");
    println!("cargo:rerun-if-changed={}", descriptors.display());

    let out_dir = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR not set"));
    let dest = out_dir.join("anki_descriptors.bin");

    let bytes = fs::read(&descriptors).unwrap_or_else(|e| {
        panic!(
            "Failed to read protobuf descriptors at {}: {e}. Build anki-upstream first or set DESCRIPTORS_BIN.",
            descriptors.display()
        )
    });
    fs::write(&dest, bytes).expect("Failed to copy descriptors.bin into OUT_DIR");
}
