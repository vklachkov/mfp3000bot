use std::path::PathBuf;

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=ffi/wrapper.h");

    check_cups();
    bindgen_cups();
}

fn check_cups() {
    // FIXME: cups.pc doesn't exists on Debian 11 that used in build image.
    // pkg_config::Config::new()
    //     .atleast_version("2.3.3")
    //     .probe("cups")
    //     .unwrap();
}

fn bindgen_cups() {
    println!("cargo:rustc-link-lib=dylib=cups");

    let out_dir = std::env::var("OUT_DIR").unwrap();
    let bindings_path = PathBuf::from(out_dir).join("bindings.rs");

    bindgen::builder()
        .header("ffi/wrapper.h")
        .generate()
        .expect("Failed to generate bindings")
        .write_to_file(&bindings_path)
        .expect("Failed to write bindings");

    // Bindgen generates invalid rust file with duplicates.
    // See issue https://github.com/rust-lang/rust-bindgen/issues/1848.
    std::process::Command::new("sed")
        .args([
            "-i",
            "/pub const IPPORT_RESERVED: u32 = 1024;/d",
            bindings_path
                .to_str()
                .expect("Failed to convert bindings path to UTF-8 string"),
        ])
        .output()
        .expect("Failed to remove duplicate in bindings");
}
