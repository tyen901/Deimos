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
};
use deimos_render::{gpu::Gpu, renderer::Renderer, util::fps_histogram::FrametimeHistogram};
use parking_lot::RwLock;
use sdl3::video::Window;
use tiger_parse::TigerReadable;
use tiger_pkg::{TagHash, package_manager};

use crate::{
    cli::AppArgs,
    config::AppConfig,
    ui::{
        Gui,
        tabs::{Tab, map::MapTab},
    },
};

pub struct App {
    pub sdl: Rc<sdl3::Sdl>,
    pub window: Rc<Window>,
    pub gpu: Arc<Gpu>,
    pub renderer: Arc<Renderer>,
    pub gui: Gui,
    pub running: bool,

    shared_state: Arc<SharedState>,

    // _spinner: FullscreenSpinner,
    last_frame_time: Instant,
    frametime_histogram: FrametimeHistogram,
}

impl App {
    pub fn new(sdl: Rc<sdl3::Sdl>, window: Rc<Window>, args: AppArgs) -> anyhow::Result<Self> {
        let gpu = Arc::new(Gpu::create(&window).context("Failed to create GPU")?);
        let renderer = Arc::new(Renderer::new(gpu.clone()));
        *gpu.extern_source.write() =
            deimos_render::tfx::externs::BaseExternSource::Renderer(renderer.clone());
        // Renderer::set_instance(renderer.clone());

        let mut gui = Gui::new(&gpu, sdl.clone(), window.clone())?;
        if let Some(map_hash) = args.open_map.as_ref() {
            match TagHash::from_str(map_hash) {
                Ok(tag) => match MapTab::new(&renderer, tag, String::new()) {
                    Ok(tab) => gui.add_tab(Tab::Map(tab)),
                    Err(e) => error!("Failed to open map tab for {}: {:?}", map_hash, e),
                },
                Err(e) => {
                    error!("Failed to parse map hash {}: {:?}", map_hash, e);
                }
            };
        }

        Ok(Self {
            // _spinner: FullscreenSpinner::create(&renderer.gpu)?,
            shared_state: SharedState::new(renderer.clone())
                .context("Failed to create shared state")?
                .into(),
            renderer,
            gui,
            sdl,
            window,
            gpu,
            running: true,

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

        self.renderer.asset_manager.remove_unreferenced();

        self.frametime_histogram.push(delta_time);

        let frame = self.gpu.begin_frame();
        {
            let _scope = self.gpu.profiler_scope(&frame.stream, "App::render");
            self.gui.draw_ui(&self.shared_state);

            frame.stream.acquire_cmd(&self.gpu).scope(|cmd| {
                let _event_scope = cmd.event_scope("App::render::ui", (255, 0, 255));
                let _profiler_scope = self.gpu.profiler_scope(&frame.stream, "App::render::ui");
                let (back_buffer_handle, back_buffer) = self.gpu.swapchain.lock().get_back_buffer();

                cmd.resource_barriers(&[ResourceBarrier::transition(
                    &back_buffer,
                    0,
                    ResourceStates::PRESENT,
                    ResourceStates::RENDER_TARGET,
                )]);

                cmd.clear_render_target_view(back_buffer_handle, &[0.0, 0.0, 0.0, 1.0]);
                cmd.om_set_render_targets(&[back_buffer_handle], None);
                cmd.set_viewports(&[d3d12::Viewport::builder()
                    .width(self.window.size_in_pixels().0 as f32)
                    .height(self.window.size_in_pixels().1 as f32)
                    .build()]);
                cmd.set_scissor_rects(&[d3d12::Rect::builder()
                    .bottom(2160)
                    .right(3840)
                    .top(0)
                    .left(0)
                    .build()]);

                self.gui.render(&self.gpu, cmd);

                cmd.resource_barriers(&[ResourceBarrier::transition(
                    &back_buffer,
                    0,
                    ResourceStates::RENDER_TARGET,
                    ResourceStates::PRESENT,
                )]);
            });
        }

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
    pub renderer: Arc<Renderer>,
}

impl SharedState {
    pub fn new(renderer: Arc<Renderer>) -> anyhow::Result<Self> {
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
            renderer,
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
