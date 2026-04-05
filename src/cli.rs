use clap::Parser;

use crate::ui::colors::MARATHON_GREEN;

#[derive(Parser, Debug, Clone)]
#[command(author, version, about, long_about = None, disable_version_flag(true))]
pub struct AppArgs {
    /// Game directory
    #[arg(short, long)]
    pub gamedir: Option<String>,

    /// What display the window should be on
    #[arg(long)]
    pub display: Option<usize>,

    #[arg(long)]
    pub open_map: Option<String>,

    #[arg(long)]
    pub open_pattern: Option<String>,
}

pub const DEIMOS_VERSION: &str = env!("CARGO_PKG_VERSION");

pub const BANNER: &str = r#"
:::::::::  :::::::::: :::::::::::   :::   :::    ::::::::   ::::::::
:+:    :+: :+:            :+:      :+:+: :+:+:  :+:    :+: :+:    :+:
+:+    +:+ +:+            +:+     +:+ +:+:+ +:+ +:+    +:+ +:+
+#+    +:+ +#++:++#       +#+     +#+  +:+  +#+ +#+    +:+ +#++:++#++
+#+    +#+ +#+            +#+     +#+       +#+ +#+    +#+        +#+
#+#    #+# #+#            #+#     #+#       #+# #+#    #+# #+#    #+#
#########  ########## ########### ###       ###  ########   ########
"#;

pub const QUOTE: &str = "Escape will make me GOD";

pub fn print_banner() {
    for line in BANNER.lines() {
        println!(
            "\x1b[38;2;{};{};{}m{}",
            MARATHON_GREEN.r(),
            MARATHON_GREEN.g(),
            MARATHON_GREEN.b(),
            line
        );
    }
    println!();
    println!("                \x1b[4mv{DEIMOS_VERSION} - {QUOTE}\x1b[0m");
    println!();
}
