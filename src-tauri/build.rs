fn main() {
    // Expose the target triple to the crate so sidecar.rs can build the
    // correct binary name without appending it at runtime.
    let target = std::env::var("TARGET").unwrap();
    println!("cargo:rustc-env=TARGET_TRIPLE={target}");

    tauri_build::build()
}
