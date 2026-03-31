use std::fs::File;
use std::io::Write;

use anyhow::Context;
use deimos_investment::Investment;

fn main() -> anyhow::Result<()> {
    deimos_core::initialize_package_manager(None)?;

    std::fs::create_dir("investment");
    let investment = Investment::load().context("failed to load investment data")?;
    let mut signatures = File::create("investment/signatures.csv")?;

    let mut items = vec![];
    for item in &investment.globals.items.table {
        let display = &item.definition.display_properties;

        items.push(InvestmentItem {
            id: item.id,
            tag: item.definition.taghash().to_string(),
            name: investment.get_string(display.name),
            icon: investment.get_string(display.icon),
            item_type: investment.get_string(display.item_type),
            description: investment.get_string(display.description),
            unk28: investment.get_string(display.unk28),
            background_text: investment.get_string(display.background_text),
        });

        if let Some(name) = investment.get_string(display.name) {
            if name.contains("\n") {
                continue;
            }
            writeln!(&mut signatures, "0x{:X},itemdef('{name}')", item.id)?;
        }
    }

    let json = serde_json::to_string_pretty(&items)?;
    std::fs::write("investment/items.json", json)?;

    let mut traits = vec![];
    for def in &investment.globals.traits.table {
        traits.push(TraitItem {
            id0: def.id,
            id1: def.id1,
            name: investment.get_string(def.name),
            description: investment.get_string(def.description),
            unk18: investment.get_string(def.unk18),
        });

        if let Some(name) = investment.get_string(def.name)
            && let Some(description) = investment.get_string(def.description)
        {
            if name.contains("\n") {
                continue;
            }
            writeln!(
                &mut signatures,
                "0x{:X},itemtrait('{name}|{}')",
                def.id,
                description.replace("\n", "<NL>")
            )?;
        }
    }

    let json = serde_json::to_string_pretty(&traits)?;
    std::fs::write("investment/traits.json", json)?;

    Ok(())
}

#[derive(serde::Serialize)]
struct InvestmentItem {
    id: u32,
    tag: String,

    name: Option<String>,
    icon: Option<String>,
    item_type: Option<String>,
    description: Option<String>,
    unk28: Option<String>,
    background_text: Option<String>,
}

#[derive(serde::Serialize)]
struct TraitItem {
    id0: u32,
    id1: u32,

    name: Option<String>,
    description: Option<String>,
    unk18: Option<String>,
}
