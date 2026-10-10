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

/// Select independent constant assignments for a renderer's consumed rows.
/// Programs with shared stack state, temporaries or output reads require a
/// dependency graph and are rejected rather than silently losing dependencies.
pub fn independent_outputs(bytecode: &[u8], required: &[u8]) -> anyhow::Result<Vec<u8>> {
    use deimos_data::tfx::opcodes::{Opcode, OpcodeIterator};
    let mut output = Vec::new();
    let (mut offset, mut start, mut depth) = (0, 0, 0usize);
    for instruction in OpcodeIterator::new(bytecode) {
        let (op, args) = instruction?;
        let (consumed, produced) = match op {
            Opcode::PushConstVec4
            | Opcode::PushExternInputFloat
            | Opcode::PushExternInputVec4
            | Opcode::PushGlobalChannelVector
            | Opcode::Unknown0x5e
            | Opcode::PushObjectChannelVector => (0, 1),
            Opcode::Add
            | Opcode::Add_
            | Opcode::Subtract
            | Opcode::Multiply
            | Opcode::Multiply_
            | Opcode::Divide
            | Opcode::Min
            | Opcode::Max
            | Opcode::LessThan
            | Opcode::Dot
            | Opcode::Merge1_3
            | Opcode::Merge2_2
            | Opcode::Merge3_1
            | Opcode::Cubic
            | Opcode::Spline8ChainConst => (2, 1),
            Opcode::Lerp | Opcode::LerpSaturated | Opcode::MultiplyAdd | Opcode::Clamp => (3, 1),
            Opcode::IsZero
            | Opcode::Abs
            | Opcode::Signum
            | Opcode::Floor
            | Opcode::Ceil
            | Opcode::Round
            | Opcode::Frac
            | Opcode::Negate
            | Opcode::VectorRotationsSin
            | Opcode::VectorRotationsCos
            | Opcode::VectorRotationsSinCos
            | Opcode::Splat
            | Opcode::Permute
            | Opcode::Saturate
            | Opcode::LerpConstant
            | Opcode::LerpConstantSaturated
            | Opcode::Spline4Const
            | Opcode::Spline8Const
            | Opcode::Gradient4Const => (1, 1),
            Opcode::PopOutput => (1, 0),
            _ => anyhow::bail!("Independent output selection does not support {op:?}"),
        };
        anyhow::ensure!(depth >= consumed, "Constant expression stack underflow");
        depth = depth - consumed + produced;
        offset += op.size();
        if op == Opcode::PopOutput {
            anyhow::ensure!(depth == 0, "Constant assignments share stack state");
            if required.contains(&args[0]) {
                output.extend_from_slice(&bytecode[start..offset]);
            }
            start = offset;
        }
    }
    anyhow::ensure!(
        depth == 0 && start == offset,
        "Unterminated constant assignment"
    );
    Ok(output)
}
