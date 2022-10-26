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
const MOVE_TIMER: f32 = 0.020;
// Multiple of the move timer:
const SPAWN_FREQUENCY: u8 = 4;
// Offset for delaying cloud spawning depending on the direction:
const SPAWN_OFFSET: [u8; 4] = [0, 1, 0, 1];
const CLOUD_TIMER: f32 = 0.4;
const SEQUENCE: [CloudDir; 4] = [
    CloudDir::Left,
    CloudDir::Up,
    CloudDir::Right,
    CloudDir::Down,
];
pub struct LogicPlugin;

/// This plugin handles player related stuff like movement
/// Player logic is only active during the State `GameState::Playing`
impl Plugin for LogicPlugin {
    fn build(&self, app: &mut App) {
        app.add_enter_system(GameState::Playing, set_up_logic)
            .add_system_set(
                ConditionSet::new()
                    .run_in_state(GameState::Playing)
                    .label("tick_clock")
                    .before("move_clouds")
                    .with_system(tick_timer)
                    .with_system(set_cloud_direction)
                    .into(),
            )
            .add_system_set(
                ConditionSet::new()
                    .run_in_state(GameState::Playing)
                    .label("fill_player_buffer")
                    .with_system(fill_player_buffer)
                    .into(),
            )
            .add_system_set(
                ConditionSet::new()
                    .run_in_state(GameState::Playing)
                    .label("pop_player_buffer")
                    .with_system(pop_player_buffer)
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
    move_timer: Timer,
    sequence: [CloudDir; 4],
    spawn_counter: [u8; 4],
    pushed_clouds: Vec<([i8; 2], CloudDir)>,
    next_pushed_clouds: Vec<([i8; 2], CloudDir, PushState)>,
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
    PushOver,
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
            .filter(|v| (self.is_occupied(*v, border) == PushState::Empty))
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
        // assert!(
        //     self.grid[target_tile[0] as usize][target_tile[1] as usize] == TileOccupation::Empty
        // );
        if self.is_out_of_range(target_tile) {
            self.grid[source_tile[0] as usize][source_tile[1] as usize] = TileOccupation::Despawn;
        } else {
            self.grid[source_tile[0] as usize][source_tile[1] as usize] = TileOccupation::Empty;
            self.grid[target_tile[0] as usize][target_tile[1] as usize] = object;
        }
    }

    /// Check whether the next tile is occupied. Here the function is called on
    /// the tile N+1 such that we check the tile N+2
    fn is_occupied(&self, tile: [i8; 2], dir: CloudDir) -> PushState {
        if self.is_out_of_range(tile) {
            return PushState::Despawn;
        }
        let target_tile_occ = self.grid[tile[0] as usize][tile[1] as usize];
        // Nothing on the target tile, you are good to go:
        if target_tile_occ == TileOccupation::Empty {
            return PushState::Empty;
        }

        // Check the N+2 tile (behind the target tile):
        let np2_tile = match dir {
            CloudDir::Down => [tile[0], tile[1] - 1],
            CloudDir::Up => [tile[0], tile[1] + 1],
            CloudDir::Left => [tile[0] - 1, tile[1]],
            CloudDir::Right => [tile[0] + 1, tile[1]],
        };

        let np2_in_range = !self.is_out_of_range(np2_tile);

        // Here deal with the case where we are on the edge of the board.
        if !np2_in_range {
            return match dir {
                CloudDir::Down => match target_tile_occ {
                    TileOccupation::UpCloud => PushState::Blocked,
                    _ => PushState::PushOver,
                },
                CloudDir::Up => match target_tile_occ {
                    TileOccupation::DownCloud => PushState::Blocked,
                    _ => PushState::PushOver,
                },
                CloudDir::Left => match target_tile_occ {
                    TileOccupation::RightCloud => PushState::Blocked,
                    _ => PushState::PushOver,
                },
                CloudDir::Right => match target_tile_occ {
                    TileOccupation::LeftCloud => PushState::Blocked,
                    _ => PushState::PushOver,
                },
            };
        }

        let next_tile_occ = self.grid[np2_tile[0] as usize][np2_tile[1] as usize];
        let tile_np2_occupied = !(matches!(
            next_tile_occ,
            TileOccupation::Empty | TileOccupation::Despawn
        ));

        // Case where there is something behind, just forget it
        if tile_np2_occupied {
            PushState::Blocked
        } else {
            // case where there is something behind, it depends on the target tile
            match dir {
                CloudDir::Down => match target_tile_occ {
                    TileOccupation::UpCloud => PushState::Blocked,
                    TileOccupation::Player => {
                        if tile[1] <= ((LEVEL_SIZE - STAGE_WIDTH) / 2) as i8 {
                            PushState::Blocked
                        } else {
                            PushState::CanPush
                        }
                    }
                    _ => PushState::CanPush,
                },
                CloudDir::Up => match target_tile_occ {
                    TileOccupation::DownCloud => PushState::Blocked,
                    TileOccupation::Player => {
                        if tile[1] >= (STAGE_WIDTH + (LEVEL_SIZE - STAGE_WIDTH) / 2 - 1) as i8 {
                            PushState::Blocked
                        } else {
                            PushState::CanPush
                        }
                    }
                    _ => PushState::CanPush,
                },
                CloudDir::Left => match target_tile_occ {
                    TileOccupation::RightCloud => PushState::Blocked,
                    TileOccupation::Player => {
                        if tile[0] <= ((LEVEL_SIZE - STAGE_WIDTH) / 2) as i8 {
                            PushState::Blocked
                        } else {
                            PushState::CanPush
                        }
                    }
                    _ => PushState::CanPush,
                },
                CloudDir::Right => match target_tile_occ {
                    TileOccupation::LeftCloud => PushState::Blocked,
                    TileOccupation::Player => {
                        if tile[0] >= (STAGE_WIDTH + (LEVEL_SIZE - STAGE_WIDTH) / 2 - 1) as i8 {
                            PushState::Blocked
                        } else {
                            PushState::CanPush
                        }
                    }
                    _ => PushState::CanPush,
                },
            }
        }
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
        move_timer: Timer::from_seconds(CLOUD_TIMER, true),
        cur_cloud: CloudDir::Left,
        sequence: SEQUENCE,
        spawn_counter: [
            SPAWN_OFFSET[0],
            SPAWN_OFFSET[1],
            SPAWN_OFFSET[2],
            SPAWN_OFFSET[3],
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
pub fn pop_player_buffer(
    mut cloud_control: ResMut<CloudControl>,
    mut grid_state: ResMut<GridState>,
    mut player_control: ResMut<PlayerControl>,
    time: Res<Time>,
) {
    // timers gotta be ticked, to work
    player_control.timer.tick(time.delta());

    // if it finished, despawn the bomb
    if player_control.timer.finished() {
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
                        player_control.player_pos.clone()
                    };
                    let dir = CloudDir::Down;
                    (new_pos, dir, grid_state.is_occupied(new_pos, dir))
                }
                GameControl::Up => {
                    let new_pos = if player_control.player_pos[1] < (STAGE_UR[1] as i8) {
                        [
                            player_control.player_pos[0],
                            player_control.player_pos[1] + 1,
                        ]
                    } else {
                        player_control.player_pos.clone()
                    };
                    let dir = CloudDir::Up;
                    (new_pos, dir, grid_state.is_occupied(new_pos, dir))
                }
                GameControl::Left => {
                    let new_pos = if player_control.player_pos[0] > (STAGE_BL[0] as i8) {
                        [
                            player_control.player_pos[0] - 1,
                            player_control.player_pos[1],
                        ]
                    } else {
                        player_control.player_pos.clone()
                    };
                    let dir = CloudDir::Left;
                    (new_pos, dir, grid_state.is_occupied(new_pos, dir))
                }
                GameControl::Right => {
                    let new_pos = if player_control.player_pos[0] < (STAGE_UR[0] as i8) {
                        [
                            player_control.player_pos[0] + 1,
                            player_control.player_pos[1],
                        ]
                    } else {
                        player_control.player_pos.clone()
                    };
                    let dir = CloudDir::Right;
                    (new_pos, dir, grid_state.is_occupied(new_pos, dir))
                }
                GameControl::Idle => (
                    player_control.player_pos.clone(),
                    CloudDir::Right,
                    PushState::Blocked,
                ),
            };
        let player_old_pos = player_control.player_pos.clone();
        // let player_new_pos = [
        //     player_control.player_pos[0] + player_move[0],
        //     player_control.player_pos[1] + player_move[1],
        // ];

        // (player_new_pos, action_direction, push_state)
        if player_action != GameControl::Idle {
            match push_state {
                PushState::Empty => {
                    player_control.player_pos = player_new_pos;
                    info!("pl. pos: {:?}", player_control.player_pos);
                    grid_state.grid[player_old_pos[0] as usize][player_old_pos[1] as usize] =
                        TileOccupation::Empty;
                    grid_state.grid[player_new_pos[0] as usize][player_new_pos[1] as usize] =
                        TileOccupation::Player;
                }
                PushState::Blocked => {}
                PushState::CanPush => {
                    // player_control.player_pos = player_new_pos;
                    // info!("pl. pos: {:?}", player_control.player_pos);
                    // grid_state.grid[player_old_pos[0] as usize][player_old_pos[1] as usize] =
                    //     TileOccupation::Empty;
                    // grid_state.grid[player_new_pos[0] as usize][player_new_pos[1] as usize] =
                    //     TileOccupation::Player;
                    cloud_control
                        .pushed_clouds
                        .push((player_old_pos, action_direction));
                    match action_direction {
                        CloudDir::Up => cloud_control.next_pushed_clouds.push((
                            player_new_pos,
                            action_direction,
                            PushState::CanPush,
                        )),
                        CloudDir::Down => cloud_control.next_pushed_clouds.push((
                            player_new_pos,
                            action_direction,
                            PushState::CanPush,
                        )),
                        CloudDir::Left => cloud_control.next_pushed_clouds.push((
                            player_new_pos,
                            action_direction,
                            PushState::CanPush,
                        )),
                        CloudDir::Right => cloud_control.next_pushed_clouds.push((
                            player_new_pos,
                            action_direction,
                            PushState::CanPush,
                        )),
                    };
                    // match action_direction {
                    //     CloudDir::Up => cloud_control.next_pushed_clouds.push((
                    //         [player_new_pos[0], player_new_pos[1] + 1],
                    //         action_direction,
                    //         PushState::CanPush,
                    //     )),
                    //     CloudDir::Down => cloud_control.next_pushed_clouds.push((
                    //         [player_new_pos[0], player_new_pos[1] - 1],
                    //         action_direction,
                    //         PushState::CanPush,
                    //     )),
                    //     CloudDir::Left => cloud_control.next_pushed_clouds.push((
                    //         [player_new_pos[0] - 1, player_new_pos[1]],
                    //         action_direction,
                    //         PushState::CanPush,
                    //     )),
                    //     CloudDir::Right => cloud_control.next_pushed_clouds.push((
                    //         [player_new_pos[0] + 1, player_new_pos[1]],
                    //         action_direction,
                    //         PushState::CanPush,
                    //     )),
                    // };
                }
                _ => {}
            }
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
    cloud_control.move_timer.tick(time.delta());
}

fn dir_index(cloud_dir: CloudDir) -> usize {
    SEQUENCE.iter().position(|&x| x == cloud_dir).unwrap()
}

fn set_cloud_direction(mut cloud_control: ResMut<CloudControl>) {
    if cloud_control.move_timer.finished() {
        let cloud_dir = Some(cloud_control.next_cloud_direction());
        info!("cloud dir.: {:?}", cloud_dir.unwrap());
        cloud_control.cur_cloud_move = cloud_dir;

        let uw_cloud_dir = cloud_dir.unwrap();
        cloud_control.spawn_counter[dir_index(uw_cloud_dir) as usize] =
            (cloud_control.spawn_counter[dir_index(uw_cloud_dir) as usize] + 1) % SPAWN_FREQUENCY;
        if cloud_control.spawn_counter[dir_index(uw_cloud_dir) as usize] == 0 {
            cloud_control.cur_new_cloud = cloud_dir;
        } else {
            cloud_control.cur_new_cloud = None
        }
    }
}

#[allow(clippy::type_complexity)]
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

    if cloud_dir == CloudDir::Down {
        for mut cloud_pos in down_query.iter_mut() {
            let next_tile_push =
                grid_state.is_occupied([cloud_pos.pos[0], cloud_pos.pos[1] - 1i8], cloud_dir);

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
                    cloud_control.next_pushed_clouds.push((
                        [cloud_pos.pos[0], cloud_pos.pos[1] - 1],
                        cloud_dir,
                        PushState::CanPush,
                    ));
                }
                PushState::PushOver => {
                    cloud_control.pushed_clouds.push((cloud_pos.pos, cloud_dir));
                    cloud_control.next_pushed_clouds.push((
                        [cloud_pos.pos[0], cloud_pos.pos[1] - 1],
                        cloud_dir,
                        PushState::PushOver,
                    ));
                }
            }
        }
    };
    if cloud_dir == CloudDir::Left {
        for mut cloud_pos in left_query.iter_mut() {
            let next_tile_push =
                grid_state.is_occupied([cloud_pos.pos[0] - 1i8, cloud_pos.pos[1]], cloud_dir);
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
                        TileOccupation::LeftCloud,
                    );
                    cloud_pos.pos[0] += -1i8;
                }
                PushState::CanPush => {
                    cloud_control.pushed_clouds.push((cloud_pos.pos, cloud_dir));
                    cloud_control.next_pushed_clouds.push((
                        [cloud_pos.pos[0] - 1, cloud_pos.pos[1]],
                        cloud_dir,
                        PushState::CanPush,
                    ));
                }
                PushState::PushOver => {
                    cloud_control.pushed_clouds.push((cloud_pos.pos, cloud_dir));
                    cloud_control.next_pushed_clouds.push((
                        [cloud_pos.pos[0] - 1, cloud_pos.pos[1]],
                        cloud_dir,
                        PushState::PushOver,
                    ));
                }
            }
        }
    };
    if cloud_dir == CloudDir::Up {
        for mut cloud_pos in up_query.iter_mut() {
            let next_tile_push =
                grid_state.is_occupied([cloud_pos.pos[0], cloud_pos.pos[1] + 1i8], cloud_dir);
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
                    cloud_control.next_pushed_clouds.push((
                        [cloud_pos.pos[0], cloud_pos.pos[1] + 1],
                        cloud_dir,
                        PushState::CanPush,
                    ));
                }
                PushState::PushOver => {
                    cloud_control.pushed_clouds.push((cloud_pos.pos, cloud_dir));
                    cloud_control.next_pushed_clouds.push((
                        [cloud_pos.pos[0], cloud_pos.pos[1] + 1],
                        cloud_dir,
                        PushState::PushOver,
                    ));
                }
            }
        }
    };
    if cloud_dir == CloudDir::Right {
        for mut cloud_pos in right_query.iter_mut() {
            let next_tile_push =
                grid_state.is_occupied([cloud_pos.pos[0] + 1i8, cloud_pos.pos[1]], cloud_dir);
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
                    cloud_control.next_pushed_clouds.push((
                        [cloud_pos.pos[0] + 1, cloud_pos.pos[1]],
                        cloud_dir,
                        PushState::CanPush,
                    ));
                }
                PushState::PushOver => {
                    cloud_control.pushed_clouds.push((cloud_pos.pos, cloud_dir));
                    cloud_control.next_pushed_clouds.push((
                        [cloud_pos.pos[0] + 1, cloud_pos.pos[1]],
                        cloud_dir,
                        PushState::PushOver,
                    ));
                }
            }
        }
    }
    cloud_control.cur_cloud_move = None;
}

// WIP: make the player push clouds

/// Deal with the cloud which need to be pushed. At this stage, one already
/// knows that the tile N+2 is empty to push the cloud
#[allow(clippy::type_complexity)]
fn push_clouds(
    mut commands: Commands,
    mut cloud_control: ResMut<CloudControl>,
    mut player_control: ResMut<PlayerControl>,
    mut grid_state: ResMut<GridState>,
    mut query: Query<
        (&Cloud, &mut GridPos, Entity),
        (
            Or<(
                With<LeftCloud>,
                With<RightCloud>,
                With<UpCloud>,
                With<DownCloud>,
            )>,
        ),
    >,
) {
    // Move first the next cloud "pushed":
    for (pos, dir, push_type) in cloud_control.next_pushed_clouds.drain(..) {
        // First push the player:
        if player_control.player_pos == pos {
            match dir {
                CloudDir::Up => {
                    player_control.player_pos = [
                        player_control.player_pos[0],
                        player_control.player_pos[1] + 1,
                    ]
                }
                CloudDir::Down => {
                    player_control.player_pos = [
                        player_control.player_pos[0],
                        player_control.player_pos[1] - 1,
                    ]
                }
                CloudDir::Left => {
                    player_control.player_pos = [
                        player_control.player_pos[0] - 1,
                        player_control.player_pos[1],
                    ]
                }
                CloudDir::Right => {
                    player_control.player_pos = [
                        player_control.player_pos[0] + 1,
                        player_control.player_pos[1],
                    ]
                }
            };
            grid_state.grid[pos[0] as usize][pos[1] as usize] = TileOccupation::Empty;
            grid_state.grid[player_control.player_pos[0] as usize]
                [player_control.player_pos[1] as usize] = TileOccupation::Player;
            continue;
        }

        for (cloud, mut cloud_pos, entity) in query.iter_mut() {
            if cloud_pos.pos == pos {
                // If cloud to be pushed out of the board, despawn it instantly:
                if push_type == PushState::PushOver {
                    // TODO: check that it actuall works:
                    commands.entity(entity).despawn();
                    grid_state.grid[cloud_pos.pos[0] as usize][cloud_pos.pos[1] as usize] =
                        TileOccupation::Empty;
                    continue;
                };

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
    for (pos, dir) in cloud_control.pushed_clouds.drain(..) {
        // First move the player:
        // First push the player:
        if player_control.player_pos == pos {
            match dir {
                CloudDir::Up => {
                    player_control.player_pos = [
                        player_control.player_pos[0],
                        player_control.player_pos[1] + 1,
                    ]
                }
                CloudDir::Down => {
                    player_control.player_pos = [
                        player_control.player_pos[0],
                        player_control.player_pos[1] - 1,
                    ]
                }
                CloudDir::Left => {
                    player_control.player_pos = [
                        player_control.player_pos[0] - 1,
                        player_control.player_pos[1],
                    ]
                }
                CloudDir::Right => {
                    player_control.player_pos = [
                        player_control.player_pos[0] + 1,
                        player_control.player_pos[1],
                    ]
                }
            };
            grid_state.grid[pos[0] as usize][pos[1] as usize] = TileOccupation::Empty;
            grid_state.grid[player_control.player_pos[0] as usize]
                [player_control.player_pos[1] as usize] = TileOccupation::Player;
            continue;
        }

        for (_, mut cloud_pos, _) in query.iter_mut() {
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

fn update_cloud_pos(mut query: Query<(&mut GridPos, &mut Transform), (With<Cloud>,)>) {
    for (cloud_pos, mut transfo) in query.iter_mut() {
        transfo.translation = grid_to_vec(cloud_pos.pos);
    }
}
// fn update_cloud_pos(
//     mut cloud_control: ResMut<CloudControl>,
//     mut left_query: Query<
//         (&mut GridPos, &mut Transform),
//         (
//             With<LeftCloud>,
//             Without<RightCloud>,
//             Without<UpCloud>,
//             Without<DownCloud>,
//         ),
//     >,
//     mut right_query: Query<
//         (&mut GridPos, &mut Transform),
//         (
//             With<RightCloud>,
//             Without<LeftCloud>,
//             Without<UpCloud>,
//             Without<DownCloud>,
//         ),
//     >,
//     mut up_query: Query<
//         (&mut GridPos, &mut Transform),
//         (
//             With<UpCloud>,
//             Without<RightCloud>,
//             Without<LeftCloud>,
//             Without<DownCloud>,
//         ),
//     >,
//     mut down_query: Query<
//         (&mut GridPos, &mut Transform),
//         (
//             With<DownCloud>,
//             Without<RightCloud>,
//             Without<UpCloud>,
//             Without<LeftCloud>,
//         ),
//     >,
// ) {
//     // return early if the timer is off or there is no cloud direction set
//     if cloud_control.cur_cloud_move.is_none() {
//         return;
//     }
//     let cloud_dir = cloud_control.cur_cloud_move.unwrap();

//     if cloud_dir == CloudDir::Down {
//         for (cloud_pos, mut transfo) in down_query.iter_mut() {
//             transfo.translation = grid_to_vec(cloud_pos.pos);
//         }
//     }
//     if cloud_dir == CloudDir::Left {
//         for (cloud_pos, mut transfo) in left_query.iter_mut() {
//             transfo.translation = grid_to_vec(cloud_pos.pos);
//         }
//     }
//     if cloud_dir == CloudDir::Up {
//         for (cloud_pos, mut transfo) in up_query.iter_mut() {
//             transfo.translation = grid_to_vec(cloud_pos.pos);
//         }
//     }
//     if cloud_dir == CloudDir::Right {
//         for (cloud_pos, mut transfo) in right_query.iter_mut() {
//             transfo.translation = grid_to_vec(cloud_pos.pos);
//         }
//     }
//     cloud_control.cur_cloud_move = None;
// }
