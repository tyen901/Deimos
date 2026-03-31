use anyhow::Context;
use deimos_data::{
    investment::{SIndexedString, globals::SInvestmentGlobals},
    strings::StringContainer,
};
use tiger_parse::{PackageManagerExt, TigerReadable};
use tiger_pkg::package_manager;

pub struct Investment {
    pub globals: SInvestmentGlobals,

    string_tables: Vec<StringContainer>,
}

impl Investment {
    pub fn load() -> anyhow::Result<Self> {
        let globals_tag = package_manager()
            .get_named_tag("investment_globals_retail", SInvestmentGlobals::ID.unwrap())
            .context("failed to get investment globals tag")?;
        let globals: SInvestmentGlobals = package_manager()
            .read_tag_struct(globals_tag)
            .context("failed to read investment globals tag")?;

        let mut strings = vec![];
        for s in &globals.string_tables.table {
            let string_table = StringContainer::load(s.container)?;
            strings.push(string_table);
        }

        Ok(Self {
            globals,
            string_tables: strings,
        })
    }

    pub fn get_string(&self, index: SIndexedString) -> Option<String> {
        self.string_tables
            .get(index.table_index as usize)?
            .try_get(index.string_id)
    }
}
