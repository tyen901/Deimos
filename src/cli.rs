
use clap::Parser;

#[derive(Parser, Debug, Clone)]
#[command(author, version, about, long_about = None, disable_version_flag(true))]
pub struct AppArgs {
    /// Game directory
    #[arg(short, long)]
    pub gamedir: Option<String>,
}
