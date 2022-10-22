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
            )
            .add_system_set(
                ConditionSet::new()
                    .run_in_state(GameState::Playing)
                    .label("push_clouds")
                    .after("move_clouds")
                    .with_system(push_clouds)
                    .into(),
            )
            .add_system_set(
                ConditionSet::new()
                    .run_in_state(GameState::Playing)
                    .label("update_sprites")
                    .after("push_clouds")
                    // .after("move_clouds")
                    .with_system(update_cloud_pos)
                    .with_system(despawn_clouds)
                    .into(),
            );
    }
}

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
    pushed_clouds: Vec<([i8; 2], CloudDir)>,
    next_pushed_clouds: Vec<([i8; 2], CloudDir)>,
}

#[derive(Default, Eq, PartialEq, Debug, Copy, Clone)]
pub enum TileOccupation {
    #[default]
    Empty,
    Player,
    LeftCloud,
    RightCloud,
    UpCloud,
    DownCloud,
    Despawn,
}

#[derive(Default, Eq, PartialEq, Debug, Copy, Clone)]
pub enum PushState {
    #[default]
    Empty,
    Blocked,
    CanPush,
    Despawn,
}

pub struct GridState {
    grid: [[TileOccupation; LEVEL_SIZE as usize]; LEVEL_SIZE as usize],
    // pushed_clouds: Vec<([i8; 2], CloudDir)>,
    // next_pushed_clouds: Vec<([i8; 2], CloudDir)>,
}

impl Default for GridState {
    fn default() -> Self {
        GridState {
            grid: [[TileOccupation::Empty; LEVEL_SIZE as usize]; LEVEL_SIZE as usize],
            // pushed_clouds: vec![],
            // next_pushed_clouds: vec![],
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

    pub fn is_out_of_range(&self, tile: [i8; 2]) -> bool {
        0 > tile[0] || tile[0] >= LEVEL_SIZE as i8 || 0 > tile[1] || tile[1] >= LEVEL_SIZE as i8
    }

    pub fn new_cloud(&mut self, border: CloudDir) -> Option<(Vec3, [i8; 2])> {
        let line = match border {
            CloudDir::Down => self.up_row(),
            CloudDir::Left => self.right_col(),
            CloudDir::Right => self.left_col(),
            CloudDir::Up => self.down_row(),
        };

        let non_occupied: Vec<[i8; 2]> = line
            .into_iter()
            .filter(|v| (self.is_occupied(*v, Some(border)) == PushState::Empty))
            .collect();
        if let Some(pos) = non_occupied.choose(&mut rand::thread_rng()) {
            // Add the cloud to the grid
            self.populate_tile_with_cloud(
                *pos,
                match border {
                    CloudDir::Down => TileOccupation::DownCloud,
                    CloudDir::Up => TileOccupation::UpCloud,
                    CloudDir::Left => TileOccupation::LeftCloud,
                    CloudDir::Right => TileOccupation::RightCloud,
                },
            );
            Some((grid_to_vec(*pos), *pos))
        } else {
            None
        }
    }

    /// Spawn something on the tile, it becomes occupied
    fn populate_tile_with_cloud(&mut self, target_tile: [i8; 2], object: TileOccupation) {
        self.grid[target_tile[0] as usize][target_tile[1] as usize] = object;
    }

    /// Remove the entity from the previous tile and bring it to the new tile
    ///
    /// Return: whether to despawn the cloud
    fn move_on_grid(&mut self, source_tile: [i8; 2], target_tile: [i8; 2], object: TileOccupation) {
        if self.is_out_of_range(target_tile) {
            self.grid[source_tile[0] as usize][source_tile[1] as usize] = TileOccupation::Despawn;
        } else {
            self.grid[source_tile[0] as usize][source_tile[1] as usize] = TileOccupation::Empty;
            self.grid[target_tile[0] as usize][target_tile[1] as usize] = object;
        }
    }

    /// Check whether the next tile is occupied
    fn is_occupied(&self, tile: [i8; 2], dir: Option<CloudDir>) -> PushState {
        if self.is_out_of_range(tile) {
            return PushState::Despawn;
        }
        if self.grid[tile[0] as usize][tile[1] as usize] == TileOccupation::Empty {
            return PushState::Empty;
        }
        if let Some(dir) = dir {
            let next_tile = match dir {
                CloudDir::Down => [tile[0], tile[1] - 1],
                CloudDir::Up => [tile[0], tile[1] + 1],
                CloudDir::Left => [tile[0] - 1, tile[1]],
                CloudDir::Right => [tile[0] + 1, tile[1]],
            };

            let next_in_range = self.is_out_of_range(next_tile);
            if !next_in_range {
                let next_cloud_dir = self.grid[next_tile[0] as usize][next_tile[1] as usize];
                return match dir {
                    CloudDir::Down => match next_cloud_dir {
                        TileOccupation::UpCloud => PushState::Blocked,
                        _ => PushState::CanPush,
                    },
                    CloudDir::Up => match next_cloud_dir {
                        TileOccupation::DownCloud => PushState::Blocked,
                        _ => PushState::CanPush,
                    },
                    CloudDir::Left => match next_cloud_dir {
                        TileOccupation::RightCloud => PushState::Blocked,
                        _ => PushState::CanPush,
                    },
                    CloudDir::Right => match next_cloud_dir {
                        TileOccupation::LeftCloud => PushState::Blocked,
                        _ => PushState::CanPush,
                    },
                };
            } else {
                return PushState::Blocked;
            }
        }
        PushState::Blocked
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
        ..Default::default()
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
    mut cloud_control: ResMut<CloudControl>,
    mut grid_state: ResMut<GridState>,
    mut left_query: Query<
        &mut GridPos,
        (
            With<LeftCloud>,
            Without<RightCloud>,
            Without<UpCloud>,
            Without<DownCloud>,
        ),
    >,
    mut right_query: Query<
        &mut GridPos,
        (
            With<RightCloud>,
            Without<LeftCloud>,
            Without<UpCloud>,
            Without<DownCloud>,
        ),
    >,
    mut up_query: Query<
        &mut GridPos,
        (
            With<UpCloud>,
            Without<RightCloud>,
            Without<LeftCloud>,
            Without<DownCloud>,
        ),
    >,
    mut down_query: Query<
        &mut GridPos,
        (
            With<DownCloud>,
            Without<RightCloud>,
            Without<UpCloud>,
            Without<LeftCloud>,
        ),
    >,
) {
    // return early if the timer is off or there is no cloud direction set
    if cloud_control.cur_cloud_move.is_none() {
        return;
    }
    let cloud_dir = cloud_control.cur_cloud_move.unwrap();
    cloud_control.pushed_clouds = vec![];
    cloud_control.next_pushed_clouds = vec![];

    if cloud_dir == CloudDir::Down {
        for mut cloud_pos in down_query.iter_mut() {
            let next_tile_push =
                grid_state.is_occupied([cloud_pos.pos[0], cloud_pos.pos[1] - 1i8], Some(cloud_dir));

            match next_tile_push {
                PushState::Blocked => continue,
                PushState::Despawn => {
                    grid_state.grid[cloud_pos.pos[0] as usize][cloud_pos.pos[1] as usize] =
                        TileOccupation::Despawn
                }
                PushState::Empty => {
                    grid_state.move_on_grid(
                        cloud_pos.pos,
                        [cloud_pos.pos[0], cloud_pos.pos[1] - 1i8],
                        TileOccupation::DownCloud,
                    );
                    cloud_pos.pos[1] += -1i8;
                }
                PushState::CanPush => {
                    cloud_control.pushed_clouds.push((cloud_pos.pos, cloud_dir));
                    cloud_control
                        .next_pushed_clouds
                        .push(([cloud_pos.pos[0], cloud_pos.pos[1] - 1], cloud_dir));
                }
            }
        }
    };
    if cloud_dir == CloudDir::Left {
        for mut cloud_pos in left_query.iter_mut() {
            let next_tile_push =
                grid_state.is_occupied([cloud_pos.pos[0] - 1i8, cloud_pos.pos[1]], Some(cloud_dir));
            match next_tile_push {
                PushState::Blocked => continue,
                PushState::Despawn => {
                    grid_state.grid[cloud_pos.pos[0] as usize][cloud_pos.pos[1] as usize] =
                        TileOccupation::Despawn
                }
                PushState::Empty => {
                    grid_state.move_on_grid(
                        cloud_pos.pos,
                        [cloud_pos.pos[0] - 1i8, cloud_pos.pos[1]],
                        TileOccupation::DownCloud,
                    );
                    cloud_pos.pos[0] += -1i8;
                }
                PushState::CanPush => {
                    cloud_control.pushed_clouds.push((cloud_pos.pos, cloud_dir));
                    cloud_control
                        .next_pushed_clouds
                        .push(([cloud_pos.pos[0] - 1, cloud_pos.pos[1]], cloud_dir));
                }
            }
        }
    };
    if cloud_dir == CloudDir::Up {
        for mut cloud_pos in up_query.iter_mut() {
            let next_tile_push =
                grid_state.is_occupied([cloud_pos.pos[0], cloud_pos.pos[1] + 1i8], Some(cloud_dir));
            match next_tile_push {
                PushState::Blocked => continue,
                PushState::Despawn => {
                    grid_state.grid[cloud_pos.pos[0] as usize][cloud_pos.pos[1] as usize] =
                        TileOccupation::Despawn
                }
                PushState::Empty => {
                    grid_state.move_on_grid(
                        cloud_pos.pos,
                        [cloud_pos.pos[0], cloud_pos.pos[1] + 1i8],
                        TileOccupation::UpCloud,
                    );
                    cloud_pos.pos[1] += 1i8;
                }
                PushState::CanPush => {
                    cloud_control.pushed_clouds.push((cloud_pos.pos, cloud_dir));
                    cloud_control
                        .next_pushed_clouds
                        .push(([cloud_pos.pos[0], cloud_pos.pos[1] + 1], cloud_dir));
                }
            }
        }
    };
    if cloud_dir == CloudDir::Right {
        for mut cloud_pos in right_query.iter_mut() {
            let next_tile_push =
                grid_state.is_occupied([cloud_pos.pos[0] + 1i8, cloud_pos.pos[1]], Some(cloud_dir));
            match next_tile_push {
                PushState::Blocked => continue,
                PushState::Despawn => {
                    grid_state.grid[cloud_pos.pos[0] as usize][cloud_pos.pos[1] as usize] =
                        TileOccupation::Despawn
                }
                PushState::Empty => {
                    grid_state.move_on_grid(
                        cloud_pos.pos,
                        [cloud_pos.pos[0] + 1i8, cloud_pos.pos[1]],
                        TileOccupation::RightCloud,
                    );
                    cloud_pos.pos[0] += 1i8;
                }
                PushState::CanPush => {
                    cloud_control.pushed_clouds.push((cloud_pos.pos, cloud_dir));
                    cloud_control
                        .next_pushed_clouds
                        .push(([cloud_pos.pos[0] + 1, cloud_pos.pos[1]], cloud_dir));
                }
            }
        }
    }
}

/// Deal with the cloud which need to be pushed. At this stage, one already
/// knows that the tile N+2 is empty to push the cloud
fn push_clouds(
    mut cloud_control: ResMut<CloudControl>,
    mut grid_state: ResMut<GridState>,
    mut query: Query<
        (&Cloud, &mut GridPos),
        (
            With<LeftCloud>,
            With<RightCloud>,
            With<UpCloud>,
            With<DownCloud>,
        ),
    >,
) {
    if cloud_control.next_pushed_clouds.len() > 0 {
        println!("{} {} {:?}", { "➤".blue() }, { "AAA:".blue() }, {
            cloud_control.next_pushed_clouds.clone()
        });
    }
    if cloud_control.pushed_clouds.len() > 0 {
        println!("{} {} {:?}", { "➤".blue() }, { "BBB:".blue() }, {
            cloud_control.next_pushed_clouds.clone()
        });
    }
    // Move first the next cloud "pushed":
    for (pos, dir) in cloud_control.next_pushed_clouds.drain(..) {
        for (cloud, mut cloud_pos) in query.iter_mut() {
            if cloud_pos.pos == pos {
                println!("{} {} {:?}", { "➤".red() }, { "CCC:".red() }, {
                    "Yeah, push"
                });
                match dir {
                    CloudDir::Down => {
                        grid_state.move_on_grid(
                            cloud_pos.pos,
                            [cloud_pos.pos[0], cloud_pos.pos[1] - 1i8],
                            match cloud.dir {
                                CloudDir::Down => TileOccupation::DownCloud,
                                CloudDir::Up => TileOccupation::UpCloud,
                                CloudDir::Left => TileOccupation::LeftCloud,
                                CloudDir::Right => TileOccupation::RightCloud,
                            },
                        );
                        cloud_pos.pos[1] += -1i8;
                    }
                    CloudDir::Left => {
                        grid_state.move_on_grid(
                            cloud_pos.pos,
                            [cloud_pos.pos[0] - 1i8, cloud_pos.pos[1]],
                            match cloud.dir {
                                CloudDir::Down => TileOccupation::DownCloud,
                                CloudDir::Up => TileOccupation::UpCloud,
                                CloudDir::Left => TileOccupation::LeftCloud,
                                CloudDir::Right => TileOccupation::RightCloud,
                            },
                        );
                        cloud_pos.pos[0] += -1i8;
                    }
                    CloudDir::Right => {
                        grid_state.move_on_grid(
                            cloud_pos.pos,
                            [cloud_pos.pos[0] + 1i8, cloud_pos.pos[1]],
                            match cloud.dir {
                                CloudDir::Down => TileOccupation::DownCloud,
                                CloudDir::Up => TileOccupation::UpCloud,
                                CloudDir::Left => TileOccupation::LeftCloud,
                                CloudDir::Right => TileOccupation::RightCloud,
                            },
                        );
                        cloud_pos.pos[0] += 1i8;
                    }
                    CloudDir::Up => {
                        grid_state.move_on_grid(
                            cloud_pos.pos,
                            [cloud_pos.pos[0], cloud_pos.pos[1] + 1i8],
                            match cloud.dir {
                                CloudDir::Down => TileOccupation::DownCloud,
                                CloudDir::Up => TileOccupation::UpCloud,
                                CloudDir::Left => TileOccupation::LeftCloud,
                                CloudDir::Right => TileOccupation::RightCloud,
                            },
                        );
                        cloud_pos.pos[1] += 1i8;
                    }
                }
            }
        }
    }

    // Then move the actual clouds pushing the other one:
    for (pos, dir) in cloud_control.next_pushed_clouds.drain(..) {
        for (_, mut cloud_pos) in query.iter_mut() {
            if cloud_pos.pos == pos {
                match dir {
                    CloudDir::Down => {
                        grid_state.move_on_grid(
                            cloud_pos.pos,
                            [cloud_pos.pos[0], cloud_pos.pos[1] - 1i8],
                            TileOccupation::DownCloud,
                        );
                        cloud_pos.pos[1] += -1i8;
                    }
                    CloudDir::Left => {
                        grid_state.move_on_grid(
                            cloud_pos.pos,
                            [cloud_pos.pos[0] - 1i8, cloud_pos.pos[1]],
                            TileOccupation::DownCloud,
                        );
                        cloud_pos.pos[0] += -1i8;
                    }
                    CloudDir::Right => {
                        grid_state.move_on_grid(
                            cloud_pos.pos,
                            [cloud_pos.pos[0] + 1i8, cloud_pos.pos[1]],
                            TileOccupation::DownCloud,
                        );
                        cloud_pos.pos[0] += 1i8;
                    }
                    CloudDir::Up => {
                        grid_state.move_on_grid(
                            cloud_pos.pos,
                            [cloud_pos.pos[0], cloud_pos.pos[1] + 1i8],
                            TileOccupation::UpCloud,
                        );
                        cloud_pos.pos[1] += 1i8;
                    }
                }
            }
        }
    }
}

fn despawn_clouds(
    mut commands: Commands,
    grid_state: ResMut<GridState>,
    mut query: Query<(&mut GridPos, Entity), (With<Cloud>,)>,
) {
    for (cloud_pos, entity) in query.iter_mut() {
        if grid_state.grid[cloud_pos.pos[0] as usize][cloud_pos.pos[1] as usize]
            == TileOccupation::Despawn
        {
            commands.entity(entity).despawn();
        }
    }
}

fn update_cloud_pos(
    mut cloud_control: ResMut<CloudControl>,
    mut left_query: Query<
        (&mut GridPos, &mut Transform),
        (
            With<LeftCloud>,
            Without<RightCloud>,
            Without<UpCloud>,
            Without<DownCloud>,
        ),
    >,
    mut right_query: Query<
        (&mut GridPos, &mut Transform),
        (
            With<RightCloud>,
            Without<LeftCloud>,
            Without<UpCloud>,
            Without<DownCloud>,
        ),
    >,
    mut up_query: Query<
        (&mut GridPos, &mut Transform),
        (
            With<UpCloud>,
            Without<RightCloud>,
            Without<LeftCloud>,
            Without<DownCloud>,
        ),
    >,
    mut down_query: Query<
        (&mut GridPos, &mut Transform),
        (
            With<DownCloud>,
            Without<RightCloud>,
            Without<UpCloud>,
            Without<LeftCloud>,
        ),
    >,
) {
    // return early if the timer is off or there is no cloud direction set
    if cloud_control.cur_cloud_move.is_none() {
        return;
    }
    let cloud_dir = cloud_control.cur_cloud_move.unwrap();

    if cloud_dir == CloudDir::Down {
        for (cloud_pos, mut transfo) in down_query.iter_mut() {
            transfo.translation = grid_to_vec(cloud_pos.pos);
        }
    }
    if cloud_dir == CloudDir::Left {
        for (cloud_pos, mut transfo) in left_query.iter_mut() {
            transfo.translation = grid_to_vec(cloud_pos.pos);
        }
    }
    if cloud_dir == CloudDir::Up {
        for (cloud_pos, mut transfo) in up_query.iter_mut() {
            transfo.translation = grid_to_vec(cloud_pos.pos);
        }
    }
    if cloud_dir == CloudDir::Right {
        for (cloud_pos, mut transfo) in right_query.iter_mut() {
            transfo.translation = grid_to_vec(cloud_pos.pos);
        }
    }
    cloud_control.cur_cloud_move = None;
}
