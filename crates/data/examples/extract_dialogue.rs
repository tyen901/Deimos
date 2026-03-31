use std::io::{Cursor, Seek};

use deimos_data::investment::dialogue::S8080AA3B;
use tiger_parse::TigerReadable;
use tiger_pkg::package_manager;

fn main() -> anyhow::Result<()> {
    deimos_core::initialize_package_manager(None)?;
    let data = package_manager().read_tag(0x80A4FA6E)?;
    let mut cur = Cursor::new(data);
    cur.seek(std::io::SeekFrom::Start(0x4E0))?;
    let components: Vec<S8080AA3B> = <_ as TigerReadable>::read_ds(&mut cur)?;
    for component in components {
        println!("{:#?}", *component.pointer);
    }

    Ok(())
}
