use std::path::PathBuf;

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=ffi/wrapper.h");

    check_sane();
    bindgen_sane();
}

fn check_sane() {
    // TODO: How to check sane version at compile time,
    // when pkg-config file does not exists?
}

fn bindgen_sane() {
    println!("cargo:rustc-link-lib=dylib=sane");

    let out_dir = std::env::var("OUT_DIR").unwrap();
    let bindings_path = PathBuf::from(out_dir).join("bindings.rs");

    bindgen::builder()
        .header("ffi/wrapper.h")
        .generate()
        .expect("Failed to generate bindings")
        .write_to_file(bindings_path)
        .expect("Failed to write bindings");
}
