use tiger_parse::{tiger_tag, NullString, Pointer};

use super::{SDynamicConstants, ShaderStage};

#[derive(Clone)]
#[tiger_tag(id = 0x808031DC, size = 0x3b8)]
pub struct SScope {
    pub file_size: u64,
    pub name: Pointer<NullString>,

    #[tag(offset = 0x48)]
    pub stage_pixel: SScopeStage,
    pub stage_vertex: SScopeStage,
    pub stage_geometry: SScopeStage,
    pub stage_hull: SScopeStage,
    pub stage_compute: SScopeStage,
    pub stage_domain: SScopeStage,
}

impl SScope {
    pub fn iter_stages(&self) -> impl Iterator<Item = (&SScopeStage, ShaderStage)> {
        vec![
            (&self.stage_pixel, ShaderStage::Pixel),
            (&self.stage_vertex, ShaderStage::Vertex),
            (&self.stage_geometry, ShaderStage::Geometry),
            (&self.stage_hull, ShaderStage::Hull),
            (&self.stage_compute, ShaderStage::Compute),
            (&self.stage_domain, ShaderStage::Domain),
        ]
        .into_iter()
    }
}

#[derive(Clone)]
#[tiger_tag(id = 0xffffffff, size = 0x80)]
pub struct SScopeStage {
    pub constants: SDynamicConstants,
}
