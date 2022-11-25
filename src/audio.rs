#![cfg_attr(debug_assertions, allow(dead_code, unused_imports))]

use crate::actions::{Actions, GameControl};
use crate::loading::AudioAssets;
use crate::logic::CloudControl;
use crate::GameState;
use bevy::prelude::*;
use bevy_kira_audio::prelude::*;
use colored::*;
use iyes_loopless::prelude::*;

pub struct InternalAudioPlugin;

#[derive(Default, Eq, PartialEq, Debug, Copy, Clone)]
pub enum SelectedSong {
    #[default]
    Song1,
    Song2,
}

// This plugin is responsible to control the game audio
impl Plugin for InternalAudioPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugin(AudioPlugin)
            .add_enter_system(GameState::Playing, play_music)
            .add_system_set(
                ConditionSet::new()
                    .run_in_state(GameState::Playing)
                    .with_system(play_debug_beep_on_spawn)
                    .into(),
            );
    }
}

fn play_music(audio_assets: Res<AudioAssets>, audio: Res<Audio>) {
    audio
        .play(audio_assets.song_1.clone())
        .looped()
        .with_volume(0.5);
}

fn play_debug_beep_on_spawn(
    cloud_control: Res<CloudControl>,
    audio_assets: Res<AudioAssets>,
    audio: Res<Audio>,
) {
    if cloud_control.move_timer.finished() {
        audio.play(audio_assets.debug_beep.clone()).with_volume(0.5);
    }
}
