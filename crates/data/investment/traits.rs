use tiger_parse::{tiger_type, FnvHash};

use crate::investment::SIndexedString;

pub type InvestmentTraitId = FnvHash;

#[tiger_type(id = 0x80806B54)]
pub struct SInvestmentTraitTable {
    pub file_size: u64,

    pub table: Vec<SInvestmentTrait>,
}

#[tiger_type(id = 0x808066AD, size = 0x28)]
pub struct SInvestmentTrait {
    pub id: InvestmentTraitId,
    pub id1: InvestmentTraitId,
    pub name: SIndexedString,
    pub description: SIndexedString,
    pub unk18: SIndexedString,
    pub unk20: FnvHash,
    pub unk24: u32,
}
