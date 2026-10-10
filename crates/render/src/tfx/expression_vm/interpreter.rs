//! Renderer bindings for Deimos's shared CPU evaluator.
use crate::tfx::externs::{ExternAccessor, ExternAccessorExt};
use ahash::AHashMap;
use anyhow::Context;
use deimos_data::tfx::ExternIndex;
use deimos_ecs::object::ObjectChannel;
use deimos_tfx::{Inputs, InterpreterState as CpuState};
use glam::{Mat4, Vec4};

struct RendererInputs<'a> {
    externs: Option<&'a dyn ExternAccessor>,
    channels: Option<&'a AHashMap<u32, ObjectChannel>>,
}
impl Inputs for RendererInputs<'_> {
    fn float(&self, index: ExternIndex, offset: usize) -> anyhow::Result<f32> {
        self.externs
            .context("Missing renderer externs")?
            .get_extern_value(index, offset)
            .context("Missing float extern")
    }
    fn vector(&self, index: ExternIndex, offset: usize) -> anyhow::Result<Vec4> {
        self.externs
            .context("Missing renderer externs")?
            .get_extern_value(index, offset)
            .context("Missing vector extern")
    }
    fn matrix(&self, index: ExternIndex, offset: usize) -> anyhow::Result<Mat4> {
        self.externs
            .context("Missing renderer externs")?
            .get_extern_value(index, offset)
            .context("Missing matrix extern")
    }
    fn global_channel(&self, index: u8) -> anyhow::Result<Vec4> {
        Ok(self
            .externs
            .context("Missing renderer externs")?
            .get_global_channel(index))
    }
    fn object_channel(&self, hash: u32) -> anyhow::Result<Vec4> {
        let channel = self
            .channels
            .context("Missing object channels")?
            .get(&hash)
            .context("Missing object channel")?;
        channel
            .usage
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        Ok(channel.value)
    }
}
pub struct InterpreterState<'a> {
    data: &'a [u8],
    pub ip: usize,
    inputs: RendererInputs<'a>,
    debug: bool,
}
impl<'a> InterpreterState<'a> {
    pub fn new(data: &'a [u8]) -> Self {
        Self {
            data,
            ip: 0,
            inputs: RendererInputs {
                externs: None,
                channels: None,
            },
            debug: false,
        }
    }
    pub fn with_externs(mut self, externs: &'a dyn ExternAccessor) -> Self {
        self.inputs.externs = Some(externs);
        self
    }
    pub fn with_object_channels(mut self, channels: &'a AHashMap<u32, ObjectChannel>) -> Self {
        self.inputs.channels = Some(channels);
        self
    }
    pub fn with_debug(mut self, debug: bool) -> Self {
        self.debug = debug;
        self
    }
    pub fn evaluate(&mut self, constants: &[Vec4], out: &mut [Vec4]) -> anyhow::Result<()> {
        let mut state = CpuState::new(self.data)
            .with_inputs(&self.inputs)
            .with_debug(self.debug);
        let result = state.evaluate(constants, out);
        self.ip = state.ip;
        result
    }
}
