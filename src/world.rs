// use crate::loading::TextureAssets;
use crate::GameState;
use bevy::{prelude::*, render::texture::ImageSettings};
use bevy_ecs_tilemap::helpers::get_centered_transform_2d;
use bevy_ecs_tilemap::prelude::*;
use bevy_prototype_debug_lines::DebugLines;
use iyes_loopless::prelude::*;

pub struct WorldPlugin;

#[derive(Component)]
pub struct World;

#[derive(Component)]
pub struct Level1;

#[derive(Component)]
pub struct Sky;

#[derive(Component)]
pub struct Platform;

const STAGE_BL: [u32; 2] = [2, 2];
const STAGE_UR: [u32; 2] = [7, 7];

/// This plugin handles world related stuff: background, cloud movement,...
impl Plugin for WorldPlugin {
    fn build(&self, app: &mut App) {
        app.add_enter_system(GameState::Playing, setup_world)
            // .add_enter_system(GameState::Playing, spawn_world)
            .add_system_set(
                ConditionSet::new()
                    .run_in_state(GameState::Playing)
                    .with_system(update_world)
                    // .with_system(draw_grid)
                    .into(),
            )
            .insert_resource(ImageSettings::default_nearest())
            .add_plugin(TilemapPlugin);
    }
}

fn setup_world(mut commands: Commands, asset_server: Res<AssetServer>) {
    // commands.spawn_bundle(Camera2dBundle::default());

    let texture_handle: Handle<Image> = asset_server.load("textures/tiles.png");

    let tilemap_size = TilemapSize { x: 10, y: 10 };
    let tilemap_entity = commands.spawn().id();
    let mut tile_storage = TileStorage::empty(tilemap_size);

    for x in 0..tilemap_size.x {
        for y in 0..tilemap_size.y {
            let tile_pos = TilePos { x, y };

            let tile_entity = commands
                .spawn()
                .insert_bundle(TileBundle {
                    position: tile_pos,
                    texture: TileTexture(1),
                    tilemap_id: TilemapId(tilemap_entity),
                    ..Default::default()
                })
                .id();
            if (STAGE_BL[0] <= x) && (x <= STAGE_UR[0]) && (STAGE_BL[1] <= y) && (y <= STAGE_UR[1])
            {
                commands
                    .entity(tile_entity)
                    .insert(Platform)
                    .insert(TileTexture(0));
            } else {
                commands
                    .entity(tile_entity)
                    .insert(Sky)
                    .insert(TileTexture(1));
            }
            tile_storage.set(&tile_pos, Some(tile_entity));
        }
    }

    let tile_size = TilemapTileSize { x: 16.0, y: 16.0 };
    let grid_size = TilemapGridSize { x: 16.0, y: 16.0 };

    commands
        .entity(tilemap_entity)
        .insert_bundle(TilemapBundle {
            grid_size,
            size: tilemap_size,
            storage: tile_storage,
            texture: TilemapTexture(texture_handle),
            tile_size,
            transform: get_centered_transform_2d(&tilemap_size, &tile_size, 0.0),
            ..Default::default()
        });
}

fn draw_grid(tile_query: Query<&mut TileVisible>) {}

fn update_world() {}
