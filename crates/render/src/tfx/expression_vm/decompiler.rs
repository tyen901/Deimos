use anyhow::{ensure, Context};
use deimos_data::tfx::ExternIndex;
use glam::Vec4;

use crate::tfx::externs::Externs;

use super::opcodes::Opcode;

pub struct DecompilerState<'a> {
    data: &'a [u8],
    pub ip: usize,

    stack: [String; 32],
    stack_pointer: usize,

    temp: [String; 16],
}

impl<'a> DecompilerState<'a> {
    pub fn new(data: &'a [u8]) -> Self {
        Self {
            ip: 0,
            data,
            stack: std::array::from_fn(|_| "NULL".to_string()),
            stack_pointer: 0,
            temp: std::array::from_fn(|_| "NULL".to_string()),
        }
    }

    fn data_ptr(&self) -> &[u8] {
        &self.data[self.ip..]
    }

    #[must_use = "Pushed value must be stored in the cache register"]
    #[inline(always)]
    fn push(&mut self, value: String) -> anyhow::Result<String> {
        #[cfg(debug_assertions)]
        anyhow::ensure!(
            self.stack_pointer < self.stack.len(),
            "Stack overflow (ip=0x{:X}, sp={})",
            self.ip,
            self.stack_pointer
        );
        self.stack_pointer += 1;
        self.stack[self.stack_pointer] = value.clone();
        Ok(value)
    }

    // #[inline(always)]
    // fn pop(&mut self) -> anyhow::Result<Vec4> {
    //     anyhow::ensure!(self.stack_pointer < 16, "Stack underflow");
    //     let value = self.stack[self.stack_pointer];
    //     self.stack_pointer += 1;
    //     Ok(value)
    // }

    #[inline(always)]
    fn get(&self, index_relative: isize) -> anyhow::Result<String> {
        let index = self.stack_pointer as isize + index_relative;
        #[cfg(debug_assertions)]
        anyhow::ensure!(
            (0..16).contains(&index),
            "Stack index out of bounds (ip=0x{:X})",
            self.ip
        );
        Ok(self.stack[index as usize].clone())
    }

    // Pops the top value off the stack and returns the value at the new top of the stack (or ZERO if the stack is empty)
    #[inline(always)]
    fn pop_top(&mut self) -> String {
        self.stack_pointer = self.stack_pointer.saturating_sub(1);
        self.stack
            .get(self.stack_pointer)
            .cloned()
            .unwrap_or("NULL".to_string())
    }

    #[inline(always)]
    fn stack_top(&mut self) -> &mut String {
        &mut self.stack[self.stack_pointer]
    }

    // // Pops the top N values off the stack and returns the value at the new top of the stack (or ZERO if the stack is empty)
    // fn pop_n(&mut self, n: usize) -> Vec4 {
    //     self.stack_pointer = self.stack_pointer.saturating_add(n);
    //     self.stack
    //         .get(self.stack_pointer)
    //         .copied()
    //         .unwrap_or(Vec4::ZERO)
    // }

    // #[profiling::function]
    pub fn evaluate(&mut self, constants: &[Vec4], out: &mut [String]) -> anyhow::Result<()> {
        let mut cached_top = "NULL".to_string();

        macro_rules! set_top {
            ($value:expr) => {{
                cached_top = $value;
                *self.stack_top() = cached_top.clone();
            }};
        }

        'exec: while self.ip < self.data.len() {
            let ptr = self.data_ptr();
            let Ok(op) = Opcode::try_from(ptr[0]) else {
                anyhow::bail!("Invalid opcode: 0x{:02X} @ ip 0x{:X}", ptr[0], self.ip);
            };

            match op {
                Opcode::ExtReturn => {
                    break 'exec;
                }
                Opcode::Add | Opcode::Add_ => {
                    cached_top = format!("({} + {cached_top})", self.get(-1)?);
                    self.stack_pointer -= 1;
                    *self.stack_top() = cached_top.clone();
                }
                Opcode::Subtract => {
                    cached_top = format!("({} - {cached_top})", self.get(-1)?);
                    self.stack_pointer -= 1;
                    *self.stack_top() = cached_top.clone();
                }
                Opcode::Multiply | Opcode::Multiply_ => {
                    cached_top = format!("({} * {cached_top})", self.get(-1)?);
                    self.stack_pointer -= 1;
                    *self.stack_top() = cached_top.clone();
                }
                Opcode::UnkDivide => {
                    let v1 = self.get(-1)?;

                    cached_top = format!("({v1} / {cached_top})");

                    self.stack_pointer -= 1;
                    *self.stack_top() = cached_top.clone();
                }
                Opcode::Min => {
                    cached_top = format!("min({}, {cached_top})", self.get(-1)?);
                    self.stack_pointer -= 1;
                    *self.stack_top() = cached_top.clone();
                }
                Opcode::Max => {
                    cached_top = format!("max({}, {cached_top})", self.get(-1)?);
                    self.stack_pointer -= 1;
                    *self.stack_top() = cached_top.clone();
                }
                Opcode::Dot => {
                    cached_top = format!("dot({}, {cached_top})", self.get(-1)?);
                    self.stack_pointer -= 1;
                    *self.stack_top() = cached_top.clone();
                }
                Opcode::Merge1_3 => {
                    let a0 = cached_top;
                    let a1 = self.get(-1)?;
                    self.stack_pointer -= 1;
                    cached_top = format!("float4({a1}.x, {a0}.xyz)");
                    *self.stack_top() = cached_top.clone();
                }
                Opcode::Merge2_2 => {
                    let a0 = cached_top;
                    let a1 = self.get(-1)?;
                    self.stack_pointer -= 1;
                    cached_top = format!("float4({a1}.xy, {a0}.xy)");
                    *self.stack_top() = cached_top.clone();
                }
                Opcode::Merge3_1 => {
                    let a0 = cached_top;
                    let a1 = self.get(-1)?;
                    self.stack_pointer -= 1;
                    set_top!(format!("float4({a1}.xyz, {a0}.x)"));
                }
                Opcode::Cubic => {
                    let x = cached_top;
                    let coefficients = self.get(-1)?;
                    set_top!(format!("cubic({x}, {coefficients})"))
                }
                Opcode::Lerp => {
                    let s = cached_top;
                    let y = self.get(-1)?;
                    let x = self.get(-2)?;
                    self.stack_pointer -= 2;
                    set_top!(format!("lerp({x}, {y}, {s})"));
                }
                Opcode::MultiplyAdd => {
                    let c = cached_top;
                    let b = self.get(-1)?;
                    let a = self.get(-2)?;
                    self.stack_pointer -= 2;
                    set_top!(format!("({a} * {b} + {c})"));
                }
                Opcode::Clamp => {
                    let min = cached_top;
                    let max = self.get(-1)?;
                    let value = self.get(-2)?;
                    self.stack_pointer -= 2;
                    set_top!(value.clamp(min, max));
                }
                Opcode::Floor => {
                    set_top!(format!("floor({cached_top})"));
                }
                Opcode::Ceil => {
                    set_top!(format!("ceil({cached_top})"));
                }
                Opcode::Round => {
                    set_top!(format!("round({cached_top})"));
                }
                Opcode::Frac => {
                    set_top!(format!("fract({cached_top})"));
                }
                Opcode::Negate => {
                    set_top!(format!("(-{cached_top})"));
                }
                Opcode::Splat => {
                    set_top!(format!("{cached_top}.xxxx"));
                }
                Opcode::Permute => {
                    let fields = ptr[1];
                    let x = (fields >> 6) & 0b11;
                    let y = (fields >> 4) & 0b11;
                    let z = (fields >> 2) & 0b11;
                    let w = fields & 0b11;

                    const COMPONENTS: [char; 4] = ['x', 'y', 'z', 'w'];

                    set_top!(format!(
                        "{cached_top}.{}{}{}{}",
                        COMPONENTS[x as usize],
                        COMPONENTS[y as usize],
                        COMPONENTS[z as usize],
                        COMPONENTS[w as usize]
                    ));
                }
                Opcode::Saturate => {
                    set_top!(format!("saturate({cached_top})"));
                }
                Opcode::Unknown0x25 => {
                    set_top!(format!("bytecode_op_25({cached_top})"));
                }
                Opcode::Triangle => {
                    set_top!(format!("triangle({cached_top})"));
                }
                Opcode::Jitter => {
                    set_top!(format!("jitter({cached_top})"));
                }
                Opcode::Wander => {
                    set_top!(format!("wander({cached_top})"));
                }
                Opcode::Rand => {
                    set_top!(format!("rand({cached_top})"));
                }
                Opcode::TransformVec4 => {
                    let value = cached_top;
                    let mat = self.get(-1)?;

                    self.stack_pointer -= 1;

                    set_top!(format!("mul({mat}, {value})"));
                }
                Opcode::VectorRotationsSin => {
                    // set_top!(super::helpers::_trig_helper_vector_sin_rotations_estimate(
                    //     cached_top
                    // ));
                    set_top!(format!("vector_sin_rotations_estimate({cached_top})"));
                }
                Opcode::VectorRotationsCos => {
                    set_top!(format!("vector_cos_rotations_estimate({cached_top})"));
                }
                Opcode::VectorRotationsSinCos => {
                    set_top!(format!("vector_sin_cos_rotations_estimate({cached_top})"));
                }
                Opcode::PushConstVec4 => {
                    let index = ptr[1];
                    anyhow::ensure!(index < constants.len() as u8, "Invalid constant index");
                    let c = constants[index as usize];
                    cached_top =
                        self.push(format!("float4({}, {}, {}, {})", c.x, c.y, c.z, c.w))?;
                }
                Opcode::LerpConstant => {
                    let constant_start = ptr[1];
                    ensure!(
                        (constant_start + 1) < constants.len() as u8,
                        "Invalid constant index"
                    );
                    let a = constants[constant_start as usize];
                    let b = constants[(constant_start + 1) as usize];
                    let t = cached_top;

                    cached_top = format!("lerp({a}, {b}, {t})");
                    *self.stack_top() = cached_top.clone();
                }
                Opcode::Spline8Const => {
                    let constant_start = ptr[1];
                    ensure!(
                        (constant_start + 9) < constants.len() as u8,
                        "Invalid constant index"
                    );

                    let cl = &constants[constant_start as usize..];
                    cached_top = format!(
                        "spline8({}, {}, {}, {}, {}, {}, {}, {}, {}, {}, {})",
                        cached_top,
                        cl[0],
                        cl[1],
                        cl[2],
                        cl[3],
                        cl[4],
                        cl[5],
                        cl[6],
                        cl[7],
                        cl[8],
                        cl[9]
                    );
                }
                // Push a temporary value onto the stack
                Opcode::PushTemp => {
                    let slot = ptr[1];
                    anyhow::ensure!(slot < self.temp.len() as u8, "Invalid temp slot");
                    cached_top = self.push(self.temp[slot as usize].clone())?;
                }
                // Pop a temporary value from the stack and store it in the specified temp slot
                Opcode::PopTemp => {
                    let slot = ptr[1];
                    anyhow::ensure!(slot < self.temp.len() as u8, "Invalid temp slot");
                    self.temp[slot as usize] = cached_top;
                    cached_top = self.pop_top();
                }
                Opcode::PopTextureView => {}
                Opcode::PopSamplerState => {}
                Opcode::PopUav => {}
                Opcode::PushSamplerState => {}
                Opcode::PushExternInputFloat => {
                    let extern_id = ExternIndex::try_from(ptr[1])
                        .ok()
                        .context("Invalid extern index")?;
                    let offset = ptr[2];

                    let field = Externs::get_extern_field_name(extern_id, offset as usize * 4)
                        .map(str::to_string)
                        .unwrap_or(format!("0x{:X}", offset as usize * 4));

                    cached_top = self.push(format!("extern<float>({extern_id:?}->{field})"))?;
                }
                Opcode::PushExternInputVec4 => {
                    let extern_id = ExternIndex::try_from(ptr[1])
                        .ok()
                        .context("Invalid extern index")?;
                    let offset = ptr[2];

                    let field = Externs::get_extern_field_name(extern_id, offset as usize * 16)
                        .map(str::to_string)
                        .unwrap_or(format!("0x{:X}", offset as usize * 16));
                    cached_top = self.push(format!("extern<float4>({extern_id:?}->{field})",))?;
                }
                Opcode::PushExternInputMat4 => {
                    let extern_id = ExternIndex::try_from(ptr[1])
                        .ok()
                        .context("Invalid extern index")?;
                    let offset = ptr[2];

                    let field = Externs::get_extern_field_name(extern_id, offset as usize * 16)
                        .map(str::to_string)
                        .unwrap_or(format!("0x{:X}", offset as usize * 16));
                    cached_top = self.push(format!("extern<float4x4>({extern_id:?}->{field})"))?;
                }
                Opcode::PushExternInputTextureView => {}
                Opcode::PushExternInputUav => {}
                Opcode::PushFromOutput => {
                    let element = ptr[1] as usize;
                    anyhow::ensure!(element < out.len(), "Invalid output element index");
                    cached_top = self.push(out[element].clone())?;
                }
                Opcode::PopOutput => {
                    let element = ptr[1] as usize;
                    anyhow::ensure!(element < out.len(), "Invalid output element index");
                    out[element] = cached_top;
                    cached_top = self.pop_top();
                }
                Opcode::PopOutputMat4 => {
                    let start_element = ptr[1] as usize;
                    anyhow::ensure!(
                        (start_element + 3) < out.len(),
                        "Invalid mat4 output starting element index"
                    );

                    out[start_element] = format!("{cached_top}[0]");
                    out[start_element + 1] = format!("{cached_top}[1]");
                    out[start_element + 2] = format!("{cached_top}[2]");
                    out[start_element + 3] = format!("{cached_top}[3]");

                    self.stack_pointer -= 1;
                    cached_top = self.get(0)?;
                }
                Opcode::PushObjectChannelVector => {
                    let channel = u32::from_be_bytes([ptr[1], ptr[2], ptr[3], ptr[4]]);
                    cached_top = self.push(format!("object_channels[{channel:08X}]"))?;
                }
                Opcode::PushGlobalChannelVector => {
                    let channel = ptr[1];
                    cached_top = self.push(format!("global_channels[{channel}]"))?;
                }
                u => {
                    anyhow::bail!("Unimplemented opcode: {u:?} / 0x{:02X}", ptr[0]);
                }
            }

            self.ip += op.size();
        }

        Ok(())
    }
}
