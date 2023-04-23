#![cfg_attr(debug_assertions, allow(dead_code, unused_imports))]

use std::time::Duration;

use crate::actions::{Actions, GameControl};
use crate::loading::AudioAssets;
use crate::logic::{CloudControl, MainClock, SPAWN_FREQUENCY, TIMER_SCALE_FACTOR};
use crate::GameState;
use bevy::prelude::*;
use bevy_kira_audio::prelude::*;
use colored::*;

pub struct InternalAudioPlugin;

#[derive(Default, Eq, PartialEq, Debug, Copy, Clone)]
pub enum SelectedSong {
    Song1,
    #[default]
    Song2,
}

pub struct SongInfo {
    pub length: f32,
    pub beat_length: f32,
    pub intro_length: f32,
}

pub const SONG_1: SongInfo = SongInfo {
    length: 60.,
    beat_length: 0.600,
    intro_length: 2.400,
};

pub const SONG_2: SongInfo = SongInfo {
    length: 60. - 2.4,
    beat_length: 0.600,
    intro_length: 2.400,
};

#[derive(Resource)]
pub struct InstanceHandle {
    pub handle: Handle<AudioInstance>,
}
// WIP: finish this replacement.
// This plugin is responsible to control the game audio
impl Plugin for InternalAudioPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugin(AudioPlugin)
            .add_system(play_music.in_schedule(OnEnter(GameState::Playing)))
            .add_system(play_debug_beep_on_spawn.run_if(in_state(GameState::Playing)));
    }
}

fn play_music(audio_assets: Res<AudioAssets>, audio: Res<Audio>, mut commands: Commands) {
    let handle: Handle<AudioInstance>;
    if audio_assets.selected_song == SelectedSong::Song1 {
        handle = audio.play(audio_assets.song_1.clone()).handle();
        commands.insert_resource(InstanceHandle { handle });
    } else if audio_assets.selected_song == SelectedSong::Song2 {
        handle = audio.play(audio_assets.song_2.clone()).handle();
        commands.insert_resource(InstanceHandle { handle });
    }
}

// fn stop_music(audio: Res<Audio>) {
//     audio.stop().fade_out(AudioTween::new(
//         Duration::from_secs(1),
//         AudioEasing::InOutPowi(2),
//     ));
// }

fn play_debug_beep_on_spawn(
    main_clock: Res<MainClock>,
    audio_assets: Res<AudioAssets>,
    audio: Res<Audio>,
) {
    if main_clock.move_clouds {
        // audio.play(audio_assets.debug_beep.clone());
    }
}
