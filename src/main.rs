// disable console on windows for release builds
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
#![cfg_attr(debug_assertions, allow(dead_code, unused_imports))]

use bevy::prelude::*;
use bevy::window::{PresentMode, PrimaryWindow, WindowResolution};
// use bevy::prelude::{
//     App, ClearColor, Color, ImagePlugin, Msaa, NonSend, PluginGroup, WindowDescriptor,
// };
// use bevy::render::texture::ImageSettings;
use bevy::winit::WinitWindows;
use bevy::DefaultPlugins;
use bevy::{
    prelude::*,
    window::{Window, WindowPlugin},
};
use cloud_sweeper::GamePlugin;
use std::io::Cursor;
use winit::window::{Icon, WindowId};
// TODO: bug, when doing special keep cooldown texture
// TODO: bug, when in a corner, doesn't detect lose condition

// TODO: add lose condition when the player is totally surrounded
// TODO: limit the player buffer to 2 moves per 2 beats?
// IDEA: add item spawing to accelerate the speed, add clouds, destroy clouds, revert everything
// TODO: make the menu actuallly quit
// TODO: add tweaning

pub const TILE_SIZE: f32 = 16.;
pub const DISPLAY_RATIO: f32 = 1. / 4.;

fn main() {
    App::new()
        .add_plugins(
            DefaultPlugins
                .set(ImagePlugin::default_nearest())
                .set(WindowPlugin {
                    primary_window: Some(Window {
                        title: "Cloud Sweeper".into(),
                        mode: bevy::window::WindowMode::Windowed,
                        resizable: false,
                        resolution: WindowResolution::new(
                            10.5 * TILE_SIZE / DISPLAY_RATIO,
                            10. * TILE_SIZE / DISPLAY_RATIO,
                        )
                        .with_scale_factor_override(1.0),
                        present_mode: PresentMode::AutoVsync,
                        // Tells wasm to resize the window according to the available canvas
                        fit_canvas_to_parent: true,
                        // Tells wasm not to override default event handling, like F5, Ctrl+R etc.
                        prevent_default_event_handling: false,
                        ..default()
                    }),
                    ..default()
                }),
        )
        .insert_resource(Msaa::Off)
        .insert_resource(ClearColor(Color::rgb(0., 0., 0.)))
        .add_plugin(GamePlugin)
        .add_startup_system(set_window_icon)
        .run();
}

// Sets the icon on windows and X11
fn set_window_icon(
    primary_query: Query<Entity, With<PrimaryWindow>>,
    windows: NonSend<WinitWindows>,
) {
    let Ok(entity) = primary_query.get_single() else {
        return;
    };
    let Some(primary) = windows.get_window(entity) else {
        return;
    };

    let icon_buf = Cursor::new(include_bytes!("../assets/textures/app_icon.png"));
    if let Ok(image) = image::load(icon_buf, image::ImageFormat::Png) {
        let image = image.into_rgba8();
        let (width, height) = image.dimensions();
        let rgba = image.into_raw();
        let icon = Icon::from_rgba(rgba, width, height).unwrap();

        primary.set_window_icon(Some(icon));
    }
}
