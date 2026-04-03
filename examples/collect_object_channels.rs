use std::{
    collections::HashSet,
    fs::File,
    io::{BufRead, BufReader, Cursor, Seek, SeekFrom},
};

use anyhow::Context;
use deimos_data::{
    hash::fnv1,
    pattern::{SComponent, SObjectChannelComponent},
    tfx::features::dynamic::SDynamicModelComponent,
};
use tiger_parse::{PackageManagerExt, TigerReadable};
use tiger_pkg::package_manager;

fn main() -> anyhow::Result<()> {
    deimos_core::initialize_package_manager(None)?;

    println!("Reading object channels...");
    let mut hashes: HashSet<u32> = HashSet::new();
    for (hash, _) in package_manager().get_all_by_reference(SComponent::ID.unwrap()) {
        let component: SComponent = package_manager()
            .read_tag_struct(hash)
            .context("Failed to read/parse tag")?;

        if component.definition.resource_type != 0x8080AF75 {
            continue;
        }

        let mut cur = Cursor::new(package_manager().read_tag(hash)?);
        cur.seek(SeekFrom::Start(component.definition.offset))?;
        let model: SObjectChannelComponent = TigerReadable::read_ds(&mut cur)?;
        for channel in model.m_channels {
            hashes.insert(channel.name);
        }
    }

    let mut hashes: Vec<u32> = hashes.into_iter().collect();
    hashes.sort_unstable();
    let mut wordlist = BufReader::new(File::open("wordlist.txt")?);
    for line in wordlist.lines() {
        let line = line?;
        let hash = fnv1(line.as_bytes());
        if hashes.binary_search(&hash).is_ok() {
            println!("0x{:08X} => {}", hash, line);
        }
    }

    // for hash in hashes {
    //     println!("0x{:08X} => {}", hash, find_hash(hash));
    // }

    Ok(())
}
