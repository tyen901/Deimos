use std::{path::PathBuf, rc::Rc, sync::Arc};

use anyhow::Context;
use app::App;
use clap::Parser;
use cli::AppArgs;
use tiger_pkg::{PackageManager, TagHash};

mod app;
mod cli;
mod input;
mod map;
mod panic_hook;

#[macro_use]
extern crate tracing;

const MARATHON_APP_ID: u64 = 3547690;

fn main() -> anyhow::Result<()> {
    fix_windows_console();
    std::panic::set_hook(Box::new(panic_hook::hook));
    tracing_subscriber::fmt()
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
    tiger_pkg::initialize(&pm);

    let sdl_context = Rc::new(sdl3::init().expect("Failed to initialize SDL"));
    let video_subsystem = sdl_context
        .video()
        .expect("Failed to initialize video subsystem");

    let window = Rc::new(
        video_subsystem
            .window("Deimos", 1920, 1080)
            .position_centered()
            .resizable()
            .build()
            .expect("Failed to create window"),
    );

    let mut app = App::new(sdl_context.clone(), window, args)?;

    let mut event_pump = sdl_context.event_pump().unwrap();
    'app: loop {
        for event in event_pump.poll_iter() {
            match event {
                sdl3::event::Event::Quit { .. } => break 'app,
                _ => app.handle_event(event),
            }
        }

        app.render(&event_pump);
    }

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
