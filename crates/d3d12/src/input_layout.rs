use bon::Builder;
use windows::Win32::Graphics::Direct3D12::*;

use crate::Format;

#[derive(Builder, Clone)]
pub struct InputElementDesc {
    #[builder(into)]
    pub semantic_name: String,
    pub semantic_index: u32,
    pub format: Format,
    #[builder(default = 0)]
    pub input_slot: u32,

    #[builder(default)]
    pub aligned_byte_offset: ElementOffset,
    #[builder(default = InputClassification::PerVertexData)]
    pub input_slot_class: InputClassification,
    #[builder(default)]
    pub instance_data_step_rate: u32,
}

#[derive(Clone, Default)]
pub enum ElementOffset {
    /// Element will be appended right after the previous one
    #[default]
    Append,
    /// Element will be placed at the given absolute offset, relative to the start of the vertex
    Absolute(u32),
}

#[repr(i32)]
#[derive(Clone)]
pub enum InputClassification {
    PerVertexData = D3D12_INPUT_CLASSIFICATION_PER_VERTEX_DATA.0,
    PerInstanceData = D3D12_INPUT_CLASSIFICATION_PER_INSTANCE_DATA.0,
}

impl From<InputClassification> for D3D12_INPUT_CLASSIFICATION {
    fn from(classification: InputClassification) -> Self {
        Self(classification as i32)
    }
}
