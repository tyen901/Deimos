use rayon::iter::{IntoParallelIterator, ParallelIterator};
use tiger_pkg::package_manager;

fn main() -> anyhow::Result<()> {
    deimos_core::initialize_package_manager(std::env::args().nth(1).as_deref())?;

    package_manager()
        .get_all_by_type(27, Some(1))
        .into_par_iter()
        .for_each(|(tag, _)| {
            let data = package_manager().read_tag(tag).expect("Failed to read USM");
            if data[0..4] != *b"CRID" {
                println!("Skipping non-USM tag {tag}");
                return;
            }

            // Find the first occurrence of ".mov" in the data, which should be the end of the original filename

            let mov_pos = data
                .windows(4)
                .position(|w| w == b".usm")
                .expect("Failed to find .usm in USM data");
            let null_pos = data[..mov_pos]
                .iter()
                .rposition(|&x| x == 0)
                .expect("Failed to find null terminator before .usm in USM data");
            let filename = std::str::from_utf8(&data[null_pos + 1..mov_pos + 4])
                .expect("Failed to parse filename in USM data");

            std::fs::write(format!("video/{filename}"), data)
                .expect("Failed to write USM data to file");
        });

    Ok(())
}
