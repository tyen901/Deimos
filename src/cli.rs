use std::str::FromStr;

use clap::Parser;
use tiger_pkg::TagHash;

#[derive(Parser, Debug, Clone)]
#[command(author, version, about, long_about = None, disable_version_flag(true))]
pub struct AppArgs {
    /// Game directory
    #[arg(short, long)]
    pub gamedir: Option<String>,

    #[arg(short, long, value_parser = parse_taghash)]
    pub map: Option<TagHash>,
    // #[arg(long)]
    // pub fullscreen: bool,
}

pub fn parse_taghash(s: &str) -> Result<TagHash, String> {
    TagHash::from_str(s).map_err(|e| e.to_string())
}
