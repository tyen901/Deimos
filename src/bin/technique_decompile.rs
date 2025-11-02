use std::str::FromStr;

use anyhow::Context;
use deimos_data::tfx::STechnique;
use deimos_render::tfx::expression_vm::{self, decompiler::DecompilerState};
use tiger_parse::PackageManagerExt;
use tiger_pkg::{package_manager, TagHash};

fn main() -> anyhow::Result<()> {
    let Some(hash) = std::env::args().nth(1) else {
        anyhow::bail!("Usage: technique_decompile <package dir> <technique tag>");
    };

    let Ok(hash) = TagHash::from_str(&hash) else {
        anyhow::bail!("Invalid technique tag hash: {}", hash);
    };

    deimos_core::initialize_package_manager(None)?;

    let tech: STechnique = package_manager()
        .read_tag_struct(hash)
        .context("Failed to read/parse tag")?;

    for (stage, shader) in tech.all_valid_shaders() {
        println!("// Stage: {stage:?}");
        println!("\t// Disassembly:");
        match expression_vm::disassemble(&shader.constants.bytecode) {
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
        let mut outputs = vec![String::new(); 1024];
        match DecompilerState::new(&shader.constants.bytecode)
            .evaluate(&shader.constants.bytecode_constants, &mut outputs)
        {
            Ok(()) => {
                for (i, line) in outputs.iter().enumerate().filter(|(_i, s)| !s.is_empty()) {
                    println!("\tcb[{i}] = {line};");
                }
            }
            Err(e) => println!("\t// Failed to decompile expression: {e}"),
        };
    }

    Ok(())
}
