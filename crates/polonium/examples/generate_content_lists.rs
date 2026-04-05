use anyhow::Context;
use deimos_data::{
    activity::SActivity, map::SBubbleParent, pattern::SPattern, tfx::features::statics::SStaticMesh,
};
use deimos_polonium::tag_rehash;
use tiger_parse::TigerReadable;
use tiger_pkg::{TagHash, package_manager};

fn main() -> anyhow::Result<()> {
    deimos_core::initialize_package_manager(None)?;

    let mut hashes = vec![];
    for ty in [
        SActivity::ID.unwrap(),
        SBubbleParent::ID.unwrap(),
        SPattern::ID.unwrap(),
        SStaticMesh::ID.unwrap(),
    ] {
        hashes.extend(collect_hashes_for_ref(ty));
    }

    let hashes_raw = hashes
        .into_iter()
        .map(|h| tag_rehash(h).0)
        .collect::<Vec<_>>();
    std::fs::write(
        "crates/polonium/content_lists.bin",
        bytemuck::cast_slice(&hashes_raw),
    )
    .context("failed to write content_lists.bin")?;

    Ok(())
}

fn collect_hashes_for_ref(reference: u32) -> Vec<TagHash> {
    package_manager()
        .get_all_by_reference(reference)
        .into_iter()
        .map(|(tag, _)| tag)
        .collect()
}
