use std::ops::{BitOr, BitOrAssign, Range};

use tiger_parse::tiger_tag;
use tiger_pkg::TagHash;

use crate::tfx::{LodCategory, PrimitiveType, RenderStage};

#[derive(Debug, Clone)]
#[tiger_tag(id = 0x8080881C, size = 0x70)]
pub struct SDynamicModel {
    pub file_size: u64,
    pub unk8: u64,
    pub meshes: Vec<SDynamicMesh>,
    pub unk20: glam::Vec4,
    #[tag(offset = 0xA0)]
    pub model_scale: glam::Vec4,
    pub model_offset: glam::Vec4,
    pub texcoord_scale: glam::Vec2,
    pub texcoord_offset: glam::Vec2,
}

#[derive(Debug, Clone)]
#[tiger_tag(id = 0x808087CB, size = 0x88)]
pub struct SDynamicMesh {
    pub vertex0_buffer: TagHash,
    pub vertex1_buffer: TagHash,
    pub buffer2: TagHash,
    pub buffer3: TagHash,
    pub index_buffer: TagHash,
    pub color_buffer: TagHash,
    pub skinning_buffer: TagHash,
    pub unk1c: u32,
    pub parts: Vec<SDynamicMeshPart>,
    /// Range of parts to render per render stage
    /// Can be obtained as follows:
    ///     - Start = part_range_per_render_stage[stage]
    ///     - End = part_range_per_render_stage[stage + 1]
    pub part_range_per_render_stage: [u16; RenderStage::COUNT + 1],
    pub input_layout_per_render_stage: [u8; RenderStage::COUNT],
    _pad7a: [u16; 3],
}

impl SDynamicMesh {
    pub fn get_range_for_stage(&self, stage: RenderStage) -> Range<usize> {
        let start = self.part_range_per_render_stage[stage as usize];
        let end = self.part_range_per_render_stage[stage as usize + 1];
        start as usize..end as usize
    }

    pub fn get_input_layout_for_stage(&self, stage: RenderStage) -> u8 {
        self.input_layout_per_render_stage[stage as usize]
    }
}

#[derive(Debug, Clone)]
#[tiger_tag(id = 0x808087D1)]
pub struct SDynamicMeshPart {
    pub technique: TagHash,
    pub variant_shader_index: u16,
    pub primitive_type: PrimitiveType,
    pub unk7: u8,
    // 0x8
    pub index_start: u32,
    pub index_count: u32,

    // 0x10
    pub unk10: u32,
    pub external_identifier: u16,
    pub unk16: u16,
    // 0x18
    pub flags: u32,

    // 0x20
    pub unk1c: u16,
    // Running increment, +1 per part
    pub unk1e: u16,

    // 0x20
    pub unk20: u8,
    pub lod_category: LodCategory,
    pub unk22: u16,
    pub unk24: u32,
}

#[derive(Debug, Clone)]
#[tiger_tag(id = 0x80806D97)]
pub struct SDynamicMeshMaterialVariants {
    pub technique_count: u32,
    pub technique_start: u32,
    pub unk8: u32,
}

bitflags::bitflags! {
    #[derive(Debug, Copy, Clone)]
    pub struct RenderStageSubscription: u32 {
        const GENERATE_GBUFFER                  = 1 << RenderStage::GenerateGbuffer as u32;
        const DECALS                            = 1 << RenderStage::Decals as u32;
        const INVESTMENT_DECALS                 = 1 << RenderStage::InvestmentDecals as u32;
        const SHADOW_GENERATE                   = 1 << RenderStage::ShadowGenerate as u32;
        const LIGHTING_APPLY                    = 1 << RenderStage::LightingApply as u32;
        const LIGHT_PROBE_APPLY                 = 1 << RenderStage::LightProbeApply as u32;
        const DECALS_ADDITIVE                   = 1 << RenderStage::DecalsAdditive as u32;
        const TRANSPARENTS                      = 1 << RenderStage::Transparents as u32;
        const DISTORTION                        = 1 << RenderStage::Distortion as u32;
        const LIGHT_SHAFT_OCCLUSION             = 1 << RenderStage::LightShaftOcclusion as u32;
        const SKIN_PREPASS                      = 1 << RenderStage::SkinPrepass as u32;
        const LENS_FLARES                       = 1 << RenderStage::LensFlares as u32;
        const DEPTH_PREPASS                     = 1 << RenderStage::DepthPrepass as u32;
        const WATER_REFLECTION                  = 1 << RenderStage::WaterReflection as u32;
        const POSTPROCESS_TRANSPARENT_STENCIL   = 1 << RenderStage::PostprocessTransparentStencil as u32;
        const IMPULSE                           = 1 << RenderStage::Impulse as u32;
        const RETICLE                           = 1 << RenderStage::Reticle as u32;
        const WATER_RIPPLES                     = 1 << RenderStage::WaterRipples as u32;
        const MASK_SUN_LIGHT                    = 1 << RenderStage::MaskSunLight as u32;
        const VOLUMETRICS                       = 1 << RenderStage::Volumetrics as u32;
        const CUBEMAPS                          = 1 << RenderStage::Cubemaps as u32;
        const POSTPROCESS_SCREEN                = 1 << RenderStage::PostprocessScreen as u32;
        const WORLD_FORCES                      = 1 << RenderStage::WorldForces as u32;
        const COMPUTE_SKINNING                  = 1 << RenderStage::ComputeSkinning as u32;
    }
}

impl RenderStageSubscription {
    pub fn is_subscribed(&self, stage: RenderStage) -> bool {
        self.contains(Self::from_bits_truncate(1 << stage as u32))
    }

    pub fn from_partrange_list(part_ranges: &[u16]) -> Self {
        let mut flags = Self::empty();
        for i in 0..RenderStage::COUNT {
            if part_ranges[i] != part_ranges[i + 1] {
                flags |= Self::from_bits_truncate(1 << i);
            }
        }
        flags
    }
}

impl From<RenderStage> for RenderStageSubscription {
    fn from(stage: RenderStage) -> Self {
        Self::from_bits_truncate(1 << stage as u32)
    }
}

impl BitOr<RenderStage> for RenderStageSubscription {
    type Output = Self;

    fn bitor(self, rhs: RenderStage) -> Self::Output {
        self | Self::from(rhs)
    }
}

impl BitOrAssign<RenderStage> for RenderStageSubscription {
    fn bitor_assign(&mut self, rhs: RenderStage) {
        *self |= Self::from(rhs);
    }
}
