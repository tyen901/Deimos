use anyhow::Context;

use crate::tfx::externs::ExternIndex;

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum Opcode {
    Add = 0x1,
    Add_ = 0x6,
    Subtract = 0x2,
    Multiply = 0x3,
    Multiply_ = 0x5,
    UnkDivide = 0x4,
    IsZero = 0x7,
    Min = 0x8,
    Max = 0x9,
    LessThan = 0xA,
    Dot = 0xB,
    Merge1_3 = 0xC,
    Merge2_2 = 0xD,
    Merge3_1 = 0xE,
    Cubic = 0xF,
    Lerp = 0x10,
    LerpSaturated = 0x11,
    MultiplyAdd = 0x12,
    Clamp = 0x13,
    Unknown0x14 = 0x14,
    Abs = 0x15,
    Signum = 0x16,
    Floor = 0x17,
    Ceil = 0x18,
    Round = 0x19,
    Frac = 0x1A,
    Unknown0x1B = 0x1B,
    Unknown0x1C = 0x1C,
    Negate = 0x1D,
    VectorRotationsSin = 0x1E,
    VectorRotationsCos = 0x1F,
    VectorRotationsSinCos = 0x20,
    Splat = 0x21,
    Permute = 0x22,
    Saturate = 0x23,
    Unknown0x24 = 0x24,
    Unknown0x25 = 0x25,
    Unknown0x26 = 0x26,
    Triangle = 0x27,
    Jitter = 0x28,
    Wander = 0x29,
    Rand = 0x2A,
    RandSmooth = 0x2B,
    Unknown0x2C = 0x2C,
    Unknown0x2D = 0x2D,
    TransformVec4 = 0x2E,
    Unknown0x2F = 0x2F,
    Unknown0x30 = 0x30,
    Unknown0x31 = 0x31,
    Unknown0x32 = 0x32,
    Unknown0x33 = 0x33,
    PushConstVec4 = 0x34,
    LerpConstant = 0x35,
    Unknown0x36 = 0x36,
    Unknown0x37 = 0x37,
    Spline8Const = 0x38,
    Unknown0x39 = 0x39,
    Unknown0x3A = 0x3A,
    Unknown0x3B = 0x3B,
    PushExternInputFloat = 0x3C,
    PushExternInputVec4 = 0x3D,
    PushExternInputMat4 = 0x3E,
    PushExternInputTextureView = 0x3F,
    PushExternInputU32 = 0x40,
    PushExternInputUav = 0x41,
    PushFromOutput = 0x42,
    PopOutput = 0x43,
    PopOutputMat4 = 0x44,
    PushTemp = 0x45,
    PopTemp = 0x46,
    PopTextureView = 0x47,
    Unknown0x48 = 0x48,
    PopSamplerState = 0x49,
    PopUav = 0x4A,
    Unknown0x4B = 0x4B,
    PushSamplerState = 0x4C,
    Unknown0x4D = 0x4D,
    PushGlobalChannelVector = 0x4E,
    Unknown0x4F = 0x4F,
    Unknown0x50 = 0x50,
    Unknown0x51 = 0x51,
    Unknown0x52 = 0x52,
    Unknown0x53 = 0x53,
    Unknown0x54 = 0x54,

    // Extended instruction set (only used internally by the interpreter)
    ExtReturn = 0x80,
    // /// Bind a texture to a sampler in a single call (inserted by the Dawn's bytecode optimizer, not used by Tiger)
    // ExtBindTexture = 0x81,
    // /// Bind a sampler to a sampler state in a single call (inserted by Dawn's bytecode optimizer, not used by Tiger)
    // ExtBindSampler = 0x82,
}

impl Opcode {
    /// Returns the size of the opcode in bytes, including the opcode itself.
    pub fn size(&self) -> usize {
        match self {
            Opcode::Add
            | Opcode::Add_
            | Opcode::Subtract
            | Opcode::Multiply
            | Opcode::Multiply_
            | Opcode::UnkDivide
            | Opcode::IsZero
            | Opcode::Min
            | Opcode::Max
            | Opcode::LessThan
            | Opcode::Dot
            | Opcode::Merge1_3
            | Opcode::Merge2_2
            | Opcode::Merge3_1
            | Opcode::Cubic
            | Opcode::Lerp
            | Opcode::LerpSaturated
            | Opcode::MultiplyAdd
            | Opcode::Clamp
            | Opcode::Unknown0x14
            | Opcode::Abs
            | Opcode::Signum
            | Opcode::Floor
            | Opcode::Ceil
            | Opcode::Round
            | Opcode::Frac
            | Opcode::Unknown0x1B
            | Opcode::Unknown0x1C
            | Opcode::Negate
            | Opcode::VectorRotationsSin
            | Opcode::VectorRotationsCos
            | Opcode::VectorRotationsSinCos
            | Opcode::Splat
            | Opcode::Saturate
            | Opcode::Triangle
            | Opcode::Jitter
            | Opcode::Wander
            | Opcode::Rand
            | Opcode::RandSmooth
            | Opcode::Unknown0x25
            | Opcode::Unknown0x26
            | Opcode::TransformVec4
            | Opcode::Unknown0x24
            | Opcode::Unknown0x2C
            | Opcode::Unknown0x2D
            | Opcode::Unknown0x2F
            | Opcode::Unknown0x30
            | Opcode::Unknown0x31
            | Opcode::Unknown0x32
            | Opcode::Unknown0x33
            | Opcode::Unknown0x50
            | Opcode::Unknown0x51
            | Opcode::Unknown0x52
            | Opcode::Unknown0x53
            | Opcode::Unknown0x54 => 1,

            Opcode::PopOutput
            | Opcode::PushTemp
            | Opcode::PopTemp
            | Opcode::PopSamplerState
            | Opcode::PushSamplerState
            | Opcode::Unknown0x36
            | Opcode::Unknown0x37
            | Opcode::Unknown0x39
            | Opcode::Unknown0x3A
            | Opcode::Unknown0x3B
            | Opcode::PushFromOutput
            | Opcode::PopOutputMat4
            | Opcode::PopTextureView
            | Opcode::Unknown0x48
            | Opcode::Unknown0x4D
            | Opcode::PushGlobalChannelVector
            | Opcode::PushConstVec4
            | Opcode::LerpConstant
            | Opcode::Spline8Const
            | Opcode::Permute
            | Opcode::PopUav
            | Opcode::Unknown0x4B
            | Opcode::Unknown0x4F => 2,

            Opcode::PushExternInputFloat
            | Opcode::PushExternInputVec4
            | Opcode::PushExternInputMat4
            | Opcode::PushExternInputTextureView
            | Opcode::PushExternInputU32
            | Opcode::PushExternInputUav => 3,

            Opcode::ExtReturn => 1,
        }
    }
}

impl TryFrom<u8> for Opcode {
    type Error = ();
    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            // Tiger opcodes
            x if (0x1..=0x54).contains(&x) => Ok(unsafe { std::mem::transmute(x) }),
            // Internal opcodes
            x if (0x80..=0x80).contains(&x) => Ok(unsafe { std::mem::transmute(x) }),
            _ => Err(()),
        }
    }
}

pub fn disassemble(data: &[u8]) -> anyhow::Result<Vec<String>> {
    let mut result = Vec::new();
    let mut i = 0;
    while i < data.len() {
        let Ok(opcode) = Opcode::try_from(data[i]) else {
            anyhow::bail!("Unimplemented opcode: {:02X}", data[i]);
        };

        let opcode_size = opcode.size();
        let mut line = format!("{:02X}: {} ", i, pascal_to_snake(&format!("{opcode:?}")));
        for j in 1..opcode_size {
            line.push_str(&format!("{:02X} ", data[i + j]));
        }
        result.push(line);
        i += opcode_size;
    }
    Ok(result)
}

pub fn get_texture_externs_from_bytecode(
    data: &[u8],
) -> anyhow::Result<Vec<(u32, ExternIndex, u32)>> {
    let mut result = Vec::new();
    let mut i = 0;

    let mut last_extern = None;
    while i < data.len() {
        let ptr = &data[i..];
        let Ok(opcode) = Opcode::try_from(ptr[0]) else {
            anyhow::bail!("Unimplemented opcode: {:02X}", data[i]);
        };

        let opcode_size = opcode.size();
        match opcode {
            Opcode::PushExternInputTextureView => {
                let extern_id = ExternIndex::try_from(ptr[1])
                    .ok()
                    .context("Invalid extern index")?;
                let offset = ptr[2] as u32 * 8;

                last_extern = Some((extern_id, offset));
            }
            Opcode::PopTextureView => {
                let slot = ptr[1] & 0x1F;
                if let Some((extern_id, offset)) = last_extern {
                    result.push((slot as u32, extern_id, offset));
                }
            }
            _ => {}
        }
        i += opcode_size;
    }
    Ok(result)
}

// FooBar -> foo_bar
fn pascal_to_snake(v: &str) -> String {
    let mut result = String::new();
    for (i, c) in v.chars().enumerate() {
        if i > 0 && c.is_uppercase() {
            result.push('_');
        }
        result.push(c.to_ascii_lowercase());
    }
    result
}
