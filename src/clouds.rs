use crate::audio::{SONG_1, SONG_2};
use crate::loading::{AudioAssets, TextureAssets};
use crate::logic::CLOUD_EASING;
use crate::{
    actions::GameControl,
    logic::{CloudControl, GridState, PUSH_COOLDOWN_FACTOR},
    player::TILE_SIZE,
    GameState,
};
use bevy::prelude::*;
use bevy_easings::EaseFunction;
use colored::*;

pub const CLOUD_LAYER: f32 = 9.;

pub struct CloudPlugin;

// pub trait GridMove {
//     fn grid_move(&self);
// }

#[derive(Component)]
pub struct Cloud {
    pub dir: CloudDir,
}

#[derive(Component)]
pub struct GridPos {
    pub pos: [i8; 2],
    pub old_pos: [i8; 2],
    pub is_pushed: bool,
}

#[derive(Component)]
pub struct LeftCloud;

#[derive(Component)]
pub struct RightCloud;
#[derive(Component)]
pub struct UpCloud;
#[derive(Component)]
pub struct DownCloud;

#[derive(Component)]
pub struct ToDespawn;

#[derive(Default, Eq, PartialEq, Debug, Copy, Clone)]
pub enum CloudDir {
    #[default]
    Up,
    Down,
    Left,
    Right,
}

#[derive(Component)]
pub struct IsCooldown {
    pub val: bool,
}

#[derive(Default, Eq, PartialEq, Debug, Copy, Clone)]
pub enum AnimationState {
    #[default]
    Init,
    Move,
    End,
}

#[derive(Component)]
pub struct Animation {
    pub state: AnimationState,
}

#[derive(Component, Deref, DerefMut)]
pub struct CooldownTimer {
    pub timer: Timer,
}

pub fn new_cloud(
    mut cloud_control: ResMut<CloudControl>,
    mut grid_state: ResMut<GridState>,
    mut commands: Commands,
    audio_assets: Res<AudioAssets>,
    asset_server: Res<AssetServer>,
) {
    let beat_length = match audio_assets.selected_song {
        crate::audio::SelectedSong::Song1 => SONG_1.beat_length,
        crate::audio::SelectedSong::Song2 => SONG_2.beat_length,
    };
    if let Some(cloud_dir) = cloud_control.cur_new_cloud {
        // Spawn a new cloud, with a sprite bundle, associate the direction
        if let Some((cloud_pos_vec, cloud_pos_grid)) = grid_state.new_cloud(cloud_dir) {
            match cloud_dir {
                CloudDir::Down => {
                    commands
                        .spawn(SpriteBundle {
                            texture: asset_server.load("textures/down_cloud.png"),
                            transform: Transform::from_translation(cloud_pos_vec),
                            ..default()
                        })
                        .insert(DownCloud)
                        .insert(CooldownTimer {
                            timer: Timer::from_seconds(
                                PUSH_COOLDOWN_FACTOR * beat_length,
                                TimerMode::Once,
                            ),
                        })
                        .insert(Cloud {
                            dir: CloudDir::Down,
                        })
                        .insert(IsCooldown { val: false })
                        .insert(GridPos {
                            pos: cloud_pos_grid,
                            old_pos: cloud_pos_grid,
                            is_pushed: false,
                        })
                        .insert(Animation {
                            state: AnimationState::Init,
                        });
                }
                CloudDir::Left => {
                    commands
                        .spawn(SpriteBundle {
                            texture: asset_server.load("textures/left_cloud.png"),
                            transform: Transform::from_translation(cloud_pos_vec),
                            ..default()
                        })
                        .insert(LeftCloud)
                        .insert(CooldownTimer {
                            timer: Timer::from_seconds(
                                PUSH_COOLDOWN_FACTOR * beat_length,
                                TimerMode::Once,
                            ),
                        })
                        .insert(Cloud {
                            dir: CloudDir::Left,
                        })
                        .insert(IsCooldown { val: false })
                        .insert(GridPos {
                            pos: cloud_pos_grid,
                            old_pos: cloud_pos_grid,
                            is_pushed: false,
                        })
                        .insert(Animation {
                            state: AnimationState::Init,
                        });
                }
                CloudDir::Up => {
                    commands
                        .spawn(SpriteBundle {
                            texture: asset_server.load("textures/up_cloud.png"),
                            transform: Transform::from_translation(cloud_pos_vec),
                            ..default()
                        })
                        .insert(UpCloud)
                        .insert(CooldownTimer {
                            timer: Timer::from_seconds(
                                PUSH_COOLDOWN_FACTOR * beat_length,
                                TimerMode::Once,
                            ),
                        })
                        .insert(IsCooldown { val: false })
                        .insert(Cloud { dir: CloudDir::Up })
                        .insert(GridPos {
                            pos: cloud_pos_grid,
                            old_pos: cloud_pos_grid,
                            is_pushed: false,
                        })
                        .insert(Animation {
                            state: AnimationState::Init,
                        });
                }
                CloudDir::Right => {
                    commands
                        .spawn(SpriteBundle {
                            texture: asset_server.load("textures/right_cloud.png"),
                            transform: Transform::from_translation(cloud_pos_vec),
                            ..default()
                        })
                        .insert(RightCloud)
                        .insert(Cloud {
                            dir: CloudDir::Right,
                        })
                        .insert(CooldownTimer {
                            timer: Timer::from_seconds(
                                PUSH_COOLDOWN_FACTOR * beat_length,
                                TimerMode::Once,
                            ),
                        })
                        .insert(IsCooldown { val: false })
                        .insert(GridPos {
                            pos: cloud_pos_grid,
                            old_pos: cloud_pos_grid,
                            is_pushed: false,
                        })
                        .insert(Animation {
                            state: AnimationState::Init,
                        });
                }
            }
        }
    }
    cloud_control.cur_new_cloud = None;
}
