use std::{
    io::{Cursor, Seek},
    path::PathBuf,
    str::FromStr,
};

use anyhow::Context;
use deimos_data::{tag::WideHash, tfx::STechnique, wwise::SWwiseEvent};
use deimos_render::tfx::expression_vm::{self, decompiler::DecompilerState};
use itertools::Itertools;
use tiger_parse::{PackageManagerExt, TigerReadable};
use tiger_pkg::{TagHash, package, package_manager};

fn main() -> anyhow::Result<()> {
    let Some(hash) = std::env::args().nth(1) else {
        anyhow::bail!("Usage: dump_sound_container <package dir> <sound container tag>");
    };

    let Ok(hash) = TagHash::from_str(&hash) else {
        anyhow::bail!("Invalid sound container tag hash: {}", hash);
    };

    deimos_core::initialize_package_manager(None)?;

    let container = package_manager()
        .read_tag(hash)
        .context("Failed to read tag")?;

    let mut cur = Cursor::new(container);
    while let Ok(wwise_event_tag) = WideHash::read_ds(&mut cur) {
        if let Ok(wwise_event) = package_manager().read_tag_struct::<SWwiseEvent>(wwise_event_tag) {
            println!("Dumping Wwise event: {wwise_event_tag}");
            for stream in wwise_event.wwise_streams {
                let stream_data = package_manager().read_tag(stream)?;
                std::fs::write(
                    PathBuf::from_str("stream")?.join(format!(
                        "{}_{}.wem",
                        wwise_event_tag.hash32(),
                        stream
                    )),
                    stream_data,
                )?;
            }
        }

        cur.seek(std::io::SeekFrom::Current(-0xC))?;
    }

    Ok(())
}
