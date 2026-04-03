use tiger_parse::{tiger_type, FnvHash, Padding, ResourcePointer, ResourcePointerWithClass};
use tiger_pkg::TagHash;

use crate::{map::SComponentDataListPtr, tfx::sequencer::SExpression};

#[tiger_type(id = 0x8080BAAD)]
pub struct SPattern {
    pub file_size: u64,
    pub components: Vec<SComponentRef>,
}

#[tiger_type(id = 0x8080BAA2)]
pub struct SComponentRef {
    pub component: TagHash,
    pub unk4: u32,
    pub unk8: u32,
}

#[tiger_type(id = 0x8080BADB, size = 0x88)]
pub struct SComponent {
    pub file_size: u64,
    // cohae: This field isn't a list, but it uses the same layout as ComponentDataListPtr
    pub dynamic_data: SComponentDataListPtr,
    pub default_instance: ResourcePointer,
    pub definition: ResourcePointer,

    pub unk20: Vec<ResourcePointerWithClass>,

    #[tiger(offset = 0x30)]
    pub resource_table1: Vec<()>,
}

#[tiger_type(id = 0x8080AF75, size = 0x1A8)]
pub struct SObjectChannelComponent {
    #[tiger(offset = 0x120)]
    pub m_providers: Vec<()>,
    #[tiger(offset = 0x130)]
    pub m_channels: Vec<SObjectChannel>,
}

#[tiger_type(id = 0x8080AF86, size = 0x70)]
pub struct SObjectChannel {
    pub name: FnvHash,
    _pad4: Padding<4>,
    pub expression: SExpression,

    #[tiger(offset = 0x60)]
    pub interpolation: u64,
}
