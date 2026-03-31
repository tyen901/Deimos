use std::fs::File;
use std::io::Write;

use anyhow::Context;
use deimos_data::investment::codex::UnkInvestmentComponent;
use deimos_investment::Investment;

fn main() -> anyhow::Result<()> {
    deimos_core::initialize_package_manager(None)?;

    let investment = Investment::load().context("failed to load investment data")?;
    for (entry_index, entry) in investment.globals.codex.table.iter().enumerate() {
        println!(
            "Codex entry #{entry_index} 0x{:X} | 0x{:X}",
            entry.unk0, entry.unk4
        );
        for (i, component) in entry.components.iter().enumerate() {
            print!("  {i} - ");
            match &**component {
                UnkInvestmentComponent::S808028E8(s) => {
                    println!(
                        "808028E8 0x{:X} 0x{:X} 0x{:X} 0x{:X} 0x{:X} | {:?} / {:?} | 0x{:X} | {:?} / {:?} | 0x{:X} | {:?} / {:?}",
                        s.unk0,
                        s.unk4,
                        s.unk8,
                        s.unkc,
                        s.unk10,
                        investment.get_string(s.unk14),
                        investment.get_string(s.unk1c),
                        s.unk24,
                        investment.get_string(s.unk28),
                        investment.get_string(s.unk30),
                        s.unk38,
                        investment.get_string(s.unk3c),
                        investment.get_string(s.unk44)
                    );
                }
                UnkInvestmentComponent::S808028E9(s) => {
                    println!(
                        "808028E9 icon={} unk4={:?} unkc={:?} unk14={}",
                        s.icon,
                        investment.get_string(s.unk4),
                        investment.get_string(s.unkc),
                        s.unk14
                    );
                }
                UnkInvestmentComponent::S808028E3(s) => {
                    println!("808028E3 entries:");
                    for (i, b) in s.unk0.iter().enumerate() {
                        println!(
                            "     {i} - {}",
                            investment.get_string(b.body_text).unwrap_or_default()
                        );
                    }
                }
                UnkInvestmentComponent::S808023B8(s) => {
                    println!("808023B8 entries:");
                    for (i, b) in s.unk0.iter().enumerate() {
                        println!(
                            "     {i} - unk0={} unkc={:?} unk14={:?}",
                            b.unk0,
                            investment.get_string(b.unkc),
                            investment.get_string(b.unk14),
                        );
                    }
                }
                UnkInvestmentComponent::Unknown { class, offset } => {
                    println!("Unknown {:08X} @ 0x{:X}", class, offset)
                }
            }
        }
    }

    Ok(())
}
