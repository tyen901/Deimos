use std::str::FromStr;

use anyhow::Context;
use deimos_data::tfx::scope::SScope;
use deimos_render::tfx::expression_vm::{self, decompiler::DecompilerState};
use itertools::Itertools;
use tiger_parse::PackageManagerExt;
use tiger_pkg::{TagHash, package_manager};

fn main() -> anyhow::Result<()> {
    let Some(package_dir) = std::env::args().nth(1) else {
        anyhow::bail!("Usage: scope_decompile <package dir> <scope tag>");
    };

    let Some(hash) = std::env::args().nth(2) else {
        anyhow::bail!("Usage: scope_decompile <package dir> <scope tag>");
    };

    let Ok(hash) = TagHash::from_str(&hash) else {
        anyhow::bail!("Invalid scope tag hash: {}", hash);
    };

    deimos_core::initialize_package_manager(Some(package_dir.as_str()))?;

    let scope: SScope = package_manager()
        .read_tag_struct(hash)
        .context("Failed to read/parse tag")?;

    for (scope, stage) in scope.iter_stages() {
        println!("// Stage: {stage:?}");
        println!("\t// Disassembly:");
        match expression_vm::disassemble(&scope.core.bytecode) {
            Ok(lines) => {
                for line in lines {
                    println!("\t{line}");
                }
            }
            Err(e) => {
                println!("\t// Failed to disassemble expression: {e}");
                continue;
            }
        }

        println!("\t// Decompiled assignments:");
        match DecompilerState::new(&scope.core.bytecode)
            .with_ansi(true)
            .evaluate(&scope.core.bytecode_constants)
        {
            Ok(o) => {
                println!("\t{}", o.pretty_print().split("\n").join("\n\t"));
            }
            Err(e) => println!("\t// Failed to decompile expression: {e}"),
        };
    }

    Ok(())
}
