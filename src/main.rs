// disable console on windows for release builds
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
#![cfg_attr(debug_assertions, allow(dead_code, unused_imports))]
#![allow(clippy::type_complexity)]

use bevy::prelude::*;
use bevy::window::{PresentMode, PrimaryWindow, WindowResolution};
use bevy::winit::WinitWindows;
use bevy::DefaultPlugins;
use bevy::{
    prelude::*,
    window::{Window, WindowPlugin},
};
use bevy_easings::EasingsPlugin;
use cloud_sweeper::GamePlugin;
use std::io::Cursor;
use winit::window::{Icon, WindowId};

pub const TILE_SIZE: f32 = 16.;
pub const DISPLAY_RATIO: f32 = 1. / 4.;

// Add moving background

fn main() {
    App::new()
        .add_plugins(
            DefaultPlugins
                .set(ImagePlugin::default_nearest())
                .set(WindowPlugin {
                    primary_window: Some(Window {
                        title: "Cloud Sweeper".into(),
                        mode: bevy::window::WindowMode::BorderlessFullscreen,
                        // resizable: false,
                        // resolution: WindowResolution::new(
                        //     11. * TILE_SIZE / DISPLAY_RATIO,
                        //     10. * TILE_SIZE / DISPLAY_RATIO,
                        // ),
                        resolution: WindowResolution::default().with_scale_factor_override(1.0),
                        present_mode: PresentMode::AutoVsync,
                        ..default()
                    }),
                    ..default()
                }),
        )
        .insert_resource(Msaa::Off)
        .add_plugin(EasingsPlugin)
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
