use tiger_parse::{tiger_type, ResourcePointer, ResourcePointerWithClass};
use tiger_pkg::TagHash;

use crate::map::SComponentDataListPtr;

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
