use windows::Win32::Graphics::Direct3D::*;

#[repr(i32)]
#[derive(Clone, Copy, Debug)]
pub enum PrimitiveTopology {
    PointList = D3D_PRIMITIVE_TOPOLOGY_POINTLIST.0,
    LineList = D3D_PRIMITIVE_TOPOLOGY_LINELIST.0,
    LineStrip = D3D_PRIMITIVE_TOPOLOGY_LINESTRIP.0,
    TriangleList = D3D_PRIMITIVE_TOPOLOGY_TRIANGLELIST.0,
    TriangleStrip = D3D_PRIMITIVE_TOPOLOGY_TRIANGLESTRIP.0,
    LineListAdj = D3D_PRIMITIVE_TOPOLOGY_LINELIST_ADJ.0,
    LineStripAdj = D3D_PRIMITIVE_TOPOLOGY_LINESTRIP_ADJ.0,
    TriangleListAdj = D3D_PRIMITIVE_TOPOLOGY_TRIANGLELIST_ADJ.0,
    TriangleStripAdj = D3D_PRIMITIVE_TOPOLOGY_TRIANGLESTRIP_ADJ.0,
    PatchList = D3D_PRIMITIVE_TOPOLOGY_1_CONTROL_POINT_PATCHLIST.0,
}

impl From<PrimitiveTopology> for D3D_PRIMITIVE_TOPOLOGY {
    fn from(topology: PrimitiveTopology) -> Self {
        Self(topology as i32)
    }
}
