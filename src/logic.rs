#![cfg_attr(debug_assertions, allow(dead_code, unused_imports))]

use std::time::Duration;

use crate::actions::{Actions, GameControl};
use crate::audio::{SongHandle, SoundOnAction, SoundOnMove, SONG_1, SONG_2};
use crate::clouds::{self, Animation, AnimationState, ToDespawn};
use crate::clouds::{
    Cloud, CloudDir, CooldownTimer, DownCloud, GridPos, IsCooldown, LeftCloud, RightCloud, UpCloud,
    CLOUD_LAYER,
};
use crate::loading::{AudioAssets, TextureAssets};
use crate::player::{
    fill_player_buffer, pop_player_buffer, Player, PlayerControl, INIT_POS, TILE_SIZE,
};
use crate::ui::MessBar;
use crate::world::{LEVEL_SIZE, STAGE_BL, STAGE_UR, STAGE_WIDTH};
use crate::GameState;
use bevy::prelude::*;
use bevy_easings::*;
use bevy_kira_audio::prelude::*;
// use bevy::render::texture::ImageSettings;
use colored::*;

use rand::seq::SliceRandom;

pub const MAX_BUFFER_INPUT: usize = 2;
// pub const MAIN_PERIOD: f32 = 0.150;
// Multiple of the move timer:
pub const SPAWN_FREQUENCY: u8 = 3;
// Offset for delaying cloud spawning depending on the direction:
const SPAWN_OFFSET: [u8; 4] = [0, 1, 0, 1];
// We sync the actions of the player with the music
pub const TIMER_SCALE_FACTOR: u8 = 4;
const SEQUENCE: [CloudDir; 4] = [
    CloudDir::Left,
    CloudDir::Up,
    CloudDir::Right,
    CloudDir::Down,
];

// The push cooldown is a multiple of the main clock:
pub const PUSH_COOLDOWN_FACTOR: f32 = 4.;
pub const CLOUD_COUNT_LOSE_COND: usize = 16;
// How late after the beat the player can be and still move:
pub const FORGIVENESS_MARGIN: f32 = 0.05;
pub const SPECIAL_ACTIVATION_NB: u8 = 2;
pub const CLOUD_EASING: bevy_easings::EaseFunction = bevy_easings::EaseFunction::QuadraticIn;
pub const CLOUD_SCALE_EASING: bevy_easings::EaseFunction = bevy_easings::EaseFunction::QuadraticIn;
pub const CLOUD_SCALE_FACTOR_EASING: f32 = 2.;
// Duration of the easing for the clouds in ms:
pub const CLOUD_EASING_DURATION: std::time::Duration = std::time::Duration::from_millis(100);
pub const SPECIAL_TIMEOUT: u8 = 4;

pub struct LogicPlugin;

// System sets can be used to group systems and configured to control relative ordering
#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
enum LogicSystem {
    TickClock,
    FillPlayerBuffer,
    PopPlayerBuffer,
    MoveClouds,
    PushClouds,
    UpdateSprites,
    RemoveClouds,
    FinishEasings,
    CheckLoss,
}

/// This plugin handles player related stuff like movement
/// Player logic is only active during the State `GameState::Playing`
impl Plugin for LogicPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(GameState::Playing), set_up_logic)
            .add_systems(
                Update,
                (tick_timers, set_cloud_direction)
                    .run_if(in_state(GameState::Playing))
                    .in_set(LogicSystem::TickClock)
                    .before(reset_cooldown_timers),
            )
            .add_systems(
                Update,
                fill_player_buffer
                    .run_if(in_state(GameState::Playing))
                    .in_set(LogicSystem::FillPlayerBuffer),
            )
            .add_systems(
                Update,
                (pop_player_buffer.run_if(in_state(GameState::Playing)),)
                    .in_set(LogicSystem::PopPlayerBuffer),
            )
            .add_systems(
                Update,
                (
                    move_clouds.run_if(
                        in_state(GameState::Playing).or_else(in_state(GameState::GameOver)),
                    ),
                    (play_special, clouds::new_cloud).run_if(in_state(GameState::Playing)),
                )
                    .in_set(LogicSystem::MoveClouds)
                    .after(LogicSystem::TickClock),
            )
            .add_systems(
                Update,
                push_clouds
                    .run_if(in_state(GameState::Playing).or_else(in_state(GameState::GameOver)))
                    .in_set(LogicSystem::PushClouds)
                    .after(LogicSystem::MoveClouds),
            )
            .add_systems(
                Update,
                (
                    update_cloud_pos.run_if(
                        in_state(GameState::Playing).or_else(in_state(GameState::GameOver)),
                    ),
                    count_clouds.run_if(in_state(GameState::Playing)),
                )
                    .in_set(LogicSystem::UpdateSprites)
                    .after(LogicSystem::PushClouds),
            )
            .add_systems(
                Update,
                despawn_clouds
                    .run_if(in_state(GameState::Playing))
                    .in_set(LogicSystem::RemoveClouds)
                    .after(LogicSystem::UpdateSprites),
            )
            .add_systems(
                Update,
                finish_easings
                    .run_if(in_state(GameState::Playing).or_else(in_state(GameState::GameOver)))
                    .in_set(LogicSystem::FinishEasings)
                    .after(LogicSystem::RemoveClouds),
            )
            .add_systems(
                Update,
                (check_loss_condition.run_if(in_state(GameState::Playing)),)
                    .in_set(LogicSystem::CheckLoss)
                    .after(LogicSystem::FinishEasings),
            )
            .add_event::<SoundOnMove>()
            .add_event::<SoundOnAction>();
    }
}

#[derive(Default, Eq, PartialEq, Debug, Copy, Clone)]
pub enum PushState {
    #[default]
    Empty,
    Blocked,
    CanPush,
    PlayerCanPush,
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
    CooldownCloud,
}

#[derive(Component, Deref, DerefMut)]
struct AnimationTimer(Timer);

#[derive(Default, Resource)]
pub struct MainClock {
    pub main_timer: Timer,
    pub absolute_timer: Timer,
    pub last_absolute_timer: f32,
    pub last_audio_time: f32,
    // pub intro_finished: bool,
    pub excess_time: f32,
    player_to_cloud_ratio: f32,
    pub move_player: bool,
    pub move_clouds: bool,
    forgiveness_margin: f32,
    cloud_counter: u8,
}

#[derive(Default, Eq, PartialEq, Debug, Copy, Clone)]
pub enum LossCondition {
    #[default]
    NoLoss,
    TooMessy,
    Stuck,
}

#[derive(Component)]
pub struct LossCause;

// #[derive(Default, Resource)]
// pub struct LoseInformation {
//     pub condition: LoseCondition,
// }

fn tick_timers(
    mut main_clock: ResMut<MainClock>,
    time: Res<Time>,
    mut audio_instances: ResMut<Assets<AudioInstance>>,
    handle: Res<SongHandle>,
    audio_assets: Res<AudioAssets>,
) {
    /* ▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓ Constants ▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓ */
    let beat_length = match audio_assets.selected_song {
        crate::audio::SelectedSong::Song1 => SONG_1.beat_length,
        crate::audio::SelectedSong::Song2 => SONG_2.beat_length,
    };
    // let song_length = match audio_assets.selected_song {
    //     crate::audio::SelectedSong::Song1 => SONG_1.length,
    //     crate::audio::SelectedSong::Song2 => SONG_2.length,
    // };
    let intro_length = match audio_assets.selected_song {
        crate::audio::SelectedSong::Song1 => SONG_1.intro_length,
        crate::audio::SelectedSong::Song2 => SONG_2.intro_length,
    };
    /* ▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓ Retrieve the audio timing ▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓ */
    let play_pos = if audio_instances.get_mut(&handle.song).is_some() {
        audio_instances.state(&handle.song).position()
    } else {
        None
    };

    /* ▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓ Tick the global clock ▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓ */
    let tick_time: f32;
    match main_clock.excess_time {
        x if x.abs() <= time.delta_seconds() => {
            tick_time = time.delta_seconds() + x;
            main_clock.excess_time = 0.;
        }
        x if x > 0. && x.abs() > time.delta_seconds() => {
            tick_time = time.delta_seconds() + x;
            main_clock.excess_time = 0.;
        }
        x if x < 0. && x.abs() > time.delta_seconds() => {
            // This is negative, we clip it a 0:
            tick_time = 0.;
            main_clock.excess_time = time.delta_seconds() + x;
        }
        _ => panic!(),
    }

    main_clock
        .main_timer
        .tick(Duration::from_secs_f32(tick_time));
    main_clock
        .absolute_timer
        .tick(Duration::from_secs_f32(tick_time));
    let current_abs_time = main_clock.absolute_timer.elapsed_secs();

    // The audio loops after the intro, artificially add a jump in the absolute
    // counter to keep the sync with the audio stream position:
    if main_clock.absolute_timer.just_finished() {
        main_clock
            .absolute_timer
            .tick(Duration::from_secs_f32(intro_length));
    }

    if main_clock.main_timer.just_finished() {
        /* ▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓ Resync audio ▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓ */
        // The correction is positive if the game logic is late, negative
        // if in advance:
        // let mut audio_sync: f64 = 0.;
        if let Some(play_pos) = play_pos {
            let audio_sync = play_pos - (current_abs_time as f64);
            // Prevent problems when looping the song:
            main_clock.excess_time = (audio_sync.signum() as f32)
                * (audio_sync.abs() as f32).rem_euclid(beat_length / (TIMER_SCALE_FACTOR as f32));
            main_clock.last_absolute_timer = main_clock.absolute_timer.elapsed_secs();
            main_clock.last_audio_time = play_pos as f32;
        }

        /* ▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓ Execute logic ▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓ */
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
}

// mut query: Query<(&mut CooldownTimer, &mut IsCooldown), With<Cloud>>,
#[allow(clippy::type_complexity)]
fn reset_cooldown_timers(
    asset_server: Res<AssetServer>,
    mut grid_state: ResMut<GridState>,
    time: Res<Time>,
    mut left_query: Query<
        (
            &mut CooldownTimer,
            &mut GridPos,
            &mut IsCooldown,
            &mut Handle<Image>,
        ),
        (
            With<LeftCloud>,
            Without<RightCloud>,
            Without<UpCloud>,
            Without<DownCloud>,
        ),
    >,
    mut right_query: Query<
        (
            &mut CooldownTimer,
            &mut GridPos,
            &mut IsCooldown,
            &mut Handle<Image>,
        ),
        (
            With<RightCloud>,
            Without<LeftCloud>,
            Without<UpCloud>,
            Without<DownCloud>,
        ),
    >,
    mut up_query: Query<
        (
            &mut CooldownTimer,
            &mut GridPos,
            &mut IsCooldown,
            &mut Handle<Image>,
        ),
        (
            With<UpCloud>,
            Without<RightCloud>,
            Without<LeftCloud>,
            Without<DownCloud>,
        ),
    >,
    mut down_query: Query<
        (
            &mut CooldownTimer,
            &mut GridPos,
            &mut IsCooldown,
            &mut Handle<Image>,
        ),
        (
            With<DownCloud>,
            Without<RightCloud>,
            Without<UpCloud>,
            Without<LeftCloud>,
        ),
    >,
) {
    for (mut timer, grid_pos, mut status, mut texture) in left_query.iter_mut() {
        timer.tick(time.delta());
        if timer.finished() {
            let pos = grid_pos.pos;
            grid_state.grid[pos[0] as usize][pos[1] as usize] = TileOccupation::LeftCloud;
            *texture = asset_server.load("textures/left_cloud.png");
            status.val = false;
            timer.reset();
        }
    }
    for (mut timer, grid_pos, mut status, mut texture) in right_query.iter_mut() {
        timer.tick(time.delta());
        if timer.finished() {
            // grid_pos.status.val = false;
            let pos = grid_pos.pos;
            grid_state.grid[pos[0] as usize][pos[1] as usize] = TileOccupation::RightCloud;
            *texture = asset_server.load("textures/right_cloud.png");
            status.val = false;
            timer.reset();
        }
    }
    for (mut timer, grid_pos, mut status, mut texture) in up_query.iter_mut() {
        timer.tick(time.delta());
        if timer.finished() {
            // grid_pos.status.val = false;
            let pos = grid_pos.pos;
            grid_state.grid[pos[0] as usize][pos[1] as usize] = TileOccupation::UpCloud;
            status.val = false;
            *texture = asset_server.load("textures/up_cloud.png");
            timer.reset();
        }
    }
    for (mut timer, grid_pos, mut status, mut texture) in down_query.iter_mut() {
        timer.tick(time.delta());
        if timer.finished() {
            // grid_pos.status.val = false;
            let pos = grid_pos.pos;
            grid_state.grid[pos[0] as usize][pos[1] as usize] = TileOccupation::DownCloud;
            status.val = false;
            timer.reset();
            *texture = asset_server.load("textures/down_cloud.png");
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
    pub loss_condition: LossCondition,
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
            loss_condition: LossCondition::NoLoss,
        }
    }
}

impl GridState {
    fn down_row(&self) -> [[i8; 2]; STAGE_WIDTH as usize] {
        let mut res = [[0i8, 0i8]; STAGE_WIDTH as usize];
        for (ndx, i) in ((LEVEL_SIZE - STAGE_WIDTH) / 2
            ..=LEVEL_SIZE - (LEVEL_SIZE - STAGE_WIDTH) / 2 - 1)
            .enumerate()
        {
            res[ndx][0] = i as i8;
            res[ndx][1] = 0i8;
        }
        res
    }

    /// Check whether the next tile is occupied. Here the function is called on
    /// the tile N+1 such that we check the tile N+2. Therefore we need the
    /// tile being pushed and the direction in which the push is done:
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
        let tile_np2_occupied = !matches!(
            next_tile_occ,
            TileOccupation::Empty | TileOccupation::Despawn
        );

        // Case where there is something behind, just forget it
        if tile_np2_occupied {
            PushState::Blocked
        } else {
            // Case where the cloud is cooling down:
            if matches!(target_tile_occ, TileOccupation::CooldownCloud) {
                return PushState::Blocked;
            }

            // case where the tile behind is empty, it depends on the target
            // tile
            match dir {
                CloudDir::Down => match target_tile_occ {
                    TileOccupation::UpCloud => PushState::Blocked,
                    TileOccupation::Player => {
                        if tile[1] <= (((LEVEL_SIZE - STAGE_WIDTH) / 2) as i8) {
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
                        if tile[1] >= ((STAGE_WIDTH + (LEVEL_SIZE - STAGE_WIDTH) / 2 - 1) as i8) {
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
                        if tile[0] <= (((LEVEL_SIZE - STAGE_WIDTH) / 2) as i8) {
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
                        if tile[0] >= ((STAGE_WIDTH + (LEVEL_SIZE - STAGE_WIDTH) / 2 - 1) as i8) {
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
        0 > tile[0] || tile[0] >= (LEVEL_SIZE as i8) || 0 > tile[1] || tile[1] >= (LEVEL_SIZE as i8)
    }

    pub fn is_sky(&self, tile: [i8; 2]) -> bool {
        tile[0] < (STAGE_BL[0] as i8)
            || tile[1] < (STAGE_BL[1] as i8)
            || tile[0] > (STAGE_UR[0] as i8)
            || tile[1] > (STAGE_UR[1] as i8)
    }
    fn left_col(&self) -> [[i8; 2]; STAGE_WIDTH as usize] {
        let mut res = [[0i8, 0i8]; STAGE_WIDTH as usize];
        for (ndx, i) in ((LEVEL_SIZE - STAGE_WIDTH) / 2
            ..=LEVEL_SIZE - (LEVEL_SIZE - STAGE_WIDTH) / 2 - 1)
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
        for (ndx, i) in ((LEVEL_SIZE - STAGE_WIDTH) / 2
            ..=LEVEL_SIZE - (LEVEL_SIZE - STAGE_WIDTH) / 2 - 1)
            .enumerate()
        {
            res[ndx][0] = (LEVEL_SIZE - 1) as i8;
            res[ndx][1] = i as i8;
        }
        res
    }

    fn up_row(&self) -> [[i8; 2]; STAGE_WIDTH as usize] {
        let mut res = [[0i8, 0i8]; STAGE_WIDTH as usize];
        for (ndx, i) in ((LEVEL_SIZE - STAGE_WIDTH) / 2
            ..=LEVEL_SIZE - (LEVEL_SIZE - STAGE_WIDTH) / 2 - 1)
            .enumerate()
        {
            res[ndx][0] = i as i8;
            res[ndx][1] = (LEVEL_SIZE - 1) as i8;
        }
        res
    }
}

/// Check whether the player cannot move at all, then it loses.
fn check_loss_condition(
    mut commands: Commands,
    mut grid_state: ResMut<GridState>,
    player_control: ResMut<PlayerControl>,
    mut next_state: ResMut<NextState<GameState>>,
    mut query: Query<(Entity, &mut GridPos), With<Cloud>>,
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

    let mut has_lost = false;
    for (i, tile) in next_tiles.into_iter().enumerate() {
        is_blocked[i] = matches!(
            grid_state.is_occupied(tile, SEQUENCE[i], TileOccupation::Player),
            PushState::Blocked
        );
        has_lost = is_blocked.into_iter().all(|x| x);
    }
    if has_lost {
        for (entity, pos) in query.iter_mut() {
            if next_tiles.contains(&pos.pos) {
                commands.entity(entity).insert(LossCause);
            }
        }

        grid_state.loss_condition = LossCondition::Stuck;
        next_state.set(GameState::GameOver);
        // WIP
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
    query_2: Query<Entity, (With<ToDespawn>,)>,
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
    for entity in query_2.iter() {
        commands.entity(entity).despawn();
    }
}

fn dir_index(cloud_dir: CloudDir) -> usize {
    SEQUENCE.iter().position(|&x| x == cloud_dir).unwrap()
}

/// Transform a cloud direction into a TileOccupation enum
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
        (grid_pos[0] as f32) * TILE_SIZE - ((LEVEL_SIZE as f32) / 2.) * TILE_SIZE + 0.5 * TILE_SIZE
            - 0.5 * TILE_SIZE,
        (grid_pos[1] as f32) * TILE_SIZE - ((LEVEL_SIZE as f32) / 2.) * TILE_SIZE + 0.5 * TILE_SIZE,
        CLOUD_LAYER,
    )
}

#[allow(clippy::type_complexity)]
fn move_clouds(
    mut cloud_control: ResMut<CloudControl>,
    mut grid_state: ResMut<GridState>,
    asset_server: Res<AssetServer>,
    mut left_query: Query<
        (&mut GridPos, &mut IsCooldown, &mut Handle<Image>),
        (
            With<LeftCloud>,
            Without<RightCloud>,
            Without<UpCloud>,
            Without<DownCloud>,
        ),
    >,
    mut right_query: Query<
        (&mut GridPos, &mut IsCooldown, &mut Handle<Image>),
        (
            With<RightCloud>,
            Without<LeftCloud>,
            Without<UpCloud>,
            Without<DownCloud>,
        ),
    >,
    mut up_query: Query<
        (&mut GridPos, &mut IsCooldown, &mut Handle<Image>),
        (
            With<UpCloud>,
            Without<RightCloud>,
            Without<LeftCloud>,
            Without<DownCloud>,
        ),
    >,
    mut down_query: Query<
        (&mut GridPos, &mut IsCooldown, &mut Handle<Image>),
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
            for (mut cloud_pos, mut is_cooling, mut texture) in down_query.iter_mut() {
                if is_cooling.val {
                    *texture = asset_server.load("textures/down_cloud.png");
                    is_cooling.val = false;
                    let pos = cloud_pos.pos;
                    grid_state.grid[pos[0] as usize][pos[1] as usize] = TileOccupation::DownCloud;
                }
                let next_tile_push = grid_state.is_occupied(
                    [cloud_pos.pos[0], cloud_pos.pos[1] - 1i8],
                    cloud_dir,
                    dir_to_tile(cloud_dir),
                );

                match next_tile_push {
                    PushState::Blocked => {
                        continue;
                    }
                    PushState::Despawn => {
                        grid_state.grid[cloud_pos.pos[0] as usize][cloud_pos.pos[1] as usize] =
                            TileOccupation::Despawn;
                    }
                    PushState::Empty => {
                        grid_state.move_on_grid(
                            cloud_pos.pos,
                            [cloud_pos.pos[0], cloud_pos.pos[1] - 1i8],
                            TileOccupation::DownCloud,
                        );
                        cloud_pos.old_pos = cloud_pos.pos;
                        cloud_pos.is_pushed = false;
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
                    PushState::PlayerCanPush => {
                        cloud_control.pushed_clouds.push((cloud_pos.pos, cloud_dir));
                        cloud_control.next_pushed_clouds.push((
                            [cloud_pos.pos[0], cloud_pos.pos[1] - 1],
                            cloud_dir,
                            PushState::PlayerCanPush,
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
            for (mut cloud_pos, mut is_cooling, mut texture) in left_query.iter_mut() {
                if is_cooling.val {
                    *texture = asset_server.load("textures/left_cloud.png");
                    is_cooling.val = false;
                    let pos = cloud_pos.pos;
                    grid_state.grid[pos[0] as usize][pos[1] as usize] = TileOccupation::LeftCloud;
                }
                let next_tile_push = grid_state.is_occupied(
                    [cloud_pos.pos[0] - 1i8, cloud_pos.pos[1]],
                    cloud_dir,
                    dir_to_tile(cloud_dir),
                );
                match next_tile_push {
                    PushState::Blocked => {
                        continue;
                    }
                    PushState::Despawn => {
                        grid_state.grid[cloud_pos.pos[0] as usize][cloud_pos.pos[1] as usize] =
                            TileOccupation::Despawn;
                    }
                    PushState::Empty => {
                        grid_state.move_on_grid(
                            cloud_pos.pos,
                            [cloud_pos.pos[0] - 1i8, cloud_pos.pos[1]],
                            TileOccupation::LeftCloud,
                        );
                        cloud_pos.old_pos = cloud_pos.pos;
                        cloud_pos.is_pushed = false;
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
                    PushState::PlayerCanPush => {
                        cloud_control.pushed_clouds.push((cloud_pos.pos, cloud_dir));
                        cloud_control.next_pushed_clouds.push((
                            [cloud_pos.pos[0] - 1, cloud_pos.pos[1]],
                            cloud_dir,
                            PushState::PlayerCanPush,
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
            for (mut cloud_pos, mut is_cooling, mut texture) in up_query.iter_mut() {
                if is_cooling.val {
                    *texture = asset_server.load("textures/up_cloud.png");
                    is_cooling.val = false;
                    let pos = cloud_pos.pos;
                    grid_state.grid[pos[0] as usize][pos[1] as usize] = TileOccupation::UpCloud;
                }
                let next_tile_push = grid_state.is_occupied(
                    [cloud_pos.pos[0], cloud_pos.pos[1] + 1i8],
                    cloud_dir,
                    dir_to_tile(cloud_dir),
                );
                match next_tile_push {
                    PushState::Blocked => {
                        continue;
                    }
                    PushState::Despawn => {
                        grid_state.grid[cloud_pos.pos[0] as usize][cloud_pos.pos[1] as usize] =
                            TileOccupation::Despawn;
                    }
                    PushState::Empty => {
                        grid_state.move_on_grid(
                            cloud_pos.pos,
                            [cloud_pos.pos[0], cloud_pos.pos[1] + 1i8],
                            TileOccupation::UpCloud,
                        );
                        cloud_pos.old_pos = cloud_pos.pos;
                        cloud_pos.is_pushed = false;
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
                    PushState::PlayerCanPush => {
                        cloud_control.pushed_clouds.push((cloud_pos.pos, cloud_dir));
                        cloud_control.next_pushed_clouds.push((
                            [cloud_pos.pos[0], cloud_pos.pos[1] + 1],
                            cloud_dir,
                            PushState::PlayerCanPush,
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
            for (mut cloud_pos, mut is_cooling, mut texture) in right_query.iter_mut() {
                if is_cooling.val {
                    *texture = asset_server.load("textures/right_cloud.png");
                    is_cooling.val = false;
                    let pos = cloud_pos.pos;
                    grid_state.grid[pos[0] as usize][pos[1] as usize] = TileOccupation::RightCloud;
                }
                let next_tile_push = grid_state.is_occupied(
                    [cloud_pos.pos[0] + 1i8, cloud_pos.pos[1]],
                    cloud_dir,
                    dir_to_tile(cloud_dir),
                );
                match next_tile_push {
                    PushState::Blocked => {
                        continue;
                    }
                    PushState::Despawn => {
                        grid_state.grid[cloud_pos.pos[0] as usize][cloud_pos.pos[1] as usize] =
                            TileOccupation::Despawn;
                    }
                    PushState::Empty => {
                        grid_state.move_on_grid(
                            cloud_pos.pos,
                            [cloud_pos.pos[0] + 1i8, cloud_pos.pos[1]],
                            TileOccupation::RightCloud,
                        );
                        cloud_pos.old_pos = cloud_pos.pos;
                        cloud_pos.is_pushed = false;
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
                    PushState::PlayerCanPush => {
                        cloud_control.pushed_clouds.push((cloud_pos.pos, cloud_dir));
                        cloud_control.next_pushed_clouds.push((
                            [cloud_pos.pos[0] + 1, cloud_pos.pos[1]],
                            cloud_dir,
                            PushState::PlayerCanPush,
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

// Apply the special action:
fn play_special(
    mut commands: Commands,
    mut player_control: ResMut<PlayerControl>,
    asset_server: Res<AssetServer>,
    mut play_push_sound_event: EventWriter<SoundOnAction>,
    mut grid_state: ResMut<GridState>,
    mut query: Query<(
        Entity,
        &mut Cloud,
        &mut GridPos,
        &IsCooldown,
        &mut Handle<Image>,
    )>,
) {
    if player_control.special_control < SPECIAL_ACTIVATION_NB {
        return;
    }
    play_push_sound_event.send(SoundOnAction {
        direction: GameControl::Special,
    });
    player_control.special_timeout = 0;

    let pl_pos = player_control.player_pos;
    let adj_clouds = [
        ([pl_pos[0] - 1, pl_pos[1]], TileOccupation::RightCloud),
        ([pl_pos[0] + 1, pl_pos[1]], TileOccupation::LeftCloud),
        ([pl_pos[0], pl_pos[1] - 1], TileOccupation::UpCloud),
        ([pl_pos[0], pl_pos[1] + 1], TileOccupation::DownCloud),
    ];

    for (entity, mut cloud, grid_pos, is_cooling, mut texture) in query.iter_mut() {
        let pos = grid_pos.pos;
        let adj_ndx = adj_clouds.iter().position(|&x| x.0 == pos);

        if let Some(ndx) = adj_ndx {
            // Change the cloud direction
            grid_state.grid[pos[0] as usize][pos[1] as usize] = adj_clouds[ndx].1;

            match cloud.dir {
                CloudDir::Up => commands.entity(entity).remove::<UpCloud>(),
                CloudDir::Down => commands.entity(entity).remove::<DownCloud>(),
                CloudDir::Left => commands.entity(entity).remove::<LeftCloud>(),
                CloudDir::Right => commands.entity(entity).remove::<RightCloud>(),
            };

            match adj_clouds[ndx].1 {
                TileOccupation::LeftCloud => {
                    cloud.dir = CloudDir::Left;
                    if is_cooling.val {
                        *texture = asset_server.load("textures/left_cooldown.png");
                    } else {
                        *texture = asset_server.load("textures/left_cloud.png");
                    }
                    commands.entity(entity).insert(LeftCloud);
                }
                TileOccupation::RightCloud => {
                    cloud.dir = CloudDir::Right;
                    if is_cooling.val {
                        *texture = asset_server.load("textures/right_cooldown.png");
                    } else {
                        *texture = asset_server.load("textures/right_cloud.png");
                    }
                    commands.entity(entity).insert(RightCloud);
                }
                TileOccupation::UpCloud => {
                    cloud.dir = CloudDir::Up;
                    if is_cooling.val {
                        *texture = asset_server.load("textures/up_cooldown.png");
                    } else {
                        *texture = asset_server.load("textures/up_cloud.png");
                    }
                    commands.entity(entity).insert(UpCloud);
                }
                TileOccupation::DownCloud => {
                    cloud.dir = CloudDir::Down;
                    if is_cooling.val {
                        *texture = asset_server.load("textures/down_cooldown.png");
                    } else {
                        *texture = asset_server.load("textures/down_cloud.png");
                    }
                    commands.entity(entity).insert(DownCloud);
                }
                _ => {}
            }
        }
    }
    // Reset the counter
    player_control.special_control = 0;
}

/// Deal with the cloud which need to be pushed. At this stage, one already
/// knows that the tile N+2 is empty to push the cloud
#[allow(clippy::type_complexity)]
fn push_clouds(
    mut commands: Commands,
    mut cloud_control: ResMut<CloudControl>,
    mut player_control: ResMut<PlayerControl>,
    mut grid_state: ResMut<GridState>,
    asset_server: Res<AssetServer>,
    mut query: Query<
        (
            &Cloud,
            &mut GridPos,
            Entity,
            &mut IsCooldown,
            &mut Handle<Image>,
        ),
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
                    ];
                }
                CloudDir::Down => {
                    player_control.player_pos = [
                        player_control.player_pos[0],
                        player_control.player_pos[1] - 1,
                    ];
                }
                CloudDir::Left => {
                    player_control.player_pos = [
                        player_control.player_pos[0] - 1,
                        player_control.player_pos[1],
                    ];
                }
                CloudDir::Right => {
                    player_control.player_pos = [
                        player_control.player_pos[0] + 1,
                        player_control.player_pos[1],
                    ];
                }
            }
            grid_state.grid[pos[0] as usize][pos[1] as usize] = TileOccupation::Empty;
            grid_state.grid[player_control.player_pos[0] as usize]
                [player_control.player_pos[1] as usize] = TileOccupation::Player;
            continue;
        }

        // Then push the clouds:
        for (cloud, mut cloud_pos, entity, mut is_cooling, mut texture) in query.iter_mut() {
            if cloud_pos.pos == pos {
                // If cloud to be pushed out of the board, despawn it instantly:
                if push_type == PushState::PushOver {
                    commands
                        .entity(entity)
                        .insert(Transform::from_translation(Vec3::new(
                            -9999., -9999., -9999.,
                        )))
                        .insert(ToDespawn);
                    grid_state.grid[cloud_pos.pos[0] as usize][cloud_pos.pos[1] as usize] =
                        TileOccupation::Empty;
                    continue;
                }

                match dir {
                    CloudDir::Down => {
                        if push_type == PushState::PlayerCanPush {
                            match cloud.dir {
                                CloudDir::Up => {
                                    *texture = asset_server.load("textures/up_cooldown.png");
                                }
                                CloudDir::Down => {
                                    *texture = asset_server.load("textures/down_cooldown.png");
                                }
                                CloudDir::Left => {
                                    *texture = asset_server.load("textures/left_cooldown.png");
                                }
                                CloudDir::Right => {
                                    *texture = asset_server.load("textures/right_cooldown.png");
                                }
                            }
                            is_cooling.val = true;
                        }
                        grid_state.move_on_grid(
                            cloud_pos.pos,
                            [cloud_pos.pos[0], cloud_pos.pos[1] - 1i8],
                            if push_type == PushState::PlayerCanPush {
                                TileOccupation::CooldownCloud
                            } else {
                                match cloud.dir {
                                    CloudDir::Down => TileOccupation::DownCloud,
                                    CloudDir::Up => TileOccupation::UpCloud,
                                    CloudDir::Left => TileOccupation::LeftCloud,
                                    CloudDir::Right => TileOccupation::RightCloud,
                                }
                            },
                        );
                        cloud_pos.is_pushed = true;
                        cloud_pos.old_pos = cloud_pos.pos;
                        cloud_pos.pos[1] += -1i8;
                    }
                    CloudDir::Left => {
                        if push_type == PushState::PlayerCanPush {
                            match cloud.dir {
                                CloudDir::Up => {
                                    *texture = asset_server.load("textures/up_cooldown.png");
                                }
                                CloudDir::Down => {
                                    *texture = asset_server.load("textures/down_cooldown.png");
                                }
                                CloudDir::Left => {
                                    *texture = asset_server.load("textures/left_cooldown.png");
                                }
                                CloudDir::Right => {
                                    *texture = asset_server.load("textures/right_cooldown.png");
                                }
                            }
                            is_cooling.val = true;
                        }
                        grid_state.move_on_grid(
                            cloud_pos.pos,
                            [cloud_pos.pos[0] - 1i8, cloud_pos.pos[1]],
                            if push_type == PushState::PlayerCanPush {
                                TileOccupation::CooldownCloud
                            } else {
                                match cloud.dir {
                                    CloudDir::Down => TileOccupation::DownCloud,
                                    CloudDir::Up => TileOccupation::UpCloud,
                                    CloudDir::Left => TileOccupation::LeftCloud,
                                    CloudDir::Right => TileOccupation::RightCloud,
                                }
                            },
                        );
                        cloud_pos.is_pushed = true;
                        cloud_pos.old_pos = cloud_pos.pos;
                        cloud_pos.pos[0] += -1i8;
                    }
                    CloudDir::Right => {
                        if push_type == PushState::PlayerCanPush {
                            match cloud.dir {
                                CloudDir::Up => {
                                    *texture = asset_server.load("textures/up_cooldown.png");
                                }
                                CloudDir::Down => {
                                    *texture = asset_server.load("textures/down_cooldown.png");
                                }
                                CloudDir::Left => {
                                    *texture = asset_server.load("textures/left_cooldown.png");
                                }
                                CloudDir::Right => {
                                    *texture = asset_server.load("textures/right_cooldown.png");
                                }
                            }
                            is_cooling.val = true;
                        }
                        grid_state.move_on_grid(
                            cloud_pos.pos,
                            [cloud_pos.pos[0] + 1i8, cloud_pos.pos[1]],
                            if push_type == PushState::PlayerCanPush {
                                TileOccupation::CooldownCloud
                            } else {
                                match cloud.dir {
                                    CloudDir::Down => TileOccupation::DownCloud,
                                    CloudDir::Up => TileOccupation::UpCloud,
                                    CloudDir::Left => TileOccupation::LeftCloud,
                                    CloudDir::Right => TileOccupation::RightCloud,
                                }
                            },
                        );
                        cloud_pos.is_pushed = true;
                        cloud_pos.old_pos = cloud_pos.pos;
                        cloud_pos.pos[0] += 1i8;
                    }
                    CloudDir::Up => {
                        if push_type == PushState::PlayerCanPush {
                            match cloud.dir {
                                CloudDir::Up => {
                                    *texture = asset_server.load("textures/up_cooldown.png");
                                }
                                CloudDir::Down => {
                                    *texture = asset_server.load("textures/down_cooldown.png");
                                }
                                CloudDir::Left => {
                                    *texture = asset_server.load("textures/left_cooldown.png");
                                }
                                CloudDir::Right => {
                                    *texture = asset_server.load("textures/right_cooldown.png");
                                }
                            }
                            is_cooling.val = true;
                        }
                        grid_state.move_on_grid(
                            cloud_pos.pos,
                            [cloud_pos.pos[0], cloud_pos.pos[1] + 1i8],
                            if push_type == PushState::PlayerCanPush {
                                TileOccupation::CooldownCloud
                            } else {
                                match cloud.dir {
                                    CloudDir::Down => TileOccupation::DownCloud,
                                    CloudDir::Up => TileOccupation::UpCloud,
                                    CloudDir::Left => TileOccupation::LeftCloud,
                                    CloudDir::Right => TileOccupation::RightCloud,
                                }
                            },
                        );
                        cloud_pos.is_pushed = true;
                        cloud_pos.old_pos = cloud_pos.pos;
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
                    ];
                }
                CloudDir::Down => {
                    player_control.player_pos = [
                        player_control.player_pos[0],
                        player_control.player_pos[1] - 1,
                    ];
                }
                CloudDir::Left => {
                    player_control.player_pos = [
                        player_control.player_pos[0] - 1,
                        player_control.player_pos[1],
                    ];
                }
                CloudDir::Right => {
                    player_control.player_pos = [
                        player_control.player_pos[0] + 1,
                        player_control.player_pos[1],
                    ];
                }
            }
            grid_state.grid[pos[0] as usize][pos[1] as usize] = TileOccupation::Empty;
            grid_state.grid[player_control.player_pos[0] as usize]
                [player_control.player_pos[1] as usize] = TileOccupation::Player;
            continue;
        }

        for (_, mut cloud_pos, _, _, _) in query.iter_mut() {
            if cloud_pos.pos == pos {
                match dir {
                    CloudDir::Down => {
                        grid_state.move_on_grid(
                            cloud_pos.pos,
                            [cloud_pos.pos[0], cloud_pos.pos[1] - 1i8],
                            dir_to_tile(dir),
                        );
                        cloud_pos.is_pushed = false;
                        cloud_pos.old_pos = cloud_pos.pos;
                        cloud_pos.pos[1] += -1i8;
                    }
                    CloudDir::Left => {
                        grid_state.move_on_grid(
                            cloud_pos.pos,
                            [cloud_pos.pos[0] - 1i8, cloud_pos.pos[1]],
                            dir_to_tile(dir),
                        );
                        cloud_pos.is_pushed = false;
                        cloud_pos.old_pos = cloud_pos.pos;
                        cloud_pos.pos[0] += -1i8;
                    }
                    CloudDir::Right => {
                        grid_state.move_on_grid(
                            cloud_pos.pos,
                            [cloud_pos.pos[0] + 1i8, cloud_pos.pos[1]],
                            dir_to_tile(dir),
                        );
                        cloud_pos.is_pushed = false;
                        cloud_pos.old_pos = cloud_pos.pos;
                        cloud_pos.pos[0] += 1i8;
                    }
                    CloudDir::Up => {
                        grid_state.move_on_grid(
                            cloud_pos.pos,
                            [cloud_pos.pos[0], cloud_pos.pos[1] + 1i8],
                            dir_to_tile(dir),
                        );
                        cloud_pos.is_pushed = false;
                        cloud_pos.old_pos = cloud_pos.pos;
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
        cloud_control.spawn_counter[dir_index(uw_cloud_dir)] =
            (cloud_control.spawn_counter[dir_index(uw_cloud_dir)] + 1) % SPAWN_FREQUENCY;
        if cloud_control.spawn_counter[dir_index(uw_cloud_dir)] == 0 {
            cloud_control.cur_new_cloud = cloud_dir;
        } else {
            cloud_control.cur_new_cloud = None;
        }
    }
}

fn set_up_logic(mut commands: Commands, audio_assets: Res<AudioAssets>) {
    /* ▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓ Constants ▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓ */
    let beat_length = match audio_assets.selected_song {
        crate::audio::SelectedSong::Song1 => SONG_1.beat_length,
        crate::audio::SelectedSong::Song2 => SONG_2.beat_length,
    };
    let song_length = match audio_assets.selected_song {
        crate::audio::SelectedSong::Song1 => SONG_1.length,
        crate::audio::SelectedSong::Song2 => SONG_2.length,
    };
    let intro_length = match audio_assets.selected_song {
        crate::audio::SelectedSong::Song1 => SONG_1.intro_length,
        crate::audio::SelectedSong::Song2 => SONG_2.intro_length,
    };
    /* ▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓ Create our game rules resource ▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓ */
    commands.insert_resource(PlayerControl {
        player_pos: INIT_POS,
        input_buffer: [GameControl::Idle; MAX_BUFFER_INPUT],
        input_timer: Timer::from_seconds(
            beat_length / (TIMER_SCALE_FACTOR as f32),
            TimerMode::Repeating,
        ),
        special_control: 0,
        special_timeout: 0,
        animation: AnimationState::Init,
    });
    commands.insert_resource(GridState::default());
    commands.insert_resource(MainClock {
        main_timer: Timer::from_seconds(
            beat_length / (TIMER_SCALE_FACTOR as f32),
            TimerMode::Repeating,
        ),
        absolute_timer: Timer::from_seconds(song_length + intro_length, TimerMode::Repeating),
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

#[allow(clippy::type_complexity)]
fn update_cloud_pos(
    mut commands: Commands,
    mut query: Query<
        (
            &mut GridPos,
            &Transform,
            Entity,
            &mut Animation,
            &mut Sprite,
        ),
        (With<Cloud>,),
    >,
) {
    for (mut cloud_pos, transfo, entity, mut animation, sprite) in query.iter_mut() {
        match animation.state {
            AnimationState::Init | AnimationState::End => {
                if cloud_pos.pos != cloud_pos.old_pos {
                    // Only do a "burst" if the cloud move by itself:
                    if !cloud_pos.is_pushed {
                        let mut orig_sprite = sprite.clone();
                        let mut bigger_sprite = sprite.clone();
                        bigger_sprite.custom_size =
                            Some(Vec2::new(TILE_SIZE, TILE_SIZE) * CLOUD_SCALE_FACTOR_EASING);
                        orig_sprite.custom_size = Some(Vec2::new(TILE_SIZE, TILE_SIZE));
                        let orig_sprite_copy = orig_sprite.clone();
                        commands.entity(entity).insert(
                            orig_sprite
                                .ease_to(
                                    bigger_sprite,
                                    CLOUD_SCALE_EASING,
                                    EasingType::Once {
                                        duration: CLOUD_EASING_DURATION / 2,
                                    },
                                )
                                .ease_to(
                                    orig_sprite_copy,
                                    CLOUD_SCALE_EASING,
                                    EasingType::Once {
                                        duration: CLOUD_EASING_DURATION / 2,
                                    },
                                ),
                        );
                    }
                    // Smooth translation for any kind of move:
                    commands.entity(entity).insert(transfo.ease_to(
                        Transform::from_translation(grid_to_vec(cloud_pos.pos)),
                        CLOUD_EASING,
                        bevy_easings::EasingType::Once {
                            duration: CLOUD_EASING_DURATION,
                        },
                    ));
                    animation.state = AnimationState::Move;
                    cloud_pos.old_pos = cloud_pos.pos;
                }
            }
            AnimationState::Move => (),
        }
    }
}

fn finish_easings(
    mut removed: RemovedComponents<EasingComponent<Transform>>,
    mut query: Query<(&mut Animation, Entity), With<Cloud>>,
) {
    for del_entity in removed.iter() {
        for (mut animation, entity) in query.iter_mut() {
            if entity == del_entity {
                animation.state = AnimationState::End;
            }
        }
    }
}
