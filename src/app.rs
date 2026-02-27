#![warn(rust_2018_idioms)]
#![deny(clippy::correctness, clippy::suspicious, clippy::complexity)]
#![allow(clippy::collapsible_else_if, clippy::missing_transmute_annotations)]

use std::{
    io::{Cursor, Seek},
    rc::Rc,
    str::FromStr,
    sync::Arc,
    time::Instant,
};

use ahash::HashMap;
use anyhow::Context;
use d3d12::{ResourceBarrier, ResourceStates};
use deimos_core::job::SCHEDULER;
use deimos_data::{
    strings::{StringContainer, StringContainerShared},
    tag::WideHash,
    tfx::{RenderStage, features::terrain::STerrain},
};
use deimos_render::{
    asset::texture::Texture,
    features::{FeatureRenderer, terrain_patches::TerrainPatchesRenderer},
    gpu::{Gpu, command_list::CommandList},
    renderer::{Renderer, globals::get_scope_samplers},
    util::fps_histogram::FrametimeHistogram,
};
use hecs::World;
use parking_lot::RwLock;
use sdl3::video::Window;
use tiger_parse::TigerReadable;
use tiger_pkg::{TagHash, package_manager};

use crate::{cli::AppArgs, config::AppConfig, ui::Gui, world::map::load_map_into_world};

pub struct App {
    pub sdl: Rc<sdl3::Sdl>,
    pub _window: Rc<Window>,
    pub gpu: Arc<Gpu>,
    pub renderer: Arc<Renderer>,
    pub gui: Gui,
    pub running: bool,

    shared_state: Arc<SharedState>,

    // _spinner: FullscreenSpinner,
    last_frame_time: Instant,
    frametime_histogram: FrametimeHistogram,
    map: World,
}

impl App {
    pub fn new(sdl: Rc<sdl3::Sdl>, window: Rc<Window>, args: AppArgs) -> anyhow::Result<Self> {
        let gpu = Arc::new(Gpu::create(&window).context("Failed to create GPU")?);
        let renderer = Arc::new(Renderer::new(gpu.clone()));
        // Renderer::set_instance(renderer.clone());

        let mut gui = Gui::new(&gpu, sdl.clone(), window.clone())?;
        // if let Some(map_hash) = args.open_map.as_ref() {
        //     match TagHash::from_str(map_hash) {
        //         Ok(tag) => match MapTab::new(tag, String::new()) {
        //             Ok(tab) => gui.add_tab(Tab::Map(tab)),
        //             Err(e) => error!("Failed to open map tab for {}: {:?}", map_hash, e),
        //         },
        //         Err(e) => {
        //             error!("Failed to parse map hash {}: {:?}", map_hash, e);
        //         }
        //     };
        // }

        // let mut terrain_temp = vec![];
        // for (tag, _) in package_manager()
        //     .get_all_by_reference(STerrain::ID.unwrap())
        //     .into_iter()
        //     .filter(|(tag, _)| tag.pkg_id() == 0x19a)
        // {
        //     terrain_temp.push(TerrainPatchesRenderer::load(&renderer, tag, 0)?);
        // }

        let mut map = World::new();
        let tag = TagHash(0x80b10471);
        load_map_into_world(&renderer, tag, &mut map).context("Failed to load map")?;

        // if let Err(e) = Technique::load(&gpu, TagHash(0x80AB0C4B)) {
        //     error!("Failed to create technique: {:?}", e);
        // }

        Ok(Self {
            map,
            // _spinner: FullscreenSpinner::create(&renderer.gpu)?,
            renderer,
            gui,
            sdl,
            _window: window,
            gpu,
            running: true,

            shared_state: SharedState::new()
                .context("Failed to create shared state")?
                .into(),

            last_frame_time: Instant::now(),
            frametime_histogram: FrametimeHistogram::new(10),
        })
    }

    pub fn handle_event(&mut self, event: sdl3::event::Event) {
        #[allow(clippy::single_match, clippy::collapsible_match)]
        match &event {
            sdl3::event::Event::Quit { .. } => {
                self.running = false;
            }
            sdl3::event::Event::Window { win_event, .. } => match win_event {
                &sdl3::event::WindowEvent::Resized(new_width, new_height) => {
                    self.gpu.wait_for_idle();
                    self.gpu
                        .swapchain
                        .lock()
                        .resize((new_width as u32, new_height as u32));

                    // self.gui
                    //     .egui_d3d11
                    //     .resize_buffers(&self.renderer.gpu, || {
                    //         self.renderer
                    //             .resize_swapchain((new_width as u32, new_height as u32));
                    //         Ok(())
                    //     })
                    //     .ok();
                }
                sdl3::event::WindowEvent::CloseRequested => {
                    self.running = false;
                }
                _ => {}
            },
            _ => {}
        };

        self.gui
            .egui_sdl3
            .handle_event(&event, &self.sdl, &self.sdl.video().unwrap());
    }

    #[profiling::function]
    pub fn render(&mut self, _event_pump: &sdl3::EventPump) -> anyhow::Result<()> {
        let delta_time = self.last_frame_time.elapsed().as_secs_f32();
        self.last_frame_time = std::time::Instant::now();

        self.frametime_histogram.push(delta_time);

        let frame = self.gpu.begin_frame();

        let cmd = &frame.command_list;
        let mut cmd_tfx =
            CommandList::from_native_command_list(&self.renderer, cmd.command_list.clone());

        self.gui.draw_ui(&self.shared_state);

        cmd.scope(|cmd| {
            let (back_buffer_handle, back_buffer) = self.gpu.swapchain.lock().get_back_buffer();

            cmd.resource_barriers(&[ResourceBarrier::transition(
                &back_buffer,
                0,
                ResourceStates::PRESENT,
                ResourceStates::RENDER_TARGET,
            )]);

            cmd.clear_render_target_view(back_buffer_handle, &[0.0, 0.0, 0.0, 1.0]);
            cmd.om_set_render_targets(&[back_buffer_handle], false, None);

            self.gui.render(&self.gpu, cmd);

            self.renderer.globals.scopes.view.bind(&mut cmd_tfx);
            for obj in self.renderer.objects.write().values_mut() {
                obj.renderer.extract(&self.renderer, &());
                obj.renderer
                    .submit(&mut cmd_tfx, RenderStage::GenerateGbuffer);
            }

            cmd.resource_barriers(&[ResourceBarrier::transition(
                &back_buffer,
                0,
                ResourceStates::RENDER_TARGET,
                ResourceStates::PRESENT,
            )]);

            Ok(())
        })?;

        self.gpu
            .queue
            .execute_command_lists(std::slice::from_ref(cmd));

        self.gpu.end_frame();
        self.gpu.present(self.shared_state.config.read().vsync);

        profiling::finish_frame!();

        Ok(())
    }
}

impl Drop for App {
    fn drop(&mut self) {
        self.shared_state.save_config().ok();
        SCHEDULER.shutdown();
    }
}

pub struct SharedState {
    pub strings: StringContainerShared,
    pub strings_by_package: HashMap<String, StringContainer>,
    pub config: RwLock<AppConfig>,
}

impl SharedState {
    pub fn new() -> anyhow::Result<Self> {
        let mut strings_by_package = HashMap::default();
        for (name, tag) in package_manager().get_named_tags_by_class(0x80808E8B) {
            let Ok(data) = package_manager().read_tag(tag) else {
                continue;
            };
            let mut cur = Cursor::new(data);
            cur.seek(std::io::SeekFrom::Start(0x10))?;
            let hash = WideHash::read_ds(&mut cur)?;
            if hash.is_none() {
                continue;
            }
            strings_by_package.insert(name, StringContainer::load(hash)?);
        }

        let s = Self {
            strings: StringContainer::load_all_global().into(),
            strings_by_package,
            config: RwLock::new(AppConfig::default()),
        };
        if let Err(e) = s.load_config() {
            warn!("Failed to load config: {:?}", e);
        }

        Ok(s)
    }

    pub fn load_config(&self) -> anyhow::Result<()> {
        let exe_path = std::env::current_exe()?.parent().unwrap().to_path_buf();
        let config_path = exe_path.join("config.toml");
        if config_path.exists() {
            let config_str = std::fs::read_to_string(&config_path)?;
            let config: AppConfig = toml::from_str(&config_str)?;
            *self.config.write() = config;
        }

        Ok(())
    }

    pub fn save_config(&self) -> anyhow::Result<()> {
        let exe_path = std::env::current_exe()?.parent().unwrap().to_path_buf();
        let config_path = exe_path.join("config.toml");
        let config_str = toml::to_string_pretty(&*self.config.read())?;
        std::fs::write(&config_path, config_str)?;

        Ok(())
    }

    pub fn get_string(&self, hash: u32) -> String {
        self.strings.get(hash)
    }

    pub fn get_string_by_package(&self, package: &str, hash: u32) -> String {
        self.strings_by_package
            .get(package)
            .and_then(|s| s.try_get(hash))
            .unwrap_or_else(|| self.get_string(hash))
    }
}
