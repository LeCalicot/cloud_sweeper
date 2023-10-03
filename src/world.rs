#![cfg_attr(debug_assertions, allow(dead_code, unused_imports))]
// use crate::loading::TextureAssets;
use crate::player::TILE_SIZE;
use crate::GameState;
use bevy::prelude::*;
use bevy_ecs_tilemap::helpers::geometry::get_tilemap_center_transform;
use bevy_ecs_tilemap::map::TilemapTexture;
use bevy_ecs_tilemap::prelude::*;
// use bevy_prototype_debug_lines::DebugLines;
use colored::*;

pub struct WorldPlugin;

#[derive(Component)]
pub struct World;

#[derive(Component)]
pub struct Level1;

#[derive(Component)]
pub struct Sky;

#[derive(Component)]
pub struct Platform;

#[derive(Component)]
pub struct AllTiles;

#[derive(Component)]
pub struct TileMapEntity;

pub const LEVEL_SIZE: u32 = 10;
pub const STAGE_WIDTH: u32 = 6;
pub const STAGE_BL: [u32; 2] = [2, 2];
pub const STAGE_UR: [u32; 2] = [7, 7];
pub const CAMERA_LAYER: f32 = 500.;
pub const DISPLAY_RATIO: f32 = 1. / 4.;

/// This plugin handles world related stuff: background, cloud movement,...
impl Plugin for WorldPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(GameState::Playing), setup_world)
            // .add_enter_system(GameState::Playing, spawn_world)
            .add_systems(Update, update_world.run_if(in_state(GameState::Playing)))
            // .insert_resource(ImageSettings::default_nearest())
            .add_plugins(TilemapPlugin);
    }
}

fn setup_world(mut commands: Commands, asset_server: Res<AssetServer>) {
    // commands.spawn_bundle(Camera2dBundle::default());

    let texture_handle: Handle<Image> = asset_server.load("textures/tiles.png");

    let tilemap_size = TilemapSize {
        x: LEVEL_SIZE,
        y: LEVEL_SIZE,
    };
    let tilemap_entity = commands.spawn_empty().insert(TileMapEntity).id();
    let mut tile_storage = TileStorage::empty(tilemap_size);

    for x in 0..tilemap_size.x {
        for y in 0..tilemap_size.y {
            let tile_pos = TilePos { x, y };

            let tile_entity = commands
                .spawn(TileBundle {
                    position: tile_pos,
                    texture_index: TileTextureIndex(1),
                    tilemap_id: TilemapId(tilemap_entity),
                    ..Default::default()
                })
                .insert(AllTiles)
                .id();
            commands.entity(tilemap_entity).add_child(tile_entity);
            if (STAGE_BL[0] <= x) && (x <= STAGE_UR[0]) && (STAGE_BL[1] <= y) && (y <= STAGE_UR[1])
            {
                commands
                    .entity(tile_entity)
                    .insert(Platform)
                    .insert(TileTextureIndex(0));
            } else {
                commands
                    .entity(tile_entity)
                    .insert(Sky)
                    .insert(TileTextureIndex(1));
            }
            tile_storage.set(&tile_pos, tile_entity);
        }
    }

    let tile_size = TilemapTileSize {
        x: TILE_SIZE,
        y: TILE_SIZE,
    };
    let grid_size = TilemapGridSize {
        x: TILE_SIZE,
        y: TILE_SIZE,
    };

    commands.entity(tilemap_entity).insert(TilemapBundle {
        grid_size,
        size: tilemap_size,
        storage: tile_storage,
        texture: TilemapTexture::Single(texture_handle),
        tile_size,
        transform: get_tilemap_center_transform(
            &tilemap_size,
            &grid_size,
            &TilemapType::Square,
            0.,
            // ) * Transform::from_xyz(0.0, TILE_SIZE / 2., 0.0),
        ) * Transform::from_xyz(-TILE_SIZE / 2., 0., 0.0),
        ..Default::default()
    });
}

fn update_world() {}
