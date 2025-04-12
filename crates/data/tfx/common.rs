use glam::Vec4;
use tiger_parse::tiger_tag;

#[tiger_tag]
#[derive(Debug, Clone)]
pub struct AxisAlignedBBox {
    pub min: Vec4,
    pub max: Vec4,
}
