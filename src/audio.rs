#![cfg_attr(debug_assertions, allow(dead_code, unused_imports))]

use crate::actions::{Actions, GameControl};
use crate::loading::AudioAssets;
use crate::GameState;
use bevy::prelude::*;
use bevy_kira_audio::prelude::*;
use colored::*;
use iyes_loopless::prelude::*;

pub struct InternalAudioPlugin;

const SONG_PATH: &str = "audio/song_1/song_full.wav";

// This plugin is responsible to control the game audio
impl Plugin for InternalAudioPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugin(AudioPlugin)
            .add_enter_system(GameState::Playing, play_music)
            .add_system_set(
                ConditionSet::new()
                    .run_in_state(GameState::Playing)
                    // .with_system(play_music)
                    .into(),
            );
    }
}

fn play_music(asset_server: Res<AssetServer>, audio: Res<Audio>) {
    println!("{} {} {:?}", { "➤".blue() }, { "AAA:".blue() }, {});
    audio
        .play(asset_server.load(SONG_PATH))
        .looped()
        .with_volume(0.5);
}
