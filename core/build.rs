fn main() {
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR");
    let udl = format!("{manifest_dir}/src/ffi/tamtri.udl");
    uniffi::generate_scaffolding_for_crate(udl, "tamtri_core")
        .expect("generate UniFFI scaffolding");
}
