//! Original technique constant buffers, evaluated by the shared Deimos VM.
use crate::{Installation, read_at};
use anyhow::{Context, Result, ensure};
use deimos_data::tfx::{STechnique, ShaderStage};
use deimos_tfx::{Inputs, InterpreterState, constant_bytecode};
use glam::Vec4;

#[derive(Clone, Copy, Default)]
pub struct ShaderDependencies {
    pub time: bool,
    pub viewport: bool,
}
pub struct ShaderParameters {
    pub slot: i32,
    pub initial: Vec<Vec4>,
    bytecode: Vec<u8>,
    constants: Vec<Vec4>,
}
impl ShaderParameters {
    pub fn dependencies(&self) -> Result<ShaderDependencies> {
        use deimos_data::tfx::{
            ExternIndex,
            opcodes::{Opcode, OpcodeIterator},
            runtime_inputs::{DeferredVector, FrameFloat},
        };
        let mut dependencies = ShaderDependencies::default();
        for instruction in OpcodeIterator::new(&self.bytecode) {
            let (opcode, args) = instruction?;
            match opcode {
                Opcode::PushExternInputFloat
                    if args[0] == ExternIndex::Frame as u8
                        && args[1] as usize * size_of::<f32>() == FrameFloat::GameTime as usize =>
                {
                    dependencies.time = true
                }
                Opcode::PushExternInputVec4
                    if args[0] == ExternIndex::Deferred as u8
                        && args[1] as usize * size_of::<Vec4>()
                            == DeferredVector::GbufferResolutionScaleOffset as usize =>
                {
                    dependencies.viewport = true
                }
                _ => {}
            }
        }
        Ok(dependencies)
    }

    pub fn evaluate(&self, inputs: &dyn Inputs) -> Result<Vec<Vec4>> {
        let mut output = self.initial.clone();
        InterpreterState::new(&self.bytecode)
            .with_inputs(inputs)
            .evaluate(&self.constants, &mut output)?;
        Ok(output)
    }
}
impl Installation {
    pub fn shader_samplers(
        &self,
        tag: u32,
        stage: ShaderStage,
    ) -> Result<Vec<(u32, deimos_data::tfx::SSamplerData)>> {
        let technique: STechnique = self.read_type(tag)?;
        let core = &technique
            .all_valid_shaders()
            .into_iter()
            .find(|(kind, _)| *kind == stage)
            .context("Technique has no requested shader stage")?
            .1
            .core;
        core.sampler_assignments()?
            .into_iter()
            .map(|(slot, tag)| Ok((slot, self.read_type(self.reference(tag.0)?)?)))
            .collect()
    }

    pub fn shader_parameters(&self, tag: u32, stage: ShaderStage) -> Result<ShaderParameters> {
        let technique: STechnique = self.read_type(tag)?;
        let core = &technique
            .all_valid_shaders()
            .into_iter()
            .find(|(kind, _)| *kind == stage)
            .context("Technique has no requested shader stage")?
            .1
            .core;
        let initial = if core.constant_buffer.is_some() {
            let bytes = self.read(self.reference(core.constant_buffer.0)?)?;
            ensure!(
                bytes.len() % size_of::<Vec4>() == 0,
                "Material constant payload is not a vector array"
            );
            (0..bytes.len())
                .step_by(size_of::<Vec4>())
                .map(|offset| read_at(&bytes, offset as u64))
                .collect::<Result<Vec<_>>>()?
        } else {
            core.unk30.clone()
        };
        Ok(ShaderParameters {
            slot: core.constant_buffer_slot,
            initial,
            bytecode: constant_bytecode(&core.bytecode)?,
            constants: core.bytecode_constants.clone(),
        })
    }
}
