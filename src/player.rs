#![cfg_attr(debug_assertions, allow(dead_code, unused_imports))]

use crate::actions::{Actions, GameControl};
use crate::clouds::CloudDir;
use crate::loading::TextureAssets;
use crate::logic::{CloudControl, GridState, PushState, TileOccupation, MAX_BUFFER_INPUT};
use crate::world::{STAGE_BL, STAGE_UR};
use crate::GameState;
use bevy::prelude::*;
// use bevy::render::texture::ImageSettings;
use iyes_loopless::prelude::*;

pub const TILE_SIZE: f32 = 16.;
pub const PLAYER_LAYER: f32 = 10.;
pub const INIT_POS: [i8; 2] = [5i8, 5i8];

pub struct PlayerPlugin;

/// Contains the info about the player
///
/// The bufferis a FIFO, with the oldest element at index 0.
#[derive(Default, Resource)]
pub struct PlayerControl {
    pub input_buffer: [GameControl; MAX_BUFFER_INPUT],
    pub player_pos: [i8; 2],
    pub timer: Timer,
}

#[derive(Component, Default)]
pub struct Player {
    pub pos: Vec2,
}

/// This plugin handles player related stuff like movement
/// Player logic is only active during the State `GameState::Playing`
impl Plugin for PlayerPlugin {
    fn build(&self, app: &mut App) {
        app.add_enter_system(GameState::Playing, spawn_player)
            .add_system_set(
                ConditionSet::new()
                    .run_in_state(GameState::Playing)
                    .after("fill_player_buffer")
                    .with_system(animate_sprite)
                    .with_system(move_player)
                    .into(),
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
    let game_control = actions.next_move;
    let idle_ndx = player_control
        .input_buffer
        .iter()
        .position(|x| x == &GameControl::Idle);

    if game_control != GameControl::Idle {
        // The buffer is not full, we can replace the first idle element
        if let Some(x) = idle_ndx {
            player_control.input_buffer[x] = game_control;
        }
        // The buffer is full, replace the last element:
        else {
            let n = player_control.input_buffer.len() - 1;
            player_control.input_buffer[n] = game_control;
        }
    };
    actions.next_move = GameControl::Idle;
}

pub fn move_player(
    mut player_query: Query<&mut Transform, With<Player>>,
    player_control: Res<PlayerControl>,
) {
    let pl_grid_pos = player_control.player_pos;
    for mut transform in player_query.iter_mut() {
        transform.translation = Vec3::new(
            (f32::from(pl_grid_pos[0]) - INIT_POS[0] as f32) * TILE_SIZE + TILE_SIZE / 2.
                - TILE_SIZE / 2.,
            (f32::from(pl_grid_pos[1]) - INIT_POS[1] as f32) * TILE_SIZE + TILE_SIZE / 2.,
            PLAYER_LAYER,
        );
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

/// Pop and applies all the player moves when the timer expires
pub fn pop_player_buffer(
    mut cloud_control: ResMut<CloudControl>,
    mut grid_state: ResMut<GridState>,
    mut player_control: ResMut<PlayerControl>,
    time: Res<Time>,
) {
    // timers gotta be ticked, to work
    player_control.timer.tick(time.delta());

    // if it finished, despawn the bomb
    if player_control.timer.just_finished() {
        let player_action = player_control.input_buffer[0];
        player_control.input_buffer[0] = GameControl::Idle;
        player_control.input_buffer.rotate_left(1);

        // let mut action_direction = CloudDir::Down;

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
                GameControl::Idle => (
                    player_control.player_pos,
                    CloudDir::Right,
                    PushState::Blocked,
                ),
            };
        let player_old_pos = player_control.player_pos;

        if player_action != GameControl::Idle {
            match push_state {
                PushState::Empty => {
                    player_control.player_pos = player_new_pos;
                    debug!("pl. pos: {:?}", player_control.player_pos);
                    grid_state.grid[player_old_pos[0] as usize][player_old_pos[1] as usize] =
                        TileOccupation::Empty;
                    grid_state.grid[player_new_pos[0] as usize][player_new_pos[1] as usize] =
                        TileOccupation::Player;
                }
                PushState::Blocked => {}
                PushState::CanPush => {
                    cloud_control
                        .pushed_clouds
                        .push((player_old_pos, action_direction));
                    match action_direction {
                        CloudDir::Up => cloud_control.next_pushed_clouds.push((
                            player_new_pos,
                            action_direction,
                            PushState::CanPushPlayer,
                        )),
                        CloudDir::Down => cloud_control.next_pushed_clouds.push((
                            player_new_pos,
                            action_direction,
                            PushState::CanPushPlayer,
                        )),
                        CloudDir::Left => cloud_control.next_pushed_clouds.push((
                            player_new_pos,
                            action_direction,
                            PushState::CanPushPlayer,
                        )),
                        CloudDir::Right => cloud_control.next_pushed_clouds.push((
                            player_new_pos,
                            action_direction,
                            PushState::CanPushPlayer,
                        )),
                    };
                }
                _ => {}
            }
        }
    };
}
