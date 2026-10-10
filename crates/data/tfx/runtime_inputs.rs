//! Shared offsets/defaults from the owning renderer extern definitions.
#[repr(usize)]
pub enum FrameFloat {
    GameTime = 0x00,
    /// Viewer-owned normalized day phase; original scene producer divides its
    /// 0..3600 environment clock by 3600 (src/ui/scene/mod.rs).
    DayPhase = 0x10,
}
#[repr(usize)]
pub enum FrameVector {
    Unk1e0 = 0x1E0,
    Unk280 = 0x280,
}
#[repr(usize)]
pub enum DeferredVector {
    GbufferResolutionScaleOffset = 0x40,
}
pub const FRAME_UNK280_DEFAULT: glam::Vec4 = glam::Vec4::ONE;
pub const FRAME_UNK1E0_DEFAULT: glam::Vec4 = glam::Vec4::ZERO;
