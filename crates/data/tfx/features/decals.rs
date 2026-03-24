use tiger_parse::{tiger_type, Padding};
use tiger_pkg::TagHash;

use crate::{
    tag::TagRef,
    tfx::{common::SOcclusionBounds, geometry::AxisAlignedBBox, RenderStage},
};

#[derive(Clone, Debug)]
#[tiger_type(id = 0x80808224)]
pub struct SDecalCollection {
    pub file_size: u64,
    pub decals: Vec<SDecalSet>,
    pub unk18: Vec<()>,
    pub vb0: TagHash,
    pub vb1: TagHash,
    pub unk30: u32,
    pub unk34: u16,
    pub render_stage: RenderStage,
    _pad37: Padding<1>,
    pub decal_bounds: TagRef<SOcclusionBounds>,
    pub unk3c: u32,
    pub bounds: AxisAlignedBBox,
}

#[derive(Clone, Debug)]
#[tiger_type(id = 0x8080822C, size = 0xC)]
pub struct SDecalSet {
    pub technique: TagHash,
    pub start: u16,
    pub count: u16,
    pub unk8: u32,
}
