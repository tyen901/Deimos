fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    embed_resource::compile("assets/res.rc", embed_resource::NONE)
        .manifest_required()
        .expect("Failed to compile resource file");

    // Include lib folder in the search path
    println!("cargo:rustc-link-search=lib");

    // Copy lib/SDL3.dll to OUT_DIR
    let out_dir = if cfg!(target_os = "windows") {
        std::path::Path::new("target").join(std::env::var("PROFILE").expect("PROFILE not set"))
    } else {
        std::path::Path::new("target")
            .join("x86_64-pc-windows-msvc")
            .join(std::env::var("PROFILE").expect("PROFILE not set"))
    };
    let sdl3_dll = std::path::Path::new("lib/SDL3.dll");
    std::fs::copy(sdl3_dll, out_dir.join("SDL3.dll")).expect("Failed to copy SDL3.dll");
}
