use anyhow::Context;
use deimos_data::tfx::ExternIndex;
use int_enum::IntEnum;

#[repr(u8)]
#[derive(Debug, Copy, Clone, PartialEq, Eq, IntEnum)]
pub enum Opcode {
    Add = 0x1,
    Subtract,
    Multiply,
    Divide,
    Multiply_,
    Add_,
    IsZero,
    Min,
    Max,
    LessThan,
    Dot,
    Merge1_3,
    Merge2_2,
    Merge3_1,

    Unknown0x0F = 0x0F,
    Unknown0x10,
    Unknown0x11,
    Cubic,
    Lerp,
    LerpSaturated,

    MultiplyAdd = 0x15,
    Clamp,
    Unknown0x17,
    Abs,
    Signum,
    Floor,
    Ceil,
    Round,
    Frac,
    Unknown0x1E,
    Unknown0x1F,
    Negate,
    VectorRotationsSin,
    VectorRotationsCos,
    VectorRotationsSinCos,

    Splat = 0x28,
    Permute,
    Saturate,
    Unknown0x24,
    Unknown0x25,
    Unknown0x26,
    Triangle,
    Jitter,
    Wander,
    Rand,
    RandSmooth,
    Unknown0x2C,
    Unknown0x2D,
    TransformVec4,

    // CompareLess = ???,
    // CompareLessEqual = ???,
    // CompareGreater = ???,
    // CompareGreaterEqual = ???,
    // CompareEqual = ???,
    // CompareNotEqual = ???,
    // CompareNotZeroTernary = ???,
    Unknown0x3B = 0x3B,
    Unknown0x3D = 0x3D,
    Unknown0x3F = 0x3F,
    Unknown0x41 = 0x41,

    PushConstVec4 = 0x42,
    LerpConstant,
    LerpConstantSaturated,
    Spline4Const,
    Spline8Const,
    Spline8ChainConst,
    Gradient4Const,
    Unk49,
    PushExternInputFloat,
    PushExternInputVec4,
    PushExternInputMat4,
    PushExternInputTextureView,
    PushExternInputU32,
    PushExternInputUav,
    Unk50,
    PushFromOutput,
    PopOutput,
    PopOutputMat4,
    PushTemp,
    PopTemp,
    PopTextureView,
    Unk57,
    PopSamplerState,
    PopUav,
    Unk5a,
    PushSamplerState,
    PushObjectChannelVector,
    PushGlobalChannelVector,
    Unk5e,
    Unk5f,
    // TODO(cohae): These need to be rechecked
    PushTexDimensions,
    PushTexTilingParams,
    PushTexTileLayerCount,
    Unk63,
    Unk64,
    Unk65,
    Unk66,
    Unk67,

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
            | Opcode::Divide
            | Opcode::IsZero
            | Opcode::Min
            | Opcode::Max
            | Opcode::LessThan
            | Opcode::Dot
            | Opcode::Merge1_3
            | Opcode::Merge2_2
            | Opcode::Merge3_1

            | Opcode::Unknown0x0F
            | Opcode::Unknown0x10
            | Opcode::Unknown0x11
            | Opcode::Cubic
            | Opcode::Lerp
            | Opcode::LerpSaturated
            | Opcode::MultiplyAdd
            | Opcode::Clamp
            | Opcode::Unknown0x17
            | Opcode::Abs
            | Opcode::Signum
            | Opcode::Floor
            | Opcode::Ceil
            | Opcode::Round
            | Opcode::Frac
            | Opcode::Unknown0x1E
            | Opcode::Unknown0x1F
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
            // | Opcode::CompareLess
            // | Opcode::CompareLessEqual
            // | Opcode::CompareGreater
            // | Opcode::CompareGreaterEqual
            // | Opcode::CompareEqual
            // | Opcode::CompareNotEqual
            // | Opcode::CompareNotZeroTernary
            | Opcode::Unknown0x3B
            | Opcode::Unknown0x3D
            | Opcode::Unknown0x3F
            | Opcode::Unknown0x41
            => 1,

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
            | Opcode::LerpConstantSaturated
            | Opcode::Spline4Const
            | Opcode::Spline8ChainConst
            | Opcode::Gradient4Const => 2,

            Opcode::PushExternInputFloat
            | Opcode::PushExternInputVec4
            | Opcode::PushExternInputMat4
            | Opcode::PushExternInputTextureView
            | Opcode::PushExternInputU32
            | Opcode::PushExternInputUav
            | Opcode::PushTexTilingParams
            | Opcode::PushTexTileLayerCount
            | Opcode::Unk63
            => 3,

            Opcode::PushObjectChannelVector => 5,

            Opcode::ExtReturn => 1,

            // Unknowns
            Opcode::Unk50
            | Opcode::Unk5f
            | Opcode::Unk64
            | Opcode::Unk65
            | Opcode::Unk66
            | Opcode::Unk67 => 1,

            Opcode::Unk49 | Opcode::Unk57 | Opcode::Unk5a | Opcode::Unk5e => 2,
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
    type Item = anyhow::Result<(Opcode, &'a [u8])>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.position >= self.data.len() {
            return None;
        }

        let byte = self.data[self.position];
        let opcode = match Opcode::try_from(byte) {
            Ok(op) => op,
            Err(_) => return Some(Err(anyhow::anyhow!("Unknown opcode: {:02X}", byte))),
        };

        let args_start = self.position + 1;
        let args_end = self.position + opcode.size();
        let size = opcode.size();
        self.position += size;
        let arg_data = if args_end <= self.data.len() {
            &self.data[args_start..args_end]
        } else {
            &self.data[args_start..]
        };

        Some(Ok((opcode, arg_data)))
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
pub fn pascal_to_snake(v: &str) -> String {
    let mut result = String::new();
    for (i, c) in v.chars().enumerate() {
        if i > 0 && c.is_uppercase() {
            result.push('_');
        }
        result.push(c.to_ascii_lowercase());
    }
    result
}
