// disable console on windows for release builds
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
#![cfg_attr(debug_assertions, allow(dead_code, unused_imports))]

use bevy::prelude::{App, ClearColor, Color, ImagePlugin, Msaa, NonSend, WindowDescriptor};
// use bevy::render::texture::ImageSettings;
use bevy::window::WindowId;
use bevy::winit::WinitWindows;
use bevy::DefaultPlugins;
use cloud_sweeper::GamePlugin;
use std::io::Cursor;
use winit::window::Icon;

// TODO: use keep pressing → + space to turn all adjacent clouds inward to unlock some situations. Must be aligned with the beat
// TODO: add item spawing to accelerate the speed, add clouds, destroy clouds, revert everything
// TODO: add music
// TODO: make the menu actuallly quit
// TODO: add tweaning
// TODO: no dead time for player actions
// TODO: sync music with cloud spawning

pub const TILE_SIZE: f32 = 16.;
pub const DISPLAY_RATIO: f32 = 1. / 4.;

fn main() {
    App::new()
        .insert_resource(Msaa { samples: 1 })
        .insert_resource(ClearColor(Color::rgb(0., 0., 0.)))
        .insert_resource(WindowDescriptor {
            width: 10.5 * TILE_SIZE / DISPLAY_RATIO,
            height: 10. * TILE_SIZE / DISPLAY_RATIO,
            title: "Cloud Sweeper".to_string(), // ToDo
            canvas: Some("#bevy".to_owned()),
            ..Default::default()
        })
        // .insert_resource(ImageSettings::default_nearest())
        .add_plugins(DefaultPlugins.set(ImagePlugin::default_nearest()))
        .add_plugin(GamePlugin)
        .add_startup_system(set_window_icon)
        .run();
}

// Sets the icon on windows and X11
fn set_window_icon(windows: NonSend<WinitWindows>) {
    let primary = windows.get_window(WindowId::primary()).unwrap();
    let icon_buf = Cursor::new(include_bytes!("../assets/textures/app_icon.png"));
    if let Ok(image) = image::load(icon_buf, image::ImageFormat::Png) {
        let image = image.into_rgba8();
        let (width, height) = image.dimensions();
        let rgba = image.into_raw();
        let icon = Icon::from_rgba(rgba, width, height).unwrap();
        primary.set_window_icon(Some(icon));
    };
}
