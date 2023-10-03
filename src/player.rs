#![cfg_attr(debug_assertions, allow(dead_code, unused_imports))]

use crate::actions::{Actions, GameControl};
use crate::audio::{SoundOnAction, SoundOnMove};
use crate::clouds::{Animation, AnimationState, CloudDir};
use crate::loading::TextureAssets;
use crate::logic::{
    CloudControl, GridState, PushState, TileOccupation, MAX_BUFFER_INPUT, SPECIAL_TIMEOUT,
};
use crate::world::{STAGE_BL, STAGE_UR};
use crate::GameState;
use bevy::prelude::*;
use bevy_easings::*;
// use bevy::render::texture::ImageSettings;
use colored::*;

pub const TILE_SIZE: f32 = 16.;
pub const PLAYER_LAYER: f32 = 10.;
pub const INIT_POS: [i8; 2] = [5i8, 5i8];
pub const BUFFER_SIZE: usize = 2;
pub const PLAYER_EASING: bevy_easings::EaseFunction = bevy_easings::EaseFunction::QuadraticIn;
// Duration of the easing for the clouds in ms:
pub const PLAYER_EASING_DURATION: u64 = 50;

pub struct PlayerPlugin;

/// Contains the info about the player
///
/// The buffer is a FIFO, with the oldest element at index 0.
#[derive(Default, Resource)]
pub struct PlayerControl {
    pub input_buffer: [GameControl; MAX_BUFFER_INPUT],
    pub special_control: u8,
    pub player_pos: [i8; BUFFER_SIZE],
    pub input_timer: Timer,
    pub special_timeout: u8,
    pub animation: AnimationState,
    pub sound_counter: u8,
}

#[derive(Component, Default)]
pub struct Player {
    pub pos: Vec2,
}

/// This plugin handles player related stuff like movement
/// Player logic is only active during the State `GameState::Playing`
impl Plugin for PlayerPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(GameState::Playing), spawn_player)
            .add_systems(
                Update,
                finish_player_move.run_if(in_state(GameState::Playing)),
            )
            .add_systems(
                Update,
                (animate_sprite, move_player)
                    .run_if(in_state(GameState::Playing))
                    .after(fill_player_buffer),
            );
    }
}

#[derive(Component, Deref, DerefMut)]
struct AnimationTimer(Timer);

fn animate_sprite(
    time: Res<Time>,
    texture_atlases: Res<Assets<TextureAtlas>>,
    mut query: Query<(
        &mut AnimationTimer,
        &mut TextureAtlasSprite,
        &Handle<TextureAtlas>,
    )>,
) {
    for (mut timer, mut sprite, texture_atlas_handle) in &mut query {
        timer.tick(time.delta());
        if timer.just_finished() {
            let texture_atlas = texture_atlases.get(texture_atlas_handle).unwrap();
            sprite.index = (sprite.index + 1) % texture_atlas.textures.len();
        }
    }
}

/// Add all the actions (moves) to the buffer whose elements are going to be popped
pub fn fill_player_buffer(mut actions: ResMut<Actions>, mut player_control: ResMut<PlayerControl>) {
    let game_control = actions.next_action;
    let idle_ndx = player_control
        .input_buffer
        .iter()
        .position(|x| x == &GameControl::Idle);
    let special_ndx = player_control
        .input_buffer
        .iter()
        .position(|x| x == &GameControl::Special);

    if game_control != GameControl::Idle {
        // The buffer is not full, we can replace the first idle element
        if let Some(x) = idle_ndx {
            match game_control {
                GameControl::Idle => {}
                GameControl::Up | GameControl::Down | GameControl::Left | GameControl::Right => {
                    // reset the special buffer every time a direction is played:
                    player_control.special_control = 0;
                    match special_ndx {
                        Some(_y) => {
                            // Reset the buffer, it forces
                            player_control.input_buffer = [GameControl::Idle; BUFFER_SIZE];
                            player_control.input_buffer[0] = game_control;
                        }
                        _ => player_control.input_buffer[x] = game_control,
                    }
                }
                GameControl::Special => player_control.input_buffer[x] = GameControl::Special,
            }
        }
        // The buffer is full, replace the last element:
        else {
            let n = player_control.input_buffer.len() - 1;
            match game_control {
                GameControl::Idle => {}
                GameControl::Up | GameControl::Down | GameControl::Left | GameControl::Right => {
                    player_control.special_control = 0;
                    match special_ndx {
                        Some(_y) => {
                            // Reset the buffer, it forces
                            player_control.input_buffer = [GameControl::Idle; BUFFER_SIZE];
                            player_control.input_buffer[0] = game_control;
                        }
                        _ => player_control.input_buffer[n] = game_control,
                    }
                }
                GameControl::Special => player_control.input_buffer[n] = GameControl::Special,
            }
        }
    };
    actions.next_action = GameControl::Idle;
}

pub fn move_player(
    mut commands: Commands,
    mut player_query: Query<(&Transform, Entity), With<Player>>,
    mut player_control: ResMut<PlayerControl>,
) {
    let pl_grid_pos = player_control.player_pos;
    for (transform, entity) in player_query.iter_mut() {
        let new_pos = Vec3::new(
            (f32::from(pl_grid_pos[0]) - INIT_POS[0] as f32) * TILE_SIZE + TILE_SIZE / 2.
                - TILE_SIZE / 2.,
            (f32::from(pl_grid_pos[1]) - INIT_POS[1] as f32) * TILE_SIZE + TILE_SIZE / 2.,
            PLAYER_LAYER,
        );
        match player_control.animation {
            AnimationState::End => {
                commands.entity(entity).insert(transform.ease_to(
                    Transform::from_translation(new_pos),
                    PLAYER_EASING,
                    bevy_easings::EasingType::Once {
                        duration: std::time::Duration::from_millis(PLAYER_EASING_DURATION),
                    },
                ));
                player_control.animation = AnimationState::Move;
            }
            AnimationState::Move => (),
            // This is used to avoid the Player sprite to ease in towards the center at spawn time:
            AnimationState::Init => {
                commands
                    .entity(entity)
                    .insert(Transform::from_translation(new_pos));
                player_control.animation = AnimationState::End;
            }
        }
    }
}

fn finish_player_move(
    mut removed: RemovedComponents<EasingComponent<Transform>>,
    mut query: Query<Entity, With<Player>>,
    mut player_control: ResMut<PlayerControl>,
) {
    for del_entity in removed.iter() {
        for entity in query.iter_mut() {
            if entity == del_entity {
                player_control.animation = AnimationState::End;
            }
        }
    }
}

fn spawn_player(
    mut commands: Commands,
    mut texture_atlases: ResMut<Assets<TextureAtlas>>,
    asset_server: Res<AssetServer>,
) {
    let texture_handle = asset_server.load("textures/duck_spritesheet.png");
    let texture_atlas = TextureAtlas::from_grid(
        texture_handle,
        Vec2::new(TILE_SIZE, TILE_SIZE),
        1,
        4,
        None,
        None,
    );
    let texture_atlas_handle = texture_atlases.add(texture_atlas);
    commands
        .spawn(SpriteSheetBundle {
            texture_atlas: texture_atlas_handle,
            transform: Transform::from_xyz(
                TILE_SIZE * (0.5 + (INIT_POS[0] as f32)),
                TILE_SIZE * (0.5 + (INIT_POS[1] as f32)),
                PLAYER_LAYER,
            ),
            ..default()
        })
        .insert(Player::default())
        .insert(AnimationTimer(Timer::from_seconds(
            0.1,
            TimerMode::Repeating,
        )));
}

/// Pop and applies all the player moves and special when the timer expires
pub fn pop_player_buffer(
    mut cloud_control: ResMut<CloudControl>,
    mut grid_state: ResMut<GridState>,
    mut player_control: ResMut<PlayerControl>,
    time: Res<Time>,
    mut play_move_sound_event: EventWriter<SoundOnMove>,
    mut play_push_sound_event: EventWriter<SoundOnAction>,
) {
    // timers gotta be ticked, to work
    player_control.input_timer.tick(time.delta());

    if player_control.input_timer.just_finished() {
        let player_action = player_control.input_buffer[0];
        player_control.input_buffer[0] = GameControl::Idle;
        player_control.input_buffer.rotate_left(1);

        let (player_new_pos, action_direction, push_state): ([i8; 2], CloudDir, PushState) =
            match player_action {
                GameControl::Down => {
                    let new_pos = if player_control.player_pos[1] > (STAGE_BL[1] as i8) {
                        [
                            player_control.player_pos[0],
                            player_control.player_pos[1] - 1,
                        ]
                    } else {
                        player_control.player_pos
                    };
                    let dir = CloudDir::Down;
                    (
                        new_pos,
                        dir,
                        grid_state.is_occupied(new_pos, dir, TileOccupation::Player),
                    )
                }
                GameControl::Up => {
                    let new_pos = if player_control.player_pos[1] < (STAGE_UR[1] as i8) {
                        [
                            player_control.player_pos[0],
                            player_control.player_pos[1] + 1,
                        ]
                    } else {
                        player_control.player_pos
                    };
                    let dir = CloudDir::Up;
                    (
                        new_pos,
                        dir,
                        grid_state.is_occupied(new_pos, dir, TileOccupation::Player),
                    )
                }
                GameControl::Left => {
                    let new_pos = if player_control.player_pos[0] > (STAGE_BL[0] as i8) {
                        [
                            player_control.player_pos[0] - 1,
                            player_control.player_pos[1],
                        ]
                    } else {
                        player_control.player_pos
                    };
                    let dir = CloudDir::Left;
                    (
                        new_pos,
                        dir,
                        grid_state.is_occupied(new_pos, dir, TileOccupation::Player),
                    )
                }
                GameControl::Right => {
                    let new_pos = if player_control.player_pos[0] < (STAGE_UR[0] as i8) {
                        [
                            player_control.player_pos[0] + 1,
                            player_control.player_pos[1],
                        ]
                    } else {
                        player_control.player_pos
                    };
                    let dir = CloudDir::Right;
                    (
                        new_pos,
                        dir,
                        grid_state.is_occupied(new_pos, dir, TileOccupation::Player),
                    )
                }
                GameControl::Idle => (player_control.player_pos, CloudDir::Right, PushState::Empty),
                GameControl::Special => {
                    player_control.special_control += 1;
                    player_control.special_timeout = 0;
                    (
                        player_control.player_pos,
                        CloudDir::Down,
                        PushState::Blocked,
                    )
                }
            };
        let player_old_pos = player_control.player_pos;

        if player_action != GameControl::Idle {
            player_control.sound_counter += 1;
            player_control.sound_counter %= 3;
            match push_state {
                PushState::Empty => {
                    player_control.player_pos = player_new_pos;
                    debug!("pl. pos: {:?}", player_control.player_pos);
                    grid_state.grid[player_old_pos[0] as usize][player_old_pos[1] as usize] =
                        TileOccupation::Empty;
                    grid_state.grid[player_new_pos[0] as usize][player_new_pos[1] as usize] =
                        TileOccupation::Player;
                    if player_control.sound_counter == 0 {
                        play_move_sound_event.send_default();
                    }
                }
                PushState::Blocked => {}
                PushState::CanPush => {
                    cloud_control
                        .pushed_clouds
                        .push((player_old_pos, action_direction));
                    match action_direction {
                        CloudDir::Up => {
                            play_push_sound_event.send(SoundOnAction {
                                direction: GameControl::Down,
                            });
                            cloud_control.next_pushed_clouds.push((
                                player_new_pos,
                                action_direction,
                                PushState::PlayerCanPush,
                            ))
                        }
                        CloudDir::Down => {
                            play_push_sound_event.send(SoundOnAction {
                                direction: GameControl::Up,
                            });
                            cloud_control.next_pushed_clouds.push((
                                player_new_pos,
                                action_direction,
                                PushState::PlayerCanPush,
                            ))
                        }
                        CloudDir::Left => {
                            play_push_sound_event.send(SoundOnAction {
                                direction: GameControl::Right,
                            });
                            cloud_control.next_pushed_clouds.push((
                                player_new_pos,
                                action_direction,
                                PushState::PlayerCanPush,
                            ))
                        }
                        CloudDir::Right => {
                            play_push_sound_event.send(SoundOnAction {
                                direction: GameControl::Left,
                            });
                            cloud_control.next_pushed_clouds.push((
                                player_new_pos,
                                action_direction,
                                PushState::PlayerCanPush,
                            ))
                        }
                    };
                    play_push_sound_event.send_default();
                }
                _ => {}
            }
        }
        player_control.special_timeout += 1;
        // if the special is not used soon enough, it expires:
        if player_control.special_timeout >= SPECIAL_TIMEOUT {
            player_control.special_timeout = 0;
            player_control.special_control = 0;
        }
    };
}
