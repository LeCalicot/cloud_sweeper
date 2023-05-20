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

#[derive(Default)]
pub struct SoundOnMove;

#[derive(Default)]
pub struct SoundOnPush;

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
pub struct SongHandle {
    pub song: Handle<AudioInstance>,
}

// This plugin is responsible to control the game audio
impl Plugin for InternalAudioPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugin(AudioPlugin)
            .add_system(play_music.in_schedule(OnEnter(GameState::Playing)))
            .add_system(play_debug_beep_on_spawn.run_if(in_state(GameState::Playing)))
            .add_system(play_sound_on_move.run_if(in_state(GameState::Playing)))
            .add_system(play_sound_on_push.run_if(in_state(GameState::Playing)));
    }
}

fn play_music(audio_assets: Res<AudioAssets>, audio: Res<Audio>, mut commands: Commands) {
    let handle: Handle<AudioInstance>;
    if audio_assets.selected_song == SelectedSong::Song1 {
        handle = audio.play(audio_assets.song_1.clone()).handle();
        commands.insert_resource(SongHandle { song: handle });
    } else if audio_assets.selected_song == SelectedSong::Song2 {
        handle = audio.play(audio_assets.song_2.clone()).handle();
        commands.insert_resource(SongHandle { song: handle });
    }
}

fn play_sound_on_move(
    mut play_sound_events: EventReader<SoundOnMove>,
    audio: Res<Audio>,
    audio_assets: Res<AudioAssets>,
) {
    for _ in play_sound_events.iter() {
        println!("{} {} {:?}", { "➤".blue() }, { "AAA:".blue() }, {
            "move"
        });
        audio.play(audio_assets.sample_1_c.clone());
    }
}

fn play_sound_on_push(mut play_sound_events: EventReader<SoundOnPush>) {
    for _ in play_sound_events.iter() {
        println!("{} {} {:?}", { "➤".blue() }, { "BBB:".blue() }, {
            "push"
        });
    }
}

// pub sample_1_a: Handle<AudioInstance>,
// pub sample_1_b: Handle<AudioInstance>,
// pub sample_1_c: Handle<AudioInstance>,
// pub sample_1_d: Handle<AudioInstance>,
// pub sample_2_a: Handle<AudioInstance>,
// pub sample_2_b: Handle<AudioInstance>,
// pub sample_2_c: Handle<AudioInstance>,
// pub sample_2_d: Handle<AudioInstance>,
// pub sample_3_a: Handle<AudioInstance>,
// pub sample_3_b: Handle<AudioInstance>,
// pub sample_3_c: Handle<AudioInstance>,
// pub sample_3_d: Handle<AudioInstance>,

// fn stop_music(audio: Res<Audio>) {
//     audio.stop().fade_out(AudioTween::new(
//         Duration::from_secs(1),
//         AudioEasing::InOutPowi(2),
//     ));
// }

// WIP: play sample on move

fn play_debug_beep_on_spawn(
    main_clock: Res<MainClock>,
    _audio_assets: Res<AudioAssets>,
    _audio: Res<Audio>,
) {
    if main_clock.move_clouds {
        // audio.play(audio_assets.debug_beep.clone());
    }
}
