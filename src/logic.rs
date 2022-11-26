#![cfg_attr(debug_assertions, allow(dead_code, unused_imports))]

use crate::actions::{Actions, GameControl};
use crate::clouds;
use crate::clouds::{
    Cloud, CloudDir, CooldownTimer, DownCloud, GridPos, IsCooldown, LeftCloud, RightCloud, UpCloud,
    CLOUD_LAYER,
};

use crate::loading::TextureAssets;
use crate::player::{
    fill_player_buffer, pop_player_buffer, Player, PlayerControl, INIT_POS, TILE_SIZE,
};
use crate::ui::MessBar;
use crate::world::{LEVEL_SIZE, STAGE_BL, STAGE_UR, STAGE_WIDTH};
use crate::GameState;
use bevy::prelude::*;
// use bevy::render::texture::ImageSettings;
use colored::*;
use iyes_loopless::prelude::*;
use rand::seq::SliceRandom;

pub const MAX_BUFFER_INPUT: usize = 2;
const MAIN_PERIOD: f32 = 0.150;
// Multiple of the move timer:
const SPAWN_FREQUENCY: u8 = 3;
// Offset for delaying cloud spawning depending on the direction:
const SPAWN_OFFSET: [u8; 4] = [0, 1, 0, 1];
// We sync the actions of the player with the music
const TIMER_SCALE_FACTOR: u8 = 4;
const SEQUENCE: [CloudDir; 4] = [
    CloudDir::Left,
    CloudDir::Up,
    CloudDir::Right,
    CloudDir::Down,
];
pub const PUSH_COOLDOWN: f32 = 0.4;
pub const CLOUD_COUNT_LOSE_COND: usize = 16;
// How late after the beat the player can be and still move:
pub const FORGIVENESS_MARGIN: f32 = 0.050;

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
                    .with_system(tick_timers)
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
                    .with_system(check_lose_condition)
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
                    .with_system(count_clouds)
                    .into(),
            );
    }
}

#[derive(Default, Eq, PartialEq, Debug, Copy, Clone)]
pub enum PushState {
    #[default]
    Empty,
    Blocked,
    CanPush,
    CanPushPlayer,
    Despawn,
    PushOver,
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

#[derive(Component, Deref, DerefMut)]
struct AnimationTimer(Timer);

#[derive(Default, Resource)]
pub struct MainClock {
    pub main_timer: Timer,
    player_to_cloud_ratio: f32,
    pub move_player: bool,
    pub move_clouds: bool,
    forgiveness_margin: f32,
    cloud_counter: u8,
}

fn tick_timers(
    mut main_clock: ResMut<MainClock>,
    time: Res<Time>,
    mut query: Query<(&mut CooldownTimer, &mut IsCooldown), With<Cloud>>,
) {
    /* ▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓ Global timers to sync with the music ▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓ */
    // timers gotta be ticked, to work
    main_clock.main_timer.tick(time.delta());

    if main_clock.main_timer.just_finished() {
        main_clock.cloud_counter += 1;
        main_clock.move_player = true;
        if main_clock.cloud_counter >= TIMER_SCALE_FACTOR {
            main_clock.move_clouds = true;
            main_clock.cloud_counter = 0;
        }
    } else {
        main_clock.move_clouds = false;
        if main_clock.main_timer.elapsed_secs() < main_clock.forgiveness_margin {
            main_clock.move_player = true;
        } else {
            main_clock.move_player = false;
        }
    }
    /* ▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓ Cooldown Timers ▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓ */
    for (mut timer, mut status) in query.iter_mut() {
        timer.tick(time.delta());
        if timer.finished() {
            status.val = false;
        }
    }
}

#[derive(Default, Resource)]
pub struct CloudControl {
    pub cur_new_cloud: Option<CloudDir>,
    pub cur_cloud_move: Option<CloudDir>,
    cur_cloud: CloudDir,
    sequence: [CloudDir; 4],
    spawn_counter: [u8; 4],
    pub pushed_clouds: Vec<([i8; 2], CloudDir)>,
    pub next_pushed_clouds: Vec<([i8; 2], CloudDir, PushState)>,
}

#[derive(Resource)]
pub struct GridState {
    pub grid: [[TileOccupation; LEVEL_SIZE as usize]; LEVEL_SIZE as usize],
    pub cloud_count: u8,
    // pushed_clouds: Vec<([i8; 2], CloudDir)>,
    // next_pushed_clouds: Vec<([i8; 2], CloudDir)>,
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

impl Default for GridState {
    fn default() -> Self {
        let mut tmp_grid = [[TileOccupation::Empty; LEVEL_SIZE as usize]; LEVEL_SIZE as usize];
        tmp_grid[INIT_POS[0] as usize][INIT_POS[1] as usize] = TileOccupation::Player;
        GridState {
            grid: tmp_grid,
            cloud_count: 0,
        }
    }
}

impl GridState {
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

    /// Check whether the next tile is occupied. Here the function is called on
    /// the tile N+1 such that we check the tile N+2
    pub fn is_occupied(&self, tile: [i8; 2], dir: CloudDir, object: TileOccupation) -> PushState {
        if self.is_out_of_range(tile) {
            return PushState::Despawn;
        }

        /* ▓▓▓▓▓ Case where the player is close to the edge of the stage ▓▓▓▓ */
        // To do before checking whether the cell is empty since we detect sky
        // tiles as empty
        if object == TileOccupation::Player && self.is_sky(tile) {
            return PushState::Blocked;
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
            // case where the tile behind is empty, it depends on the target
            // tile
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

    pub fn is_out_of_range(&self, tile: [i8; 2]) -> bool {
        0 > tile[0] || tile[0] >= LEVEL_SIZE as i8 || 0 > tile[1] || tile[1] >= LEVEL_SIZE as i8
    }

    pub fn is_sky(&self, tile: [i8; 2]) -> bool {
        tile[0] < (STAGE_BL[0] as i8)
            || tile[1] < (STAGE_BL[1] as i8)
            || tile[0] > (STAGE_UR[0] as i8)
            || tile[1] > (STAGE_UR[1] as i8)
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

    pub fn new_cloud(&mut self, border: CloudDir) -> Option<(Vec3, [i8; 2])> {
        let line = match border {
            CloudDir::Down => self.up_row(),
            CloudDir::Left => self.right_col(),
            CloudDir::Right => self.left_col(),
            CloudDir::Up => self.down_row(),
        };

        let non_occupied: Vec<[i8; 2]> = line
            .into_iter()
            .filter(|v| self.is_occupied(*v, border, dir_to_tile(border)) == PushState::Empty)
            .collect();
        if let Some(pos) = non_occupied.choose(&mut rand::thread_rng()) {
            // Add the cloud to the grid
            self.populate_tile_with_cloud(*pos, dir_to_tile(border));
            Some((grid_to_vec(*pos), *pos))
        } else {
            None
        }
    }

    /// Spawn something on the tile, it becomes occupied
    fn populate_tile_with_cloud(&mut self, target_tile: [i8; 2], object: TileOccupation) {
        self.grid[target_tile[0] as usize][target_tile[1] as usize] = object;
    }

    pub fn reset_grid(&mut self) {
        *self = GridState::default();
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
}

/// Check whether the player cannot move at all, then it loses.
fn check_lose_condition(
    mut commands: Commands,
    grid_state: ResMut<GridState>,
    player_control: ResMut<PlayerControl>,
) {
    let next_tiles = [
        [
            player_control.player_pos[0] - 1,
            player_control.player_pos[1],
        ],
        [
            player_control.player_pos[0],
            player_control.player_pos[1] + 1,
        ],
        [
            player_control.player_pos[0] + 1,
            player_control.player_pos[1],
        ],
        [
            player_control.player_pos[0],
            player_control.player_pos[1] - 1,
        ],
    ];
    let mut is_blocked = [false, false, false, false];

    for (i, tile) in next_tiles.into_iter().enumerate() {
        is_blocked[i] = matches!(
            grid_state.is_occupied(tile, SEQUENCE[i], TileOccupation::Player),
            PushState::Blocked
        );
        let has_lost = is_blocked.into_iter().all(|x| x);
        if has_lost {
            commands.insert_resource(NextState(GameState::GameOver))
        }
    }
}

fn count_clouds(grid_state: Res<GridState>, mut query: Query<&mut MessBar>) {
    let cloud_types = [
        TileOccupation::DownCloud,
        TileOccupation::UpCloud,
        TileOccupation::RightCloud,
        TileOccupation::DownCloud,
    ];
    let mut tmp_counter: usize = 0;
    for i in 0..grid_state.grid.len() {
        for j in 0..grid_state.grid[i].len() {
            let is_stage = !grid_state.is_sky([i as i8, j as i8]);
            let is_cloud = cloud_types.contains(&grid_state.grid[i][j]);
            if is_stage && is_cloud {
                tmp_counter += 1;
            }
        }

        // OPTI: do not duplicate the data:
        for mut mess_bar in query.iter_mut() {
            mess_bar.counter = tmp_counter;
        }
    }
}

fn despawn_clouds(
    mut commands: Commands,
    mut grid_state: ResMut<GridState>,
    mut query: Query<(&mut GridPos, Entity), (With<Cloud>,)>,
) {
    for (cloud_pos, entity) in query.iter_mut() {
        if grid_state.grid[cloud_pos.pos[0] as usize][cloud_pos.pos[1] as usize]
            == TileOccupation::Despawn
        {
            grid_state.grid[cloud_pos.pos[0] as usize][cloud_pos.pos[1] as usize] =
                TileOccupation::Empty;
            commands.entity(entity).despawn();
        }
    }
}

fn dir_index(cloud_dir: CloudDir) -> usize {
    SEQUENCE.iter().position(|&x| x == cloud_dir).unwrap()
}

/// Transform a cloud direcion into a TileOccupatio enum
fn dir_to_tile(dir: CloudDir) -> TileOccupation {
    match dir {
        CloudDir::Down => TileOccupation::DownCloud,
        CloudDir::Up => TileOccupation::UpCloud,
        CloudDir::Left => TileOccupation::LeftCloud,
        CloudDir::Right => TileOccupation::RightCloud,
    }
}

///Method to compute the cloud positions.
///
/// There are a couple of value inside the formula:
/// - the position of the cloud on the grid, starting at (0, 0) at the bottom
///   left
/// - the offset to express the coordinates in relative to the center
/// - a 0.5 offset to have the arrows centered on the tiles
/// - a -0.5 offset in the X direction because of the positioning of the load
///   bar
fn grid_to_vec(grid_pos: [i8; 2]) -> Vec3 {
    Vec3::new(
        (grid_pos[0]) as f32 * TILE_SIZE - (LEVEL_SIZE as f32) / 2. * TILE_SIZE + 0.5 * TILE_SIZE
            - 0.5 * TILE_SIZE,
        (grid_pos[1]) as f32 * TILE_SIZE - (LEVEL_SIZE as f32) / 2. * TILE_SIZE + 0.5 * TILE_SIZE,
        CLOUD_LAYER,
    )
}

#[allow(clippy::type_complexity)]
fn move_clouds(
    mut cloud_control: ResMut<CloudControl>,
    mut grid_state: ResMut<GridState>,
    mut left_query: Query<
        (&mut GridPos, &IsCooldown),
        (
            With<LeftCloud>,
            Without<RightCloud>,
            Without<UpCloud>,
            Without<DownCloud>,
        ),
    >,
    mut right_query: Query<
        (&mut GridPos, &IsCooldown),
        (
            With<RightCloud>,
            Without<LeftCloud>,
            Without<UpCloud>,
            Without<DownCloud>,
        ),
    >,
    mut up_query: Query<
        (&mut GridPos, &IsCooldown),
        (
            With<UpCloud>,
            Without<RightCloud>,
            Without<LeftCloud>,
            Without<DownCloud>,
        ),
    >,
    mut down_query: Query<
        (&mut GridPos, &IsCooldown),
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

    match cloud_dir {
        CloudDir::Down => {
            for (mut cloud_pos, is_cooling) in down_query.iter_mut() {
                if is_cooling.val {
                    continue;
                }
                let next_tile_push = grid_state.is_occupied(
                    [cloud_pos.pos[0], cloud_pos.pos[1] - 1i8],
                    cloud_dir,
                    dir_to_tile(cloud_dir),
                );

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
                    PushState::CanPushPlayer => {
                        cloud_control.pushed_clouds.push((cloud_pos.pos, cloud_dir));
                        cloud_control.next_pushed_clouds.push((
                            [cloud_pos.pos[0], cloud_pos.pos[1] - 1],
                            cloud_dir,
                            PushState::CanPushPlayer,
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
        }
        CloudDir::Left => {
            for (mut cloud_pos, is_cooling) in left_query.iter_mut() {
                if is_cooling.val {
                    continue;
                }
                let next_tile_push = grid_state.is_occupied(
                    [cloud_pos.pos[0] - 1i8, cloud_pos.pos[1]],
                    cloud_dir,
                    dir_to_tile(cloud_dir),
                );
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
                    PushState::CanPushPlayer => {
                        cloud_control.pushed_clouds.push((cloud_pos.pos, cloud_dir));
                        cloud_control.next_pushed_clouds.push((
                            [cloud_pos.pos[0] - 1, cloud_pos.pos[1]],
                            cloud_dir,
                            PushState::CanPushPlayer,
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
        }
        CloudDir::Up => {
            for (mut cloud_pos, is_cooling) in up_query.iter_mut() {
                if is_cooling.val {
                    continue;
                }
                let next_tile_push = grid_state.is_occupied(
                    [cloud_pos.pos[0], cloud_pos.pos[1] + 1i8],
                    cloud_dir,
                    dir_to_tile(cloud_dir),
                );
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
                    PushState::CanPushPlayer => {
                        cloud_control.pushed_clouds.push((cloud_pos.pos, cloud_dir));
                        cloud_control.next_pushed_clouds.push((
                            [cloud_pos.pos[0], cloud_pos.pos[1] + 1],
                            cloud_dir,
                            PushState::CanPushPlayer,
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
        }
        CloudDir::Right => {
            for (mut cloud_pos, is_cooling) in right_query.iter_mut() {
                if is_cooling.val {
                    continue;
                }
                let next_tile_push = grid_state.is_occupied(
                    [cloud_pos.pos[0] + 1i8, cloud_pos.pos[1]],
                    cloud_dir,
                    dir_to_tile(cloud_dir),
                );
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
                    PushState::CanPushPlayer => {
                        cloud_control.pushed_clouds.push((cloud_pos.pos, cloud_dir));
                        cloud_control.next_pushed_clouds.push((
                            [cloud_pos.pos[0] + 1, cloud_pos.pos[1]],
                            cloud_dir,
                            PushState::CanPushPlayer,
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
    }
    cloud_control.cur_cloud_move = None;
}

/// Deal with the cloud which need to be pushed. At this stage, one already
/// knows that the tile N+2 is empty to push the cloud
#[allow(clippy::type_complexity)]
fn push_clouds(
    mut commands: Commands,
    mut cloud_control: ResMut<CloudControl>,
    mut player_control: ResMut<PlayerControl>,
    mut grid_state: ResMut<GridState>,
    mut query: Query<
        (&Cloud, &mut GridPos, Entity, &mut IsCooldown),
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
    /* ▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓ Move first the next cloud "pushed": ▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓ */
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

        for (cloud, mut cloud_pos, entity, mut is_cooling) in query.iter_mut() {
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
                        if push_type == PushState::CanPushPlayer {
                            is_cooling.val = true;
                        }
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
                        if push_type == PushState::CanPushPlayer {
                            is_cooling.val = true;
                        }
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
                        if push_type == PushState::CanPushPlayer {
                            is_cooling.val = true;
                        }
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
                        if push_type == PushState::CanPushPlayer {
                            is_cooling.val = true;
                        }
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

    /* ▓▓▓▓▓▓▓▓▓ Then move the actual clouds pushing the other one: ▓▓▓▓▓▓▓▓▓ */
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

        for (_, mut cloud_pos, _, _) in query.iter_mut() {
            if cloud_pos.pos == pos {
                match dir {
                    CloudDir::Down => {
                        grid_state.move_on_grid(
                            cloud_pos.pos,
                            [cloud_pos.pos[0], cloud_pos.pos[1] - 1i8],
                            dir_to_tile(dir),
                        );
                        cloud_pos.pos[1] += -1i8;
                    }
                    CloudDir::Left => {
                        grid_state.move_on_grid(
                            cloud_pos.pos,
                            [cloud_pos.pos[0] - 1i8, cloud_pos.pos[1]],
                            dir_to_tile(dir),
                        );
                        cloud_pos.pos[0] += -1i8;
                    }
                    CloudDir::Right => {
                        grid_state.move_on_grid(
                            cloud_pos.pos,
                            [cloud_pos.pos[0] + 1i8, cloud_pos.pos[1]],
                            dir_to_tile(dir),
                        );
                        cloud_pos.pos[0] += 1i8;
                    }
                    CloudDir::Up => {
                        grid_state.move_on_grid(
                            cloud_pos.pos,
                            [cloud_pos.pos[0], cloud_pos.pos[1] + 1i8],
                            dir_to_tile(dir),
                        );
                        cloud_pos.pos[1] += 1i8;
                    }
                }
            }
        }
    }
}

fn set_cloud_direction(mut cloud_control: ResMut<CloudControl>, main_clock: Res<MainClock>) {
    if main_clock.move_clouds {
        let cloud_dir = Some(cloud_control.next_cloud_direction());
        debug!("cloud dir.: {:?}", cloud_dir.unwrap());
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

fn set_up_logic(mut commands: Commands) {
    // Create our game rules resource
    commands.insert_resource(PlayerControl {
        player_pos: INIT_POS,
        input_buffer: [GameControl::Idle; MAX_BUFFER_INPUT],
        timer: Timer::from_seconds(MAIN_PERIOD, TimerMode::Repeating),
    });
    commands.insert_resource(GridState::default());
    commands.insert_resource(MainClock {
        main_timer: Timer::from_seconds(MAIN_PERIOD, TimerMode::Repeating),
        player_to_cloud_ratio: TIMER_SCALE_FACTOR as f32,
        forgiveness_margin: FORGIVENESS_MARGIN,
        ..Default::default()
    });
    commands.insert_resource(CloudControl {
        cur_new_cloud: None,
        cur_cloud_move: None,
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

fn update_cloud_pos(mut query: Query<(&mut GridPos, &mut Transform), (With<Cloud>,)>) {
    for (cloud_pos, mut transfo) in query.iter_mut() {
        transfo.translation = grid_to_vec(cloud_pos.pos);
    }
}
