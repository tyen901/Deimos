use d3d12::D3D12_SHADING_RATE_COMBINER;
use tiger_parse::{tiger_type, tiger_variant_enum, FnvHash, Padding, VariantPointer};

use crate::investment::SIndexedString;

#[tiger_type(id = 0x808029D1)]
pub struct S808029D1 {
    pub file_size: u64,
    pub table: Vec<S808029AA>,
}
#[tiger_type(id = 0x808029AA, size = 0x18)]
pub struct S808029AA {
    pub unk0: FnvHash,
    pub unk4: FnvHash,
    pub components: Vec<VariantPointer<UnkInvestmentComponent>>,
}

tiger_variant_enum! {
    #[derive(Debug, Clone)]
    [Unknown(true)]
    enum UnkInvestmentComponent {
        S808028E3,
        S808028E8,
        S808028E9,
        S808023B8
    }
}

#[derive(Debug, Clone)]
#[tiger_type(id = 0x808028E8, size = 0x50)]
pub struct S808028E8 {
    pub unk0: u32,
    pub unk4: u32,
    pub unk8: u32,
    pub unkc: u32,
    pub unk10: u32,
    pub unk14: SIndexedString,
    pub unk1c: SIndexedString,
    pub unk24: u32,
    pub unk28: SIndexedString,
    pub unk30: SIndexedString,
    pub unk38: u32,
    pub unk3c: SIndexedString,
    pub unk44: SIndexedString,
}

#[derive(Debug, Clone)]
#[tiger_type(id = 0x808028E9, size = 0x18)]
pub struct S808028E9 {
    pub icon: i16,
    _pad: Padding<2>,
    pub unk4: SIndexedString,
    pub unkc: SIndexedString,
    pub unk14: i16,
    _pad16: Padding<2>,
}

#[derive(Debug, Clone)]
#[tiger_type(id = 0x808028E3)]
pub struct S808028E3 {
    pub unk0: Vec<S808028E5>,
}

#[derive(Debug, Clone)]
#[tiger_type(id = 0x808028E5, size = 0x20)]
pub struct S808028E5 {
    pub body_text: SIndexedString,
}

#[derive(Debug, Clone)]
#[tiger_type(id = 0x808023B8)]
pub struct S808023B8 {
    pub unk0: Vec<S808023BA>,
}

#[derive(Debug, Clone)]
#[tiger_type(id = 0x808023BA, size = 0x24)]
pub struct S808023BA {
    pub unk0: u32,
    pub unk4: FnvHash,
    pub unk8: u32,
    pub unkc: SIndexedString,
    pub unk14: SIndexedString,
    pub unk1c: u32,
    pub unk20: u32,
}
