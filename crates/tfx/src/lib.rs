//! Shared CPU material expression evaluator. The embedding renderer supplies
//! typed external values; package decoding and GPU handles stay outside the VM.
pub mod helpers;
pub mod interpreter;
use deimos_data::tfx::ExternIndex;
use glam::{Mat4, Vec4};
pub use interpreter::InterpreterState;

pub trait Inputs {
    fn float(&self, index: ExternIndex, byte_offset: usize) -> anyhow::Result<f32>;
    fn vector(&self, index: ExternIndex, byte_offset: usize) -> anyhow::Result<Vec4>;
    fn matrix(&self, index: ExternIndex, byte_offset: usize) -> anyhow::Result<Mat4>;
    fn global_channel(&self, index: u8) -> anyhow::Result<Vec4>;
    fn object_channel(&self, hash: u32) -> anyhow::Result<Vec4>;
}

/// Constant evaluation excludes GPU resource assignments, exactly as the
/// source DynamicCore does after separately resolving texture/sampler bindings.
pub fn constant_bytecode(original: &[u8]) -> anyhow::Result<Vec<u8>> {
    use deimos_data::tfx::opcodes::{Opcode, OpcodeIterator};
    let mut output = Vec::with_capacity(original.len());
    let mut offset = 0;
    for instruction in OpcodeIterator::new(original) {
        let (opcode, _) = instruction?;
        if !matches!(
            opcode,
            Opcode::PushSamplerState
                | Opcode::PopSamplerState
                | Opcode::PushExternInputTextureView
                | Opcode::PopTextureView
        ) {
            output.extend_from_slice(&original[offset..offset + opcode.size()]);
        }
        offset += opcode.size();
    }
    Ok(output)
}
