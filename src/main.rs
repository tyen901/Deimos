use std::{path::PathBuf, rc::Rc, sync::Arc};

use anyhow::Context;
use app::App;
use deimos_data::map::SBubbleParent;
use deimos_render::tfx::expression_vm;
use glam::Vec4;
use tiger_parse::TigerReadable;
use tiger_pkg::{PackageManager, TagHash};
use tracing_subscriber::fmt::format::FmtSpan;

mod app;
mod input;
mod panic_hook;

#[macro_use]
extern crate tracing;

const MARATHON_APP_ID: u64 = 3547690;

fn main() -> anyhow::Result<()> {
    std::panic::set_hook(Box::new(panic_hook::hook));
    tracing_subscriber::fmt()
        .pretty()
        // .with_span_events(FmtSpan::NONE)
        .with_file(false)
        .init();

    let Some(steamapp) = game_detector::steam::get_all_apps()
        .context("Failed to enumerate Steam apps")?
        .into_iter()
        .find(|a| a.appid == MARATHON_APP_ID)
    else {
        error!("Failed to find Marathon app in Steam library");
        return Ok(());
    };

    info!(
        "Found Marathon Alpha installation at '{}'",
        steamapp.game_path
    );

    let pm = Arc::new(
        PackageManager::new(
            PathBuf::from(&steamapp.game_path).join("packages"),
            tiger_pkg::GameVersion::Marathon(tiger_pkg::MarathonVersion::MarathonAlpha),
            None,
        )
        .context("Failed to initialize package manager")?,
    );
    tiger_pkg::initialize(&pm);

    // for (t, bubble) in pm.get_all_by_reference(SBubbleParent::ID.unwrap()) {
    //     println!("Bubble {} ({})", t, pm.package_paths[&t.pkg_id()].filename);
    // }

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

    let map_marsh = TagHash(0x80A8C43F);
    let map_perimeter = TagHash(0x80A75EAC);
    let mut app = App::new(sdl_context.clone(), window, map_perimeter)?;

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
