use glam::Vec4;
use tiger_parse::tiger_type;

#[tiger_type(id = 0x8080B7A8, size = 0x30)]
pub struct SExpression {
    pub bytecode: Vec<u8>,
    pub bytecode_constants: Vec<Vec4>,
    pub unk20: u64,
    pub unk28: u64,
}
