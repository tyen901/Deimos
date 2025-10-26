use anyhow::Context;
use int_enum::IntEnum;

use crate::tfx::externs::ExternIndex;

#[repr(u8)]
#[derive(Debug, Copy, Clone, PartialEq, Eq, IntEnum)]
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

    CompareLess = 0x3b,
    CompareLessEqual = 0x3c,
    CompareGreater = 0x3d,
    CompareGreaterEqual = 0x3e,
    CompareEqual = 0x3f,
    CompareNotEqual = 0x40,
    CompareNotZeroTernary = 0x41,

    PushConstVec4 = 0x42,
    LerpConstant = 0x43,
    LerpConstantSaturated = 0x44,
    Spline4Const = 0x45,
    Spline8Const = 0x46,
    Spline8ChainConst = 0x47,
    Gradient4Const = 0x48,
    Unk3b = 0x49,
    PushExternInputFloat = 0x4a,
    PushExternInputVec4 = 0x4b,
    PushExternInputMat4 = 0x4c,
    PushExternInputTextureView = 0x4d,
    PushExternInputU32 = 0x4e,
    PushExternInputUav = 0x4f,
    Unk42 = 0x50,
    PushFromOutput = 0x51,
    PopOutput = 0x52,
    PopOutputMat4 = 0x53,
    PushTemp = 0x54,
    PopTemp = 0x55,
    PopTextureView = 0x56,
    Unk49 = 0x57,
    PopSamplerState = 0x58,
    PopUav = 0x59,
    Unk4c = 0x5a,
    PushSamplerState = 0x5b,
    PushObjectChannelVector = 0x5c,
    PushGlobalChannelVector = 0x5d,
    Unk50 = 0x5e,
    Unk51 = 0x5f,
    PushTexDimensions = 0x60,
    PushTexTilingParams = 0x61,
    PushTexTileLayerCount = 0x62,
    Unk55 = 0x63,
    Unk56 = 0x64,
    Unk57 = 0x65,
    Unk58 = 0x66,

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
            | Opcode::CompareLess
            | Opcode::CompareLessEqual
            | Opcode::CompareGreater
            | Opcode::CompareGreaterEqual
            | Opcode::CompareEqual
            | Opcode::CompareNotEqual
            | Opcode::CompareNotZeroTernary => 1,

            Opcode::PopOutput
            | Opcode::PushTemp
            | Opcode::PopTemp
            | Opcode::PopSamplerState
            | Opcode::PushSamplerState
            | Opcode::PushFromOutput
            | Opcode::PopOutputMat4
            | Opcode::PopTextureView
            | Opcode::PushGlobalChannelVector
            | Opcode::PushConstVec4
            | Opcode::LerpConstant
            | Opcode::Spline8Const
            | Opcode::Permute
            | Opcode::PopUav
            | Opcode::PushTexDimensions
            | Opcode::PushTexTilingParams
            | Opcode::LerpConstantSaturated
            | Opcode::Spline4Const
            | Opcode::Spline8ChainConst
            | Opcode::Gradient4Const
            | Opcode::PushTexTileLayerCount => 2,

            Opcode::PushExternInputFloat
            | Opcode::PushExternInputVec4
            | Opcode::PushExternInputMat4
            | Opcode::PushExternInputTextureView
            | Opcode::PushExternInputU32
            | Opcode::PushExternInputUav => 3,

            Opcode::PushObjectChannelVector => 5,

            Opcode::ExtReturn => 1,

            // Unknowns
            Opcode::Unk42
            | Opcode::Unk51
            | Opcode::Unk55
            | Opcode::Unk56
            | Opcode::Unk57
            | Opcode::Unk58 => 1,

            Opcode::Unk3b | Opcode::Unk49 | Opcode::Unk4c | Opcode::Unk50 => 2,
        }
    }
}

pub struct OpcodeIterator<'a> {
    data: &'a [u8],
    position: usize,
}

impl<'a> OpcodeIterator<'a> {
    pub fn new(data: &'a [u8]) -> Self {
        Self { data, position: 0 }
    }
}

impl<'a> Iterator for OpcodeIterator<'a> {
    type Item = anyhow::Result<Opcode>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.position >= self.data.len() {
            return None;
        }

        let byte = self.data[self.position];
        let opcode = match Opcode::try_from(byte) {
            Ok(op) => op,
            Err(_) => return Some(Err(anyhow::anyhow!("Unknown opcode: {:02X}", byte))),
        };

        let size = opcode.size();
        self.position += size;

        Some(Ok(opcode))
    }
}

pub fn disassemble(data: &[u8]) -> anyhow::Result<Vec<String>> {
    let mut result = Vec::new();
    let mut i = 0;
    while i < data.len() {
        let Ok(opcode) = Opcode::try_from(data[i]) else {
            anyhow::bail!("Unimplemented opcode: {:02X} ({:02X?})", data[i], data);
        };

        let opcode_size = opcode.size();
        let mut line = format!(
            "{:02X}: {:02X} {} ",
            i,
            data[i],
            pascal_to_snake(&format!("{opcode:?}"))
        );
        for j in 1..opcode_size {
            line.push_str(&format!(
                "{:02X} ",
                data.get(i + j).context("Opcode size invalid")?
            ));
        }
        // println!("{line}");
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
