use std::sync::Arc;

use d3d11::dxgi;
use glam::{Mat4, Vec3};

use crate::{
    renderer::{
        submit::buffers::Gbuffers,
        surface::{SizeRelativity, SurfaceDesc, SurfaceHandle, Surfaces},
    },
    Gpu,
};

pub struct View {
    pub(crate) position: Vec3,
    pub(crate) world_to_camera: Mat4,
    pub(crate) camera_to_projective: Mat4,

    pub(crate) resolution: (u32, u32),
    pub(crate) surfaces: Arc<Surfaces>,
    pub(crate) gbuffers: Gbuffers,
    pub(crate) shading_result: SurfaceHandle,
}

impl View {
    pub fn new(gpu: &Gpu, resolution: (u32, u32)) -> anyhow::Result<Self> {
        let surfaces = Arc::new(Surfaces::new(gpu.device.clone(), resolution));
        let gbuffers = Gbuffers::create(gpu, &surfaces, resolution)?;

        let shading_result = surfaces.create_surface(
            resolution,
            SurfaceDesc::builder("shading_result", SizeRelativity::RelativeToFramebuffer)
                .format(dxgi::Format::R11g11b10Float)
                .build(),
        )?;

        Ok(Self {
            position: Vec3::ZERO,
            world_to_camera: Mat4::IDENTITY,
            camera_to_projective: Mat4::IDENTITY,
            resolution,
            surfaces,
            gbuffers,
            shading_result,
        })
    }

    pub fn update(&mut self, world_to_camera: Mat4, camera_to_projective: Mat4) {
        self.world_to_camera = world_to_camera;
        self.camera_to_projective = camera_to_projective;
        self.position = self.world_to_camera.inverse().transform_point3(Vec3::ZERO);
    }
}
