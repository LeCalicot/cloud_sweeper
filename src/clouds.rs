use crate::{
    actions::GameControl,
    logic::{CloudControl, GridState, PUSH_COOLDOWN},
    player::TILE_SIZE,
    GameState,
};
use bevy::prelude::*;
use colored::*;
use iyes_loopless::prelude::*;

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
}

#[derive(Component)]
pub struct LeftCloud;

#[derive(Component)]
pub struct RightCloud;
#[derive(Component)]
pub struct UpCloud;
#[derive(Component)]
pub struct DownCloud;

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

#[derive(Component, Deref, DerefMut)]
pub struct CooldownTimer {
    pub timer: Timer,
}

pub fn new_cloud(
    mut cloud_control: ResMut<CloudControl>,
    mut grid_state: ResMut<GridState>,
    mut commands: Commands,
    asset_server: Res<AssetServer>,
) {
    if let Some(cloud_dir) = cloud_control.cur_new_cloud {
        // Spawn a new cloud, with a sprite bundle, associate the direction
        if let Some((cloud_pos_vec, cloud_pos_grid)) = grid_state.new_cloud(cloud_dir) {
            if cloud_dir == CloudDir::Down {
                commands
                    .spawn_bundle(SpriteBundle {
                        texture: asset_server.load("textures/down_cloud.png"),
                        transform: Transform::from_translation(cloud_pos_vec),
                        ..default()
                    })
                    .insert(DownCloud)
                    .insert(CooldownTimer {
                        timer: Timer::from_seconds(PUSH_COOLDOWN, true),
                    })
                    .insert(Cloud {
                        dir: CloudDir::Down,
                    })
                    .insert(IsCooldown { val: false })
                    .insert(GridPos {
                        pos: cloud_pos_grid,
                    });
            }
            if cloud_dir == CloudDir::Left {
                commands
                    .spawn_bundle(SpriteBundle {
                        texture: asset_server.load("textures/left_cloud.png"),
                        transform: Transform::from_translation(cloud_pos_vec),
                        ..default()
                    })
                    .insert(LeftCloud)
                    .insert(CooldownTimer {
                        timer: Timer::from_seconds(PUSH_COOLDOWN, true),
                    })
                    .insert(Cloud {
                        dir: CloudDir::Left,
                    })
                    .insert(IsCooldown { val: false })
                    .insert(GridPos {
                        pos: cloud_pos_grid,
                    });
            }
            if cloud_dir == CloudDir::Up {
                commands
                    .spawn_bundle(SpriteBundle {
                        texture: asset_server.load("textures/up_cloud.png"),
                        transform: Transform::from_translation(cloud_pos_vec),
                        ..default()
                    })
                    .insert(UpCloud)
                    .insert(CooldownTimer {
                        timer: Timer::from_seconds(PUSH_COOLDOWN, true),
                    })
                    .insert(IsCooldown { val: false })
                    .insert(Cloud { dir: CloudDir::Up })
                    .insert(GridPos {
                        pos: cloud_pos_grid,
                    });
            }
            if cloud_dir == CloudDir::Right {
                commands
                    .spawn_bundle(SpriteBundle {
                        texture: asset_server.load("textures/right_cloud.png"),
                        transform: Transform::from_translation(cloud_pos_vec),
                        ..default()
                    })
                    .insert(RightCloud)
                    .insert(Cloud {
                        dir: CloudDir::Right,
                    })
                    .insert(CooldownTimer {
                        timer: Timer::from_seconds(PUSH_COOLDOWN, true),
                    })
                    .insert(IsCooldown { val: false })
                    .insert(GridPos {
                        pos: cloud_pos_grid,
                    });
            }
        }
    }
    cloud_control.cur_new_cloud = None;
}
