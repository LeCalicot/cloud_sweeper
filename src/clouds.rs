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
    if let Some(cloud_dir) = cloud_control.cur_cloud_dir {
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
    cloud_control.cur_cloud_dir = None;
}

// fn update_cloud_pos(mut query: Query<(&GridPos, &mut Transform), With<Cloud>>) {
//     for (grid_pos, mut transfo) in query.iter_mut() {
//         transfo.translation = Vec3::new(
//             f32::from(grid_pos.pos[0]) * TILE_SIZE + TILE_SIZE / 2.,
//             f32::from(grid_pos.pos[1]) * TILE_SIZE + TILE_SIZE / 2.,
//             CLOUD_LAYER,
//         );
//     }
// }
// fn update_cloud_pos(
//     mut left_query: Query<(&GridPos, &mut Transform), With<LeftCloud>>,
//     mut right_query: Query<(&GridPos, &mut Transform), With<RightCloud>>,
//     mut up_query: Query<(&GridPos, &mut Transform), With<UpCloud>>,
//     mut down_query: Query<(&GridPos, &mut Transform), With<DownCloud>>,
// ) {
//     for (grid_pos, mut transfo) in left_query.iter_mut() {
//         transfo.translation = Vec3::new(
//             f32::from(grid_pos.pos[0]) * TILE_SIZE + TILE_SIZE / 2.,
//             f32::from(grid_pos.pos[1]) * TILE_SIZE + TILE_SIZE / 2.,
//             CLOUD_LAYER,
//         );
//     }
//     for (grid_pos, mut transfo) in right_query.iter_mut() {
//         transfo.translation = Vec3::new(
//             f32::from(grid_pos.pos[0]) * TILE_SIZE + TILE_SIZE / 2.,
//             f32::from(grid_pos.pos[1]) * TILE_SIZE + TILE_SIZE / 2.,
//             CLOUD_LAYER,
//         );
//     }
//     for (grid_pos, mut transfo) in up_query.iter_mut() {
//         transfo.translation = Vec3::new(
//             f32::from(grid_pos.pos[0]) * TILE_SIZE + TILE_SIZE / 2.,
//             f32::from(grid_pos.pos[1]) * TILE_SIZE + TILE_SIZE / 2.,
//             CLOUD_LAYER,
//         );
//     }
//     for (grid_pos, mut transfo) in down_query.iter_mut() {
//         transfo.translation = Vec3::new(
//             f32::from(grid_pos.pos[0]) * TILE_SIZE + TILE_SIZE / 2.,
//             f32::from(grid_pos.pos[1]) * TILE_SIZE + TILE_SIZE / 2.,
//             CLOUD_LAYER,
//         );
//     }
// }

// fn update_cloud_pos(
//     mut transform_query: Query<(Entity, &mut Transform), With<Cloud>>,
//     cloud_query: Query<(Entity, &Cloud)>,
// ) {
//     for (id, mut transfo) in transform_query.iter_mut() {
//         let pos: [i8; 2] = cloud_query
//             .iter()
//             .filter(|(x, y)| id == *x)
//             .map(|(x, y)| y.pos)
//             .collect::<[i8; 2]>()[0];

//         transfo.translation = Vec3::new(
//             f32::from(pos[0]) * TILE_SIZE + TILE_SIZE / 2.,
//             f32::from(pos[1]) * TILE_SIZE + TILE_SIZE / 2.,
//             CLOUD_LAYER,
//         );
//     }
// }
// fn update_cloud_pos(
//     mut cloud_query: Query<(&Cloud, &mut Transform), (Without<Cloud>, With<Cloud>)>,
// ) {
//     for (cloud, mut transform) in cloud_query.iter_mut() {
//         transform.translation = Vec3::new(
//             f32::from(cloud.pos[0]) * TILE_SIZE + TILE_SIZE / 2.,
//             f32::from(cloud.pos[1]) * TILE_SIZE + TILE_SIZE / 2.,
//             CLOUD_LAYER,
//         );
//     }
// }
// fn update_cloud_pos(mut cloud_query: Query<(Entity, &mut Transform), With<Cloud>>) {
//     for (entity, mut transform) in cloud_query.iter_mut() {

//         let cloud = transform.transform.translation = Vec3::new(
//             f32::from(cloud.pos[0]) * TILE_SIZE + TILE_SIZE / 2.,
//             f32::from(cloud.pos[1]) * TILE_SIZE + TILE_SIZE / 2.,
//             CLOUD_LAYER,
//         );
//     }
// }
