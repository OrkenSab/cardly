// Copyright: Cardly contributors
// License: GNU AGPL, version 3 or later; https://www.gnu.org/licenses/agpl-3.0.html

use std::env;
use std::fs;
use std::path::PathBuf;

fn main() {
    let descriptors = env::var("DESCRIPTORS_BIN")
        .map(PathBuf::from)
        .unwrap_or_else(|_| {
            let upstream = PathBuf::from("../anki-upstream/out/rslib/proto/descriptors.bin");
            if upstream.exists() {
                upstream
            } else {
                PathBuf::from("descriptors.bin")
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
