#![cfg_attr(debug_assertions, allow(dead_code, unused_imports))]

use crate::actions::{Actions, GameControl};
use crate::loading::AudioAssets;
use crate::logic::{CloudControl, MainClock, SPAWN_FREQUENCY, TIMER_SCALE_FACTOR};
use crate::GameState;
use bevy::prelude::*;
use bevy_kira_audio::prelude::*;
use colored::*;
use iyes_loopless::prelude::*;

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
            .add_enter_system(GameState::Playing, play_music)
            .add_system_set(
                ConditionSet::new()
                    .run_in_state(GameState::Playing)
                    .with_system(play_debug_beep_on_spawn)
                    .with_system(resync_music)
                    .into(),
            );
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

fn resync_music(
    mut audio_instances: ResMut<Assets<AudioInstance>>,
    mut main_clock: ResMut<MainClock>,
    handle: Res<InstanceHandle>,
) {
    // if audio_instances.get_mut(&handle.handle).is_some() {
    //     let play_pos = audio_instances.state(&handle.handle).position();
    //     if main_clock.move_clouds {
    //         if let Some(play_pos) = play_pos {
    //             // The correction is positive if the game logic is late, negative
    //             // if in advance (we add the correction):
    //             let beat_width = MAIN_PERIOD as f64 * TIMER_SCALE_FACTOR as f64;
    //             let div = play_pos.div_euclid(beat_width);
    //             let rem = play_pos.rem_euclid(beat_width);
    //             let time_correction = if (beat_width) * div > play_pos {
    //                 // This is negative, logic in advance:
    //                 (-rem) as f32
    //             } else {
    //                 // This is positive, logic is late:
    //                 (play_pos - (beat_width) * div) as f32
    //             };

    //             println!(
    //                 "{} {} {:?} {:?} {:?}",
    //                 { "➤".blue() },
    //                 { "BBB:".blue() },
    //                 { play_pos },
    //                 { beat_width * div },
    //                 { play_pos - (beat_width * div) - main_clock.excess_time as f64 }
    //             );
    //             main_clock.time_correction = time_correction;
    //         }
    //     }
    // }
}

fn play_debug_beep_on_spawn(
    main_clock: Res<MainClock>,
    audio_assets: Res<AudioAssets>,
    audio: Res<Audio>,
) {
    if main_clock.move_clouds {
        // audio.play(audio_assets.debug_beep.clone());
    }
}
