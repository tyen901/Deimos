use std::{collections::HashSet, path::PathBuf, sync::Arc};

use anyhow::Context;
use deimos_core::MARATHON_APP_ID;
use deimos_data::tfx::{scope::SScope, ExternIndex, SDynamicConstants, STechnique};
use deimos_render::tfx::expression_vm::opcodes::{pascal_to_snake, Opcode, OpcodeIterator};
use itertools::Itertools;
use tiger_parse::{PackageManagerExt, TigerReadable};
use tiger_pkg::{package_manager, PackageManager};

fn main() -> anyhow::Result<()> {
    let game_path = if let Some(path) = std::env::args().nth(1) {
        path.clone()
    } else {
        let Some(steamapp) = game_detector::steam::get_all_apps()
            .context("Failed to enumerate Steam apps")?
            .into_iter()
            .find(|a| a.appid == MARATHON_APP_ID)
        else {
            eprintln!("Failed to find Marathon app in Steam library. If you don't have Marathon installed through Steam, then you can specify the path to the game directory using the --gamedir/-g argument.");
            return Ok(());
        };

        steamapp.game_path
    };

    let pm = Arc::new(
        PackageManager::new(
            PathBuf::from(&game_path).join("packages"),
            tiger_pkg::GameVersion::Marathon(tiger_pkg::MarathonVersion::MarathonAlpha),
            None,
        )
        .context("Failed to initialize package manager")?,
    );
    tiger_pkg::initialize_package_manager(&pm);

    #[derive(Copy, Clone, Debug, Hash, PartialEq, Eq)]
    pub enum ExternFieldType {
        Float,
        Vec4,
        Mat4,
        U32,
        Texture,
        Uav,
    }

    let mut fields: HashSet<(ExternIndex, ExternFieldType, usize)> = Default::default();

    let mut process_constants = |dc: &SDynamicConstants| {
        for op in OpcodeIterator::new(&dc.bytecode) {
            let (op, args) = match op {
                Ok(o) => o,
                Err(e) => {
                    eprintln!(
                        "Failed to parse opcode in dynamic constants, fields may be missing: {e} ({:02X?})",
                        dc.bytecode
                    );
                    break;
                }
            };

            if !matches!(
                op,
                Opcode::PushExternInputFloat
                    | Opcode::PushExternInputVec4
                    | Opcode::PushExternInputMat4
                    | Opcode::PushExternInputTextureView
                    | Opcode::PushExternInputU32
                    | Opcode::PushExternInputUav
            ) {
                continue;
            }

            let extern_ = ExternIndex::try_from(args[0]).unwrap();
            let offset_raw = args[1];

            match op {
                Opcode::PushExternInputFloat => {
                    fields.insert((extern_, ExternFieldType::Float, offset_raw as usize * 4));
                }
                Opcode::PushExternInputVec4 => {
                    fields.insert((extern_, ExternFieldType::Vec4, offset_raw as usize * 16));
                }
                Opcode::PushExternInputMat4 => {
                    fields.insert((extern_, ExternFieldType::Mat4, offset_raw as usize * 16));
                }
                Opcode::PushExternInputTextureView => {
                    fields.insert((extern_, ExternFieldType::Texture, offset_raw as usize * 8));
                }
                Opcode::PushExternInputU32 => {
                    fields.insert((extern_, ExternFieldType::U32, offset_raw as usize * 4));
                }
                Opcode::PushExternInputUav => {
                    fields.insert((extern_, ExternFieldType::Uav, offset_raw as usize * 8));
                }
                _ => {}
            }
        }
    };

    for (t, _) in package_manager()
        .get_all_by_reference(SScope::ID.unwrap())
        .into_iter()
    {
        let Ok(scope): tiger_parse::Result<SScope> = package_manager().read_tag_struct(t) else {
            continue;
        };
        for (s, _) in scope.iter_stages() {
            process_constants(&s.constants);
        }
    }

    for (t, _) in package_manager()
        .get_all_by_reference(STechnique::ID.unwrap())
        .into_iter()
    {
        let Ok(technique): tiger_parse::Result<STechnique> = package_manager().read_tag_struct(t)
        else {
            continue;
        };
        for (_, s) in technique.all_shaders() {
            process_constants(&s.constants);
        }
    }

    for ext in (0..ExternIndex::COUNT).map(|v| ExternIndex::try_from(v as u8).unwrap()) {
        let mut sfields = fields
            .iter()
            .filter(|(e, _, _)| *e == ext)
            .map(|(_, a, b)| (*a, *b))
            .collect_vec();

        sfields.sort_by_key(|(_, offset)| *offset);

        if sfields.is_empty() {
            continue;
        }

        println!("extern_struct! {{");
        println!(
            "\tstruct {ext:?}(\"{}\") {{",
            pascal_to_snake(&format!("{ext:?}"))
        );

        for (ty, offset) in sfields {
            let ty_str = match ty {
                ExternFieldType::Float => "f32",
                ExternFieldType::Vec4 => "Vec4",
                ExternFieldType::Mat4 => "Mat4",
                ExternFieldType::U32 => "u32",
                ExternFieldType::Texture => "TextureView",
                ExternFieldType::Uav => "UnorderedAccessView",
            };

            println!("\t\t0x{offset:02X} => unk{offset:02x}: {ty_str},");
        }

        println!("\t}}");
        println!("}}\n");
    }

    Ok(())
}
