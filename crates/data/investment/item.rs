use tiger_parse::{tiger_type, FnvHash};

use crate::{investment::SIndexedString, tag::TagRef};

pub type InvestmentItemId = FnvHash;

#[tiger_type(id = 0x80806EF0)]
pub struct SInvestmentItemTable {
    pub file_size: u64,

    pub table: Vec<SInvestmentItem>,
}

#[tiger_type(id = 0x80806EF4, size = 0x20)]
pub struct SInvestmentItem {
    pub id: InvestmentItemId,
    #[tiger(offset = 0x10)]
    pub definition: TagRef<SInvestmentItemDefinition>,
}

#[derive(Debug)]
#[tiger_type(id = 0x80806EF6, size = 0x198)]
pub struct SInvestmentItemDefinition {
    pub file_size: u64,

    #[tiger(offset = 0xB8)]
    pub display_properties: SItemDisplayProperties,
}

#[derive(Debug)]
#[tiger_type(id = 0x80806F08, size = 0xA0)]
pub struct SItemDisplayProperties {
    pub unk0: u32,
    pub name: SIndexedString,
    pub icon: SIndexedString,
    pub unk14: u32,
    pub item_type: SIndexedString,
    pub description: SIndexedString,
    pub unk28: SIndexedString,
    pub background_text: SIndexedString,
}
