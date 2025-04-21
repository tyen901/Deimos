fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    embed_resource::compile("assets/res.rc", embed_resource::NONE)
        .manifest_required()
        .expect("Failed to compile resource file");
}
