#![cfg_attr(debug_assertions, allow(dead_code, unused_imports))]

use crate::actions::Actions;
use crate::actions::GameControl;
use crate::clouds;
use crate::clouds::Cloud;
use crate::clouds::CloudDir;
use crate::clouds::DownCloud;
use crate::clouds::GridPos;
use crate::clouds::LeftCloud;
use crate::clouds::RightCloud;
use crate::clouds::UpCloud;
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
    pub cur_new_cloud: Option<CloudDir>,
    pub cur_cloud_move: Option<CloudDir>,
    cur_cloud: CloudDir,
    timer: Timer,
    sequence: [CloudDir; 4],
}

pub struct GridState {
    grid: [[bool; LEVEL_SIZE as usize]; LEVEL_SIZE as usize],
}

impl Default for GridState {
    fn default() -> Self {
        GridState {
            grid: [[false; LEVEL_SIZE as usize]; LEVEL_SIZE as usize],
        }
    }
}

fn grid_to_vec(grid_pos: [i8; 2]) -> Vec3 {
    Vec3::new(
        (grid_pos[0]) as f32 * TILE_SIZE - (LEVEL_SIZE as f32) / 2. * TILE_SIZE + 0.5 * TILE_SIZE,
        (grid_pos[1]) as f32 * TILE_SIZE - (LEVEL_SIZE as f32) / 2. * TILE_SIZE + 0.5 * TILE_SIZE,
        CLOUD_LAYER,
    )
}

impl GridState {
    fn up_row(&self) -> [[i8; 2]; STAGE_WIDTH as usize] {
        let mut res = [[0i8, 0i8]; STAGE_WIDTH as usize];
        for (ndx, i) in (((LEVEL_SIZE - STAGE_WIDTH) / 2)
            ..=(LEVEL_SIZE - (LEVEL_SIZE - STAGE_WIDTH) / 2) - 1)
            .enumerate()
        {
            println!(
                "{} {} {:?} {:?}",
                { "➤".blue() },
                { ":".blue() },
                { ndx },
                { i }
            );
            res[ndx][0] = i as i8;
            res[ndx][1] = (LEVEL_SIZE - 1) as i8;
        }
        res
    }
    fn down_row(&self) -> [[i8; 2]; STAGE_WIDTH as usize] {
        let mut res = [[0i8, 0i8]; STAGE_WIDTH as usize];
        for (ndx, i) in (((LEVEL_SIZE - STAGE_WIDTH) / 2)
            ..=(LEVEL_SIZE - (LEVEL_SIZE - STAGE_WIDTH) / 2) - 1)
            .enumerate()
        {
            res[ndx][0] = i as i8;
            res[ndx][1] = 0i8;
        }
        res
    }
    fn left_col(&self) -> [[i8; 2]; STAGE_WIDTH as usize] {
        let mut res = [[0i8, 0i8]; STAGE_WIDTH as usize];
        for (ndx, i) in (((LEVEL_SIZE - STAGE_WIDTH) / 2)
            ..=(LEVEL_SIZE - (LEVEL_SIZE - STAGE_WIDTH) / 2) - 1)
            .enumerate()
        {
            res[ndx][0] = 0i8;
            res[ndx][1] = i as i8;
        }
        res
    }
    fn right_col(&self) -> [[i8; 2]; STAGE_WIDTH as usize] {
        let mut res = [[0i8, 0i8]; STAGE_WIDTH as usize];
        for (ndx, i) in (((LEVEL_SIZE - STAGE_WIDTH) / 2)
            ..=(LEVEL_SIZE - (LEVEL_SIZE - STAGE_WIDTH) / 2) - 1)
            .enumerate()
        {
            res[ndx][0] = (LEVEL_SIZE - 1) as i8;
            res[ndx][1] = i as i8;
        }
        res
    }

    pub fn new_cloud(&mut self, border: CloudDir) -> Option<(Vec3, [i8; 2])> {
        let line = match border {
            CloudDir::Down => self.up_row(),
            CloudDir::Left => self.right_col(),
            CloudDir::Right => self.left_col(),
            CloudDir::Up => self.down_row(),
        };

        let non_occupied: Vec<[i8; 2]> =
            line.into_iter().filter(|v| !self.is_occupied(*v)).collect();

        println!(
            "{} {} {:?} {:?}",
            { "➤".blue() },
            { ":".blue() },
            { border },
            { non_occupied.clone() }
        );

        if let Some(pos) = non_occupied.choose(&mut rand::thread_rng()) {
            // Add the cloud to the grid
            self.populate_tile(*pos);
            Some((grid_to_vec(*pos), *pos))
        } else {
            None
        }
    }

    /// Spawn something on the tile, it becomes occupied
    fn populate_tile(&mut self, target_tile: [i8; 2]) {
        self.grid[target_tile[0] as usize][target_tile[1] as usize] = true;
    }

    /// Remove the entity from the previous tile and bring it to the new tile
    ///
    /// Return: whether to despawn the cloud
    fn move_on_grid(&mut self, source_tile: [i8; 2], target_tile: [i8; 2]) -> bool {
        self.grid[source_tile[0] as usize][source_tile[1] as usize] = false;
        if 0 < target_tile[0]
            && target_tile[0] < LEVEL_SIZE as i8
            && 0 < target_tile[1]
            && target_tile[1] < LEVEL_SIZE as i8
        {
            self.grid[target_tile[0] as usize][target_tile[1] as usize] = true;
            false
        } else {
            true
        }
    }

    /// Check whether the tile is occupied
    fn is_occupied(&self, tile: [i8; 2]) -> bool {
        self.grid[tile[0] as usize][tile[1] as usize]
    }
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
                    .label("tick_clock")
                    .before("move_clouds")
                    .before("new_cloud")
                    .with_system(tick_timer)
                    .with_system(set_cloud_direction)
                    .into(),
            )
            .add_system_set(
                ConditionSet::new()
                    .run_in_state(GameState::Playing)
                    .label("move_clouds")
                    .after("tick_clock")
                    .with_system(move_clouds)
                    .with_system(clouds::new_cloud)
                    .into(),
            );
        // .add_system_set(
        //     ConditionSet::new()
        //         .run_in_state(GameState::Playing)
        //         .after("move_clouds")
        //         .after("tick_clock")
        //         .label("new_cloud")
        //         .with_system(clouds::new_cloud)
        //         .into(),
        // );
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
        cur_new_cloud: None,
        cur_cloud_move: None,
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

fn set_cloud_direction(mut cloud_control: ResMut<CloudControl>) {
    if cloud_control.timer.finished() {
        let cloud_dir = Some(cloud_control.next_cloud_direction());
        println!("{} {} {:?}", { "➤".blue() }, { "New Dir:".blue() }, {
            cloud_dir.clone()
        });
        cloud_control.cur_new_cloud = cloud_dir.clone();
        cloud_control.cur_cloud_move = cloud_dir;
    }
}

fn move_clouds(
    mut commands: Commands,
    mut cloud_control: ResMut<CloudControl>,
    mut grid_state: ResMut<GridState>,
    mut left_query: Query<
        (&mut GridPos, &mut Transform, Entity),
        (
            With<LeftCloud>,
            Without<RightCloud>,
            Without<UpCloud>,
            Without<DownCloud>,
        ),
    >,
    mut right_query: Query<
        (&mut GridPos, &mut Transform, Entity),
        (
            With<RightCloud>,
            Without<LeftCloud>,
            Without<UpCloud>,
            Without<DownCloud>,
        ),
    >,
    mut up_query: Query<
        (&mut GridPos, &mut Transform, Entity),
        (
            With<UpCloud>,
            Without<RightCloud>,
            Without<LeftCloud>,
            Without<DownCloud>,
        ),
    >,
    mut down_query: Query<
        (&mut GridPos, &mut Transform, Entity),
        (
            With<DownCloud>,
            Without<RightCloud>,
            Without<UpCloud>,
            Without<LeftCloud>,
        ),
    >,
) {
    // println!("{} {} {:?}", { "➤".blue() }, { "Enter move:".blue() }, {});
    // return early if the timer is off or there is no cloud direction set
    if cloud_control.cur_cloud_move.is_none() {
        return;
    }
    let cloud_dir = cloud_control.cur_cloud_move.unwrap();
    println!("{} {} {:?}", { "➤".red() }, { "Move cloud:".red() }, {
        cloud_dir.clone()
    });

    if cloud_dir == CloudDir::Down {
        for (mut cloud_pos, mut transfo, entity) in down_query.iter_mut() {
            println!("{} {} {:?}", { "➤".red() }, { "down:".red() }, {
                cloud_pos.pos.clone()
            });
            let despawn =
                grid_state.move_on_grid(cloud_pos.pos, [cloud_pos.pos[0], cloud_pos.pos[1] - 1i8]);
            if despawn {
                commands.entity(entity).despawn()
            } else {
                cloud_pos.pos[1] += -1i8;
                println!("{} {} {:?}", { "➤".red() }, { "down:".red() }, {
                    cloud_pos.pos.clone()
                });
                transfo.translation = grid_to_vec(cloud_pos.pos);
            }
        }
    }
    if cloud_dir == CloudDir::Left {
        for (mut cloud_pos, mut transfo, entity) in left_query.iter_mut() {
            println!("{} {} {:?}", { "➤".red() }, { "left:".red() }, {});
            let despawn =
                grid_state.move_on_grid(cloud_pos.pos, [cloud_pos.pos[0] - 1i8, cloud_pos.pos[1]]);
            if despawn {
                commands.entity(entity).despawn()
            } else {
                cloud_pos.pos[0] += -1i8;
                transfo.translation = grid_to_vec(cloud_pos.pos);
            }
        }
    }
    if cloud_dir == CloudDir::Up {
        for (mut cloud_pos, mut transfo, entity) in up_query.iter_mut() {
            println!("{} {} {:?}", { "➤".red() }, { "up:".red() }, {
                cloud_pos.pos.clone()
            });
            let despawn =
                grid_state.move_on_grid(cloud_pos.pos, [cloud_pos.pos[0], cloud_pos.pos[1] + 1i8]);
            if despawn {
                commands.entity(entity).despawn()
            } else {
                cloud_pos.pos[1] += 1i8;
                println!("{} {} {:?}", { "➤".red() }, { "up:".red() }, {
                    cloud_pos.pos.clone()
                });
                transfo.translation = grid_to_vec(cloud_pos.pos);
            }
        }
    }
    if cloud_dir == CloudDir::Right {
        for (mut cloud_pos, mut transfo, entity) in right_query.iter_mut() {
            println!("{} {} {:?}", { "➤".red() }, { "right:".red() }, {});
            let despawn =
                grid_state.move_on_grid(cloud_pos.pos, [cloud_pos.pos[0] + 1i8, cloud_pos.pos[1]]);
            if despawn {
                commands.entity(entity).despawn()
            } else {
                cloud_pos.pos[0] += 1i8;
                transfo.translation = grid_to_vec(cloud_pos.pos);
            }
        }
    }
    cloud_control.cur_cloud_move = None;
}
