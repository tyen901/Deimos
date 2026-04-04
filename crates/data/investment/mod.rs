use tiger_parse::{tiger_type, FnvHash};

pub mod codex;
pub mod dialogue;
pub mod globals;
pub mod item;
pub mod traits;

#[tiger_type(size = 0x8)]
#[derive(Debug, Clone, Copy)]
pub struct SIndexedString {
    pub table_index: u16,
    #[tiger(offset = 0x4)]
    pub string_id: FnvHash,
}
