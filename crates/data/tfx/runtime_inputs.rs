//! Shared offsets/defaults from the owning renderer extern definitions.
#[repr(usize)]
pub enum FrameFloat {
    GameTime = 0x00,
}
#[repr(usize)]
pub enum FrameVector {
    Unk280 = 0x280,
}
#[repr(usize)]
pub enum DeferredVector {
    GbufferResolutionScaleOffset = 0x40,
}
pub const FRAME_UNK280_DEFAULT: glam::Vec4 = glam::Vec4::ONE;
