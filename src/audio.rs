#![cfg_attr(debug_assertions, allow(dead_code, unused_imports))]

use crate::actions::{Actions, GameControl};
use crate::loading::AudioAssets;
use crate::logic::{CloudControl, MainClock};
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

#[derive(Resource)]
pub struct InstanceHandle {
    handle: Handle<AudioInstance>,
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
    main_clock: Res<MainClock>,
    handle: Res<InstanceHandle>,
) {
    if audio_instances.get_mut(&handle.handle).is_some() {
        let play_pos = audio_instances.state(&handle.handle).position();
        if main_clock.move_clouds {
            info!("{play_pos:?}")
        }
    }
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
