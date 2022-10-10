#![cfg_attr(debug_assertions, allow(dead_code, unused_imports))]

mod actions;
mod audio;
mod clouds;
mod loading;
mod logic;
mod menu;
mod player;
mod world;

use crate::actions::ActionsPlugin;
use crate::audio::InternalAudioPlugin;
use crate::clouds::CloudPlugin;
use crate::loading::LoadingPlugin;
use crate::logic::LogicPlugin;
use crate::menu::MenuPlugin;
use crate::player::PlayerPlugin;
use crate::world::WorldPlugin;

use bevy::app::App;
#[cfg(debug_assertions)]
use bevy::diagnostic::{FrameTimeDiagnosticsPlugin, LogDiagnosticsPlugin};
use bevy::prelude::*;
use bevy_prototype_debug_lines::DebugLinesPlugin;
use iyes_loopless::prelude::*;

// This example game uses States to separate logic
// See https://bevy-cheatbook.github.io/programming/states.html
// Or https://github.com/bevyengine/bevy/blob/main/examples/ecs/state.rs
#[derive(Clone, Eq, PartialEq, Debug, Hash)]
enum GameState {
    // During the loading State the LoadingPlugin will load our assets
    Loading,
    // During this State the actual game logic is executed
    Playing,
    // Here the menu is drawn and waiting for player interaction
    Menu,
}

pub struct GamePlugin;

impl Plugin for GamePlugin {
    fn build(&self, app: &mut App) {
        app.add_loopless_state(GameState::Loading)
            .add_plugin(DebugLinesPlugin::default())
            .add_plugin(LoadingPlugin)
            .add_plugin(MenuPlugin)
            .add_plugin(ActionsPlugin)
            // .add_plugin(InternalAudioPlugin)
            .add_plugin(WorldPlugin)
            .add_plugin(PlayerPlugin)
            .add_plugin(LogicPlugin)
            .add_plugin(CloudPlugin);
        #[cfg(debug_assertions)]
        {
            app.add_system(bevy::window::close_on_esc)
            // /.add_plugin(FrameTimeDiagnosticsPlugin::default())
                // .add_plugin(LogDiagnosticsPlugin::default())
                ;
        }
    }
}
