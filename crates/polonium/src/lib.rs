use std::{collections::HashSet, sync::LazyLock};

use tiger_pkg::TagHash;

const TAG_KEY: u32 = 0x7734DEAD;

static CONTENT_LISTS: LazyLock<HashSet<TagHash>> = LazyLock::new(|| {
    let bytes = include_bytes!("../content_lists.bin");
    let hashes_raw: &[u32] = bytemuck::cast_slice(bytes);
    hashes_raw.iter().copied().map(TagHash).collect()
});

pub fn check_tag(tag: TagHash) -> bool {
    CONTENT_LISTS.contains(&tag_rehash(tag))
}

pub const fn tag_rehash(tag: TagHash) -> TagHash {
    TagHash(tag.0 ^ TAG_KEY)
}
