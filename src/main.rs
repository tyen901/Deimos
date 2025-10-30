use std::{path::PathBuf, rc::Rc, sync::Arc};

use anyhow::Context;
use app::App;
use clap::Parser;
use cli::AppArgs;
use deimos_core::MARATHON_APP_ID;
use itertools::Itertools;
use tiger_pkg::PackageManager;
use tracing_subscriber::filter::{EnvFilter, LevelFilter};

mod app;
mod cli;
mod map;
mod panic_hook;
mod task;
mod ui;

#[macro_use]
extern crate tracing;

#[cfg(all(feature = "dhat-heap", not(feature = "tracy")))]
#[global_allocator]
static ALLOC: dhat::Alloc = dhat::Alloc;

// #[cfg(feature = "tracy")]
// #[global_allocator]
// static GLOBAL: tracy_client::ProfiledAllocator<std::alloc::System> =
//     tracy_client::ProfiledAllocator::new(std::alloc::System, 100);

fn main() -> anyhow::Result<()> {
    #[cfg(feature = "dhat-heap")]
    let _profiler = dhat::Profiler::new_heap();

    fix_windows_console();
    std::panic::set_hook(Box::new(panic_hook::hook));
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::builder()
                .with_default_directive(LevelFilter::INFO.into())
                .from_env_lossy(),
        )
        .pretty()
        // .with_span_events(FmtSpan::NONE)
        .with_file(false)
        .init();

    let args = AppArgs::parse();

    let game_path = if let Some(path) = &args.gamedir {
        path.clone()
    } else {
        let Some(steamapp) = game_detector::steam::get_all_apps()
            .context("Failed to enumerate Steam apps")?
            .into_iter()
            .find(|a| a.appid == MARATHON_APP_ID)
        else {
            error!("Failed to find Marathon app in Steam library. If you don't have Marathon installed through Steam, then you can specify the path to the game directory using the --gamedir/-g argument.");
            return Ok(());
        };

        info!(
            "Found Marathon Alpha installation at '{}'",
            steamapp.game_path
        );

        steamapp.game_path
    };

    let pm = Arc::new(
        PackageManager::new(
            PathBuf::from(&game_path).join("packages"),
            tiger_pkg::GameVersion::Marathon(tiger_pkg::MarathonVersion::MarathonAlpha),
            None,
        )
        .context("Failed to initialize package manager")?,
    );
    tiger_pkg::initialize_package_manager(&pm);

    let sdl_context = Rc::new(sdl3::init().expect("Failed to initialize SDL"));
    let video_subsystem = sdl_context
        .video()
        .expect("Failed to initialize video subsystem");

    let mut window = {
        let mut builder = video_subsystem.window("Deimos", 1920, 1080);

        let mut builder_ref = builder.position_centered().resizable();

        if cfg!(not(target_os = "windows")) {
            builder_ref = builder_ref.vulkan();
        }

        builder_ref.build().expect("Failed to create window")
    };

    if let Some(display_index) = args.display {
        let displays = video_subsystem.displays()?;
        let Some(display) = displays.get(display_index) else {
            anyhow::bail!(
                "Invalid display index (available displays: {:?})",
                displays.iter().enumerate().map(|(i, _d)| i).collect_vec()
            );
        };
        let display_center = display.get_bounds()?.center();
        let window_size = window.size();
        window.set_position(
            sdl3::video::WindowPos::Positioned(display_center.x - window_size.0 as i32 / 2),
            sdl3::video::WindowPos::Positioned(display_center.y - window_size.1 as i32 / 2),
        );
    }

    let mut app = App::new(sdl_context.clone(), Rc::new(window), args)?;

    let mut event_pump = sdl_context.event_pump().unwrap();
    'app: while app.running {
        for event in event_pump.poll_iter() {
            match event {
                sdl3::event::Event::Quit { .. } => break 'app,
                _ => app.handle_event(event),
            }
        }

        app.render(&event_pump);
    }

    tiger_pkg::finalize_package_manager();

    Ok(())
}

fn fix_windows_console() {
    #[cfg(target_os = "windows")]
    {
        pub type Handle = *mut std::ffi::c_void;

        extern "C" {
            fn SetConsoleMode(handle: Handle, mode: u32) -> i32;
            fn GetStdHandle(handle: u32) -> Handle;
        }

        const STD_OUTPUT_HANDLE: u32 = -11i32 as u32;
        const ENABLE_PROCESSED_OUTPUT: u32 = 1u32;
        const ENABLE_VIRTUAL_TERMINAL_PROCESSING: u32 = 4u32;
        unsafe {
            let stdout = GetStdHandle(STD_OUTPUT_HANDLE);
            if !stdout.is_null() {
                SetConsoleMode(
                    stdout,
                    ENABLE_PROCESSED_OUTPUT | ENABLE_VIRTUAL_TERMINAL_PROCESSING,
                );
            }
        }
    }
}

// Workaround for subsecond missing this symbol while linking (even though its not used)
#[no_mangle]
extern "C" fn CoCreateGuid() {}
