pub mod debug;
pub mod globals;
pub mod submit;
pub mod surface;
pub mod util;

use std::sync::{Arc, OnceLock};

use anyhow::Context;
use crossbeam::atomic::AtomicCell;
use d3d11::dxgi;
use deimos_core::ConVars;
use deimos_data::tfx::FeatureRendererSubscription;
use globals::RenderGlobals;
use parking_lot::{Mutex, RwLock};
use submit::buffers::{Gbuffers, LightBuffers, WaterBuffers};
use surface::{SizeRelativity, SurfaceDesc, SurfaceHandle, SurfaceProxy, Surfaces};

use crate::{
    asset::{texture::Texture, AssetManager},
    feature::immediate::ImmediateShapeRenderer,
    gpu::{cbuffer::ConstantBuffer, debug_text::DebugTextRenderer},
    object::{RenderObject, RenderObjectHandle},
    tfx::{externs::Externs, packet::FramePacket, scope::TempFrameScope},
    util::{arena::Arena, threading::ThreadMutCell},
    Gpu,
};

const DEBUG_SHADER: &str = include_str!("../builtin/shaders/debug.hlsl");
const CLEAR_AO_SHADER: &str = include_str!("../builtin/shaders/clear_ao.hlsl");
const BLIT_SHADER: &str = include_str!("../builtin/shaders/blit_srgb.hlsl");
const BLIT_FAKE_WEAPON_SHADER: &str = include_str!("../builtin/shaders/blit_fake_weapon.hlsl");

pub struct Renderer {
    pub gpu: Arc<Gpu>,
    pub asset_manager: AssetManager,
    // pub timestamps: TimestampManager,
    pub surfaces: Surfaces,
    pub immediate: Mutex<ImmediateShapeRenderer>,
    pub debug_text: Mutex<DebugTextRenderer>,
    pub externs: ThreadMutCell<Externs>,

    pub objects: RwLock<Arena<RenderObject>>,
    pub frame_packet: RwLock<FramePacket<'static>>,
    pub globals: RenderGlobals,

    // pub ao: RwLock<Option<SStaticAmbientOcclusion>>,
    // pub ao_buffer: Mutex<Option<Handle<VertexBuffer>>>,
    submit_jobs: submit::lowlevel::SubmitJobManager,

    gbuffers: Gbuffers,
    lighting: LightBuffers,
    water: WaterBuffers,

    frame_scope: ConstantBuffer<TempFrameScope>,
    shading_result: SurfaceHandle,
    shading_result_read: Mutex<SurfaceProxy>,

    debug_vs: d3d11::VertexShader,
    debug_ps: d3d11::PixelShader,
    clear_ao_vs: d3d11::VertexShader,
    clear_ao_ps: d3d11::PixelShader,

    common: CommonResources,
    active_feature_renderers: AtomicCell<FeatureRendererSubscription>,
}

unsafe impl Send for Renderer {}
unsafe impl Sync for Renderer {}

static RENDERER_GLOBAL: OnceLock<Arc<Renderer>> = OnceLock::new();
impl Renderer {
    pub fn new(gpu: Arc<Gpu>, swapchain_resolution: (u32, u32)) -> anyhow::Result<Self> {
        ConVars::register("render.sky", true);
        ConVars::register("render.global_lighting", false);
        ConVars::register("render.threaded_submit", true);
        ConVars::register("render.patch_light_shader", false);
        ConVars::register("render.vertex_ao_workaround", true);
        ConVars::register("render.vao_buffer", false);
        ConVars::register("render.ssao", true);

        ConVars::register("render.feature.static_objects", true);
        ConVars::register("render.feature.rigid_objects", true);
        ConVars::register("render.feature.chunked_lights", true);
        ConVars::register("render.feature.deferred_lights", true);
        ConVars::register("render.feature.sky_transparent", true);
        ConVars::register("render.feature.decals", true);
        ConVars::register("render.feature.dynamic_decals", true);
        ConVars::register("render.feature.road_decals", true);
        ConVars::register("render.feature.water", true);
        ConVars::register("render.feature.volumetrics", true);
        ConVars::register("render.feature.cubemaps", false);

        let surfaces = Surfaces::new(gpu.device.clone(), swapchain_resolution);

        // TODO(move this to gbuffer?)
        let shading_result = surfaces.create_surface(
            swapchain_resolution,
            SurfaceDesc::builder("shading_result", SizeRelativity::RelativeToFramebuffer)
                .format(dxgi::Format::R11g11b10Float)
                .build(),
        )?;

        let (debug_vs, debug_ps) =
            gpu.compile_shader_vs_ps("debug", DEBUG_SHADER, "mainVS", "mainPS")?;

        let (clear_ao_vs, clear_ao_ps) =
            gpu.compile_shader_vs_ps("clear_ao", CLEAR_AO_SHADER, "mainVS", "mainPS")?;

        Ok(Self {
            globals: RenderGlobals::load(&gpu).context("Failed to load render globals")?,
            // timestamps: TimestampManager::new(&gpu.device)?,
            asset_manager: AssetManager::new(&gpu),
            debug_text: Mutex::new(DebugTextRenderer::create(&gpu)?),
            immediate: Mutex::new(
                ImmediateShapeRenderer::new(&gpu).context("Failed to create immediate renderer")?,
            ),
            externs: ThreadMutCell::new(Externs::default()),
            objects: RwLock::new(Arena::new()),
            frame_packet: RwLock::new(FramePacket::default()),
            // ao: RwLock::new(None),
            // ao_buffer: Mutex::new(None),
            submit_jobs: submit::lowlevel::SubmitJobManager::new(4),

            shading_result_read: Mutex::new(
                SurfaceProxy::new(&gpu, surfaces.get(shading_result), None, false)
                    .context("Failed to create shading_result_read surface proxy")?,
            ),
            shading_result,
            frame_scope: ConstantBuffer::create(&gpu, None)?,

            gbuffers: Gbuffers::create(&gpu, &surfaces, swapchain_resolution)?,
            lighting: LightBuffers::create(&surfaces, swapchain_resolution)?,
            water: WaterBuffers::create(&surfaces, swapchain_resolution)?,

            debug_vs,
            debug_ps,
            clear_ao_vs,
            clear_ao_ps,

            common: CommonResources::load(&gpu)?,

            gpu,
            surfaces,
            active_feature_renderers: AtomicCell::new(FeatureRendererSubscription::all()),
        })
    }

    pub fn set_instance(renderer: Arc<Self>) {
        if RENDERER_GLOBAL.set(renderer).is_err() {
            panic!("GPU is already initialized!");
        }
    }

    pub fn is_initialized() -> bool {
        RENDERER_GLOBAL.get().is_some()
    }

    pub fn instance() -> &'static Renderer {
        RENDERER_GLOBAL.get().expect("GPU is not yet initialized!")
    }

    pub fn add_object(&self, object: RenderObject) -> RenderObjectHandle {
        RenderObjectHandle(self.objects.write().insert(object))
    }

    pub fn clone_object(&self, handle: RenderObjectHandle) -> Option<RenderObjectHandle> {
        let objects = self.objects.read();
        let object = objects.get(handle.into()).and_then(|o| o.dyn_clone())?;
        drop(objects);

        Some(RenderObjectHandle(self.objects.write().insert(object)))
    }

    pub fn remove_object(&self, handle: RenderObjectHandle) {
        let mut objects = self.objects.write();
        objects.remove(handle.into());
    }

    pub fn shutdown(&self) {
        self.asset_manager.shutdown();
    }

    pub fn resize_swapchain(&self, resolution: (u32, u32)) {
        self.surfaces.resize_surfaces(resolution);
        self.gpu.resize_swapchain(resolution);
    }
}

impl Renderer {
    pub fn begin_frame(&self) {
        // self.timestamps.begin_frame();
        self.asset_manager.remove_unreferenced();
    }

    pub fn present_frame(&self, vsync: bool) {
        self.gpu.present(vsync);
        // self.timestamps.collect();
        self.debug_text.lock().clear();
    }
}

pub struct CommonResources {
    default_lut: Texture,

    blit_vs: d3d11::VertexShader,
    blit_ps: d3d11::PixelShader,
    blit_ps_linear: d3d11::PixelShader,

    blit_fw_vs: d3d11::VertexShader,
    blit_fw_ps: d3d11::PixelShader,

    temporary_sky_hemisphere: Texture,
    temporary_vignette: Texture,
    temporary_health_overlay: Texture,
    temporary_bloom: Texture,

    temporary_atmos: Texture,
    temporary_depth_angle_lookup: Texture,
    temporary_depth_lookup: Texture,
}

impl CommonResources {
    pub fn load(gpu: &Gpu) -> anyhow::Result<Self> {
        let mut default_lut_data = vec![];
        for z in 0..32 {
            for y in 0..32 {
                for x in 0..32 {
                    let r = x as f32 / 31.0;
                    let g = y as f32 / 31.0;
                    let b = z as f32 / 31.0;
                    default_lut_data.push((r * 255.0) as u8);
                    default_lut_data.push((g * 255.0) as u8);
                    default_lut_data.push((b * 255.0) as u8);
                    default_lut_data.push(255);
                }
            }
        }

        let default_lut = Texture::load_3d_raw(
            gpu,
            32,
            32,
            32,
            &default_lut_data,
            dxgi::Format::R8g8b8a8Unorm,
            Some("lut3d_temp"),
        )?;

        let (blit_vs, blit_ps) =
            gpu.compile_shader_vs_ps("blit", BLIT_SHADER, "mainVS", "mainPS")?;
        let (_, blit_ps_linear) =
            gpu.compile_shader_vs_ps("blit", BLIT_SHADER, "mainVS", "mainPS_linear")?;

        let (blit_fw_vs, blit_fw_ps) =
            gpu.compile_shader_vs_ps("blit_fw", BLIT_FAKE_WEAPON_SHADER, "mainVS", "mainPS")?;

        Ok(Self {
            temporary_sky_hemisphere: Texture::load_2d_dds(
                gpu,
                include_bytes!("../builtin/textures/sky_hemisphere_divalian.dds"),
            )?,
            default_lut,
            temporary_vignette: Texture::load_2d_dds(
                gpu,
                include_bytes!("../builtin/textures/vignette.dds"),
            )?,
            temporary_health_overlay: Texture::load_2d_dds(
                gpu,
                include_bytes!("../builtin/textures/health_overlay.dds"),
            )?,
            temporary_bloom: Texture::load_2d_dds(
                gpu,
                include_bytes!("../builtin/textures/screen_area_0x18.dds"),
            )?,
            temporary_atmos: Texture::load_2d_dds(
                gpu,
                include_bytes!("../builtin/textures/atmos0.dds"),
            )?,
            temporary_depth_angle_lookup: Texture::load_2d_dds(
                gpu,
                include_bytes!("../builtin/textures/depth_angle_lookup.dds"),
            )?,
            temporary_depth_lookup: Texture::load_2d_dds(
                gpu,
                include_bytes!("../builtin/textures/depth_lookup.dds"),
            )?,
            blit_vs,
            blit_ps,
            blit_ps_linear,
            blit_fw_vs,
            blit_fw_ps,
        })
    }
}
