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

/// Select consumed, independent assignments without evaluating unused source inputs.
/// Each retained expression must build its own stack value; output reads and
/// temporary dependencies remain explicit errors inside a consumed expression.
pub fn independent_outputs(bytecode: &[u8], required: &[u8]) -> anyhow::Result<Vec<u8>> {
    use deimos_data::tfx::opcodes::{Opcode, OpcodeIterator};
    let mut instructions = Vec::new();
    let mut offset = 0;
    for instruction in OpcodeIterator::new(bytecode) {
        let (op, args) = instruction?;
        let end = offset + op.size();
        instructions.push((offset, end, op, args));
        offset = end;
    }
    let mut retained = vec![false; instructions.len()];
    for (index, (_, _, op, args)) in instructions.iter().enumerate() {
        if *op != Opcode::PopOutput || !required.contains(&args[0]) { continue; }
        retained[index] = true;
        let mut needed = 1usize;
        let mut cursor = index;
        while needed != 0 {
            anyhow::ensure!(cursor != 0, "Consumed constant expression stack underflow");
            cursor -= 1;
            let op = instructions[cursor].2;
            anyhow::ensure!(op != Opcode::PopOutput, "Consumed constant expression crosses an assignment");
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
            anyhow::ensure!(produced <= needed, "Consumed constant expression shares stack state");
            needed = needed - produced + consumed;
            retained[cursor] = true;
        }
    }
    let mut output = Vec::new();
    for ((start, end, _, _), keep) in instructions.iter().zip(retained) {
        if keep { output.extend_from_slice(&bytecode[*start..*end]); }
    }
    Ok(output)
}
