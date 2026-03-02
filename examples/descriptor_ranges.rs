use anyhow::Context;
use deimos_data::tfx::{STechnique, scope::SScope};
use deimos_render::tfx::expression_vm::{
    self, decompiler::DecompilerState, opcodes::OpcodeIterator,
};
use itertools::Itertools;
use tiger_parse::{PackageManagerExt, TigerReadable};
use tiger_pkg::package_manager;

fn main() -> anyhow::Result<()> {
    let Some(package_dir) = std::env::args().nth(1) else {
        anyhow::bail!("Usage: scope_decompile <package dir> <scope tag>");
    };

    deimos_core::initialize_package_manager(Some(package_dir.as_str()))?;

    for (hash, _) in package_manager().get_all_by_reference(SScope::ID.unwrap()) {
        let scope: SScope = package_manager()
            .read_tag_struct(hash)
            .context("Failed to read/parse tag")?;
        println!("=== Scope {} ===", scope.name.0);

        for (scope, stage) in scope.iter_stages() {
            if scope.core.bytecode.is_empty() {
                continue;
            }

            println!("// Stage: {stage:?}");
            // println!("\t// Disassembly:");
            // match expression_vm::disassemble(&scope.constants.bytecode) {
            //     Ok(lines) => {
            //         for line in lines {
            //             println!("\t{line}");
            //         }
            //     }
            //     Err(e) => {
            //         println!("\t// Failed to disassemble expression: {e}");
            //         continue;
            //     }
            // }

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
    }

    let mut usages: Vec<(u32, u32)> = vec![(0, 0); 64];

    for (hash, _) in package_manager().get_all_by_reference(STechnique::ID.unwrap()) {
        let technique: STechnique = package_manager()
            .read_tag_struct(hash)
            .context("Failed to read/parse tag")?;
        // println!("=== Technique {hash} ===");

        for (_stage, shader) in technique.all_valid_shaders() {
            if shader.core.bytecode.is_empty() {
                continue;
            }

            let mut op_iter = OpcodeIterator::new(&shader.core.bytecode);
            while let Some(Ok((op, ptr))) = op_iter.next() {
                if op == expression_vm::opcodes::Opcode::PopTextureView {
                    let slot = ptr[0] & 0x1F;
                    if let Some((_, usage_dynamic)) = usages.get_mut(slot as usize) {
                        *usage_dynamic += 1;
                    }
                }
            }

            for tex in &shader.core.textures {
                if tex.slot == 30 {
                    println!("Big boy? {}", hash);
                }
                if let Some((usage_static, _)) = usages.get_mut(tex.slot as usize) {
                    *usage_static += 1;
                } else {
                    println!("Big texture slot {}?", tex.slot);
                }
            }
        }
    }

    for (slot, (usage_static, usage_dynamic)) in usages.into_iter().enumerate() {
        if usage_static > 0 || usage_dynamic > 0 {
            println!("Slot {slot}: Static {usage_static}, Dynamic {usage_dynamic}");
        }
    }

    Ok(())
}
