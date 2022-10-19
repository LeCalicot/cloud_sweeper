use crate::{
    actions::GameControl,
    logic::{CloudControl, GridState},
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
pub struct Cloud;

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

// impl Plugin for CloudPlugin {
//     fn build(&self, app: &mut App) {
//         app.add_system_set(
//             ConditionSet::new()
//                 .run_in_state(GameState::Playing)
//                 .with_system(new_cloud)
//                 // .with_system(update_cloud_pos)
//                 .into(),
//         );
//     }
// }

// WIP:initialize the position in the grid when inserting Cloud

pub fn new_cloud(
    mut cloud_control: ResMut<CloudControl>,
    mut grid_state: ResMut<GridState>,
    mut commands: Commands,
    asset_server: Res<AssetServer>,
) {
    if let Some(cloud_dir) = cloud_control.cur_new_cloud {
        println!("{} {} {:?}", { "➤ New Cloud".blue() }, { ":".blue() }, {
            cloud_dir
        });

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
                    .insert(Cloud)
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
                    .insert(Cloud)
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
                    .insert(Cloud)
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
                    .insert(Cloud)
                    .insert(GridPos {
                        pos: cloud_pos_grid,
                    });
            }
        }
    }
    cloud_control.cur_new_cloud = None;
}
