use assert_offset::AssertOffsets;
use glam::{Quat, Vec4};
use tiger_parse::{tiger_tag, tiger_variant_enum, FnvHash, OptionalVariantPointer, Padding};
use tiger_pkg::TagHash;

use crate::tag::{Tag, WideHash, WideTag};

#[derive(Debug, AssertOffsets)]
#[tiger_tag(id = 0x8080AB27, size = 0x50)]
pub struct SBubbleParent {
    pub file_size: u64,

    #[offset(0x8)]
    pub definition: Tag<SBubbleDefinition>,
    pub unkc: Padding<4>,

    pub unk10: u64,
    pub map_name: FnvHash,
}

#[derive(Debug, AssertOffsets)]
#[tiger_tag(id = 0x8080A7BB, size = 0x50)]
pub struct SBubbleDefinition {
    pub file_size: u64,
    pub containers: Vec<WideTag<SMapContainer>>,
}

#[derive(Debug)]
#[tiger_tag(id = 0x8080A7C1, size = 0x38)]
pub struct SMapContainer {
    pub file_size: u64,
    #[tag(offset = 0x28)]
    pub data_tables: Vec<TagHash>,
}

#[tiger_tag(id = 0x8080B1A7)]
pub struct SMapNodeTable {
    pub file_size: u64,
    pub data_entries: Vec<SMapNodeEntry>,
}

#[tiger_tag(id = 0x8080B3F5)]
pub struct SMapNodeEntry {
    pub rotation: Quat,
    pub translation: Vec4,
    pub entity_old: TagHash,
    pub unk24: u32,
    pub entity: WideHash,
    pub unk38: [u32; 9], //
    pub unk5c: f32,
    pub unk60: f32,
    pub unk64: TagHash,
    pub unk68: FnvHash,
    pub unk6c: u32,
    pub world_id: u64,
    pub data_resource: OptionalVariantPointer<MapNodeResource>,
    pub unk80: [u32; 4],
}

tiger_variant_enum! {
    [Unknown(true)]
    enum MapNodeResource {
        SStaticTerrainPatchesComponent,
        SStaticInstancesCollectionComponent
    }
}

// #[tiger_tag(id = 0x80806F38)]
// pub struct SStaticAmbientOcclusionComponent {
//     pub ao: Tag<SStaticAmbientOcclusion>,
// }

#[tiger_tag(id = 0x808085B0)]
pub struct SStaticInstancesCollectionComponent {
    pub instances: TagHash,
}

// #[tiger_tag(id = 0x80806F91)]
// pub struct SSkyObjectCollectionComponent {
//     pub objects: Tag<SSkyObjectCollection>,
// }

// #[tiger_tag(id = 0x80806F5A)]
// pub struct SLightCollectionComponent {
//     pub lights: Tag<SLightCollection>,
// }

// #[tiger_tag(id = 0x80806DE0)]
// pub struct SWaterPlaneComponent {
//     pub model: TagHash,
// }

// #[tiger_tag(id = 0x80807133)]
// pub struct SShadowingLightComponent {
//     pub light: Tag<SShadowingLight>,
// }

// #[tiger_tag(id = 0x80806E62)]
// pub struct SDecalCollectionComponent {
//     pub decals: TagHash,
// }

#[derive(Clone, Debug)]
#[tiger_tag(id = 0x80808563)]
pub struct SStaticTerrainPatchesComponent {
    pub identifier: u64,
    pub terrain: TagHash,
    pub terrain_bounds: TagHash,
}
