use bevy::prelude::*;
use bevy_ecs_tilemap::prelude::*;
use bevy_ecs_tilemap::tiles::TileStorage;
use colored::*;
use iyes_loopless::prelude::*;

use crate::{
    logic::{GridState, CLOUD_COUNT_LOSE_COND},
    player::TILE_SIZE,
    world::LEVEL_SIZE,
    GameState,
};

pub struct UiPlugin;

#[derive(Component, Default)]
pub struct MessBar {
    pub counter: usize,
}

#[derive(Component)]
pub struct MessTile;

/// This plugin handles the UI interface
impl Plugin for UiPlugin {
    fn build(&self, app: &mut App) {
        app.add_enter_system(GameState::Playing, setup_mess_bar)
            .add_system_set(
                ConditionSet::new()
                    .run_in_state(GameState::Playing)
                    .with_system(update_mess_bar)
                    .into(),
            );
    }
}

fn setup_mess_bar(mut commands: Commands, asset_server: Res<AssetServer>) {
    let texture_handle: Handle<Image> = asset_server.load("textures/mess_bar.png");
    // let texture_atlas =
    //     TextureAtlas::from_grid(texture_handle, Vec2::new(TILE_SIZE, TILE_SIZE), 6, 1);
    let tilemap_size = TilemapSize {
        x: 1,
        y: LEVEL_SIZE,
    };
    let tile_size = TilemapTileSize {
        x: TILE_SIZE,
        y: TILE_SIZE,
    };
    let grid_size = TilemapGridSize {
        x: TILE_SIZE,
        y: TILE_SIZE,
    };

    let tilemap_entity = commands.spawn_empty().id();
    let mut tile_storage = TileStorage::empty(tilemap_size);

    for y in 0..tilemap_size.y {
        let tile_pos = TilePos { x: 0, y };

        let tile_entity = commands
            .spawn_empty()
            .insert_bundle(TileBundle {
                position: tile_pos,
                texture_index: TileTextureIndex(1 + y / 2),
                tilemap_id: TilemapId(tilemap_entity),
                ..Default::default()
            })
            .id();
        commands.entity(tile_entity).insert(MessTile);
        tile_storage.set(&tile_pos, Some(tile_entity));
    }

    commands
        .entity(tilemap_entity)
        .insert_bundle(TilemapBundle {
            grid_size,
            size: tilemap_size,
            storage: tile_storage,
            texture: TilemapTexture::Single(texture_handle),
            tile_size,
            transform: get_mess_tile_pos(0, 100.),
            ..Default::default()
        })
        .insert(MessBar::default());
}

/// Method to compute the positions of the blocks of the load bar
pub fn get_mess_tile_pos(ndx: u32, z: f32) -> Transform {
    Transform::from_xyz(
        ((LEVEL_SIZE as f32) / 2. - 0.5) * TILE_SIZE,
        -TILE_SIZE * (LEVEL_SIZE as f32) / 2. + ndx as f32,
        z,
    )
}

fn update_mess_bar(
    mut commands: Commands,
    mess_query: Query<&mut MessBar>,
    mut tile_query: Query<(&TilePos, &mut TileVisible), With<MessTile>>,
) {
    // The counter is duplicated...
    let mess_counter = mess_query.into_iter().collect::<Vec<&MessBar>>()[0].counter;

    let threshold: f32 = mess_counter as f32 * LEVEL_SIZE as f32 / CLOUD_COUNT_LOSE_COND as f32;

    for (pos, mut vis) in tile_query.iter_mut() {
        if pos.y <= threshold as u32 {
            vis.0 = true;
        } else {
            vis.0 = false;
        }
    }

    if mess_counter > CLOUD_COUNT_LOSE_COND {
        commands.insert_resource(NextState(GameState::GameOver))
    }
}
