#![cfg_attr(debug_assertions, allow(dead_code, unused_imports))]

use crate::actions::Actions;
use crate::actions::GameControl;
use crate::clouds;
use crate::clouds::CloudDir;
use crate::clouds::CLOUD_LAYER;
use crate::loading::TextureAssets;
use crate::player::Player;
use crate::player::INIT_POS;
use crate::player::TILE_SIZE;
use crate::world::LEVEL_SIZE;
use crate::world::STAGE_BL;
use crate::world::STAGE_UR;
use crate::world::STAGE_WIDTH;
use crate::GameState;
use bevy::prelude::*;
use bevy::render::texture::ImageSettings;
use colored::*;
use iyes_loopless::prelude::*;
use rand::seq::SliceRandom;

const MAX_BUFFER_INPUT: usize = 10;
const MOVE_TIMER: f32 = 0.050;
const CLOUD_TIMER: f32 = 0.5;

pub struct LogicPlugin;

/// Contains the info about the player
///
/// The bufferis a FIFO, with the oldest element at index 0.
#[derive(Default)]
pub struct PlayerControl {
    pub player_pos: [i8; 2],
    input_buffer: [GameControl; MAX_BUFFER_INPUT],
    timer: Timer,
}

#[derive(Default)]
pub struct CloudControl {
    pub cur_cloud_dir: Option<CloudDir>,
    cur_cloud: CloudDir,
    timer: Timer,
    sequence: [CloudDir; 4],
}

pub struct GridState {
    left_col: [bool; STAGE_WIDTH as usize],
    right_col: [bool; STAGE_WIDTH as usize],
    up_row: [bool; STAGE_WIDTH as usize],
    down_row: [bool; STAGE_WIDTH as usize],
}

impl Default for GridState {
    fn default() -> Self {
        GridState {
            left_col: [false; STAGE_WIDTH as usize],
            right_col: [false; STAGE_WIDTH as usize],
            up_row: [false; STAGE_WIDTH as usize],
            down_row: [false; STAGE_WIDTH as usize],
        }
    }
}

impl GridState {
    pub fn new_cloud(&mut self, border: CloudDir) -> Option<Vec3> {
        let line = match border {
            CloudDir::Down => &mut self.up_row,
            CloudDir::Left => &mut self.right_col,
            CloudDir::Right => &mut self.left_col,
            CloudDir::Up => &mut self.down_row,
        };

        let non_occupied = line
            .iter()
            .enumerate()
            .filter(|(_, &v)| !v)
            .map(|(index, _)| index)
            .collect::<Vec<_>>();

        if let Some(ndx) = non_occupied.choose(&mut rand::thread_rng()) {
            line[*ndx] = true;

            let (x, y) = match border {
                CloudDir::Down => (
                    ((LEVEL_SIZE - STAGE_WIDTH) as f32 + (*ndx as f32) - 1.5),
                    LEVEL_SIZE as f32 - 0.5,
                ),
                CloudDir::Left => (
                    LEVEL_SIZE as f32 - 0.5,
                    ((LEVEL_SIZE - STAGE_WIDTH) as f32 + (*ndx as f32) - 1.5),
                ),
                CloudDir::Right => (
                    0.5,
                    ((LEVEL_SIZE - STAGE_WIDTH) as f32 + (*ndx as f32) - 1.5),
                ),
                CloudDir::Up => (
                    ((LEVEL_SIZE - STAGE_WIDTH) as f32 + (*ndx as f32) - 1.5),
                    0.5,
                ),
            };

            Some(Vec3 {
                x: TILE_SIZE * (x - (LEVEL_SIZE as f32) / 2.),
                y: TILE_SIZE * (y - (LEVEL_SIZE as f32) / 2.),
                z: CLOUD_LAYER,
            })
        } else {
            None
        }
    }
}

#[derive(Component)]
struct Cloud {
    pos: [i8; 2],
}

/// This plugin handles player related stuff like movement
/// Player logic is only active during the State `GameState::Playing`
impl Plugin for LogicPlugin {
    fn build(&self, app: &mut App) {
        app.add_enter_system(GameState::Playing, set_up_logic)
            .add_system_set(
                ConditionSet::new()
                    .run_in_state(GameState::Playing)
                    .label("fill_player_buffer")
                    .after("pop_player_buffer")
                    .with_system(fill_player_buffer)
                    .into(),
            )
            .add_system_set(
                ConditionSet::new()
                    .run_in_state(GameState::Playing)
                    .before("fill_player_buffer")
                    .label("pop_player_buffer")
                    .with_system(pop_player_buffer)
                    .into(),
            )
            .add_system_set(
                ConditionSet::new()
                    .run_in_state(GameState::Playing)
                    .with_system(tick_timer)
                    .with_system(set_cloud_direction)
                    .with_system(move_clouds)
                    .into(),
            );
    }
}

#[derive(Component, Deref, DerefMut)]
struct AnimationTimer(Timer);

fn set_up_logic(mut commands: Commands) {
    // Create our game rules resource
    commands.insert_resource(PlayerControl {
        player_pos: INIT_POS,
        input_buffer: [GameControl::Idle; MAX_BUFFER_INPUT],
        timer: Timer::from_seconds(MOVE_TIMER, true),
    });
    commands.insert_resource(GridState::default());
    commands.insert_resource(CloudControl {
        cur_cloud_dir: None,
        timer: Timer::from_seconds(CLOUD_TIMER, true),
        cur_cloud: CloudDir::Left,
        sequence: [
            CloudDir::Left,
            CloudDir::Up,
            CloudDir::Right,
            CloudDir::Down,
        ],
    });
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

/// Pop and applies all the player moves when the timer expires
pub fn pop_player_buffer(mut player_control: ResMut<PlayerControl>, time: Res<Time>) {
    // timers gotta be ticked, to work
    player_control.timer.tick(time.delta());

    // if it finished, despawn the bomb
    if player_control.timer.finished() {
        let player_action = player_control.input_buffer[0];
        player_control.input_buffer[0] = GameControl::Idle;
        player_control.input_buffer.rotate_left(1);

        let player_move: [i8; 2] = match player_action {
            GameControl::Down => {
                if player_control.player_pos[1] > (STAGE_BL[1] as i8) {
                    [0, -1]
                } else {
                    [0, 0]
                }
            }
            GameControl::Up => {
                if player_control.player_pos[1] < (STAGE_UR[1] as i8) {
                    [0, 1]
                } else {
                    [0, 0]
                }
            }
            GameControl::Left => {
                if player_control.player_pos[0] > (STAGE_BL[0] as i8) {
                    [-1, 0]
                } else {
                    [0, 0]
                }
            }
            GameControl::Right => {
                if player_control.player_pos[0] < (STAGE_UR[0] as i8) {
                    [1, 0]
                } else {
                    [0, 0]
                }
            }
            GameControl::Idle => [0, 0],
        };
        let player_abs_pos = [
            player_control.player_pos[0] + player_move[0],
            player_control.player_pos[1] + player_move[1],
        ];
        player_control.player_pos = player_abs_pos;

        if player_action != GameControl::Idle {
            info!("pl. pos: {:?}", player_control.player_pos)
        }
    };
}

impl CloudControl {
    fn next_cloud_direction(&mut self) -> CloudDir {
        let cur_cloud = self.cur_cloud;
        let cur_ndx = self.sequence.iter().position(|x| x == &cur_cloud);
        let next_cloud = self.sequence[(cur_ndx.unwrap() + 1) % self.sequence.len()];
        self.cur_cloud = next_cloud;
        next_cloud
    }
}

fn tick_timer(mut cloud_control: ResMut<CloudControl>, time: Res<Time>) {
    // timers gotta be ticked, to work
    cloud_control.timer.tick(time.delta());
}

fn set_cloud_direction(mut cloud_control: ResMut<CloudControl>, time: Res<Time>) {
    if cloud_control.timer.finished() {
        cloud_control.cur_cloud_dir = Some(cloud_control.next_cloud_direction());
    } else {
        cloud_control.cur_cloud_dir = None;
    }
}

fn move_clouds(mut cloud_control: ResMut<CloudControl>, time: Res<Time>) {
    // return early if the timer is off or there is no cloud direction set
    if !cloud_control.timer.finished() || cloud_control.cur_cloud_dir.is_none() {
        return;
    }

    // let cur_cloud_dir =
}
