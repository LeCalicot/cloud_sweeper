use crate::actions::{Actions, GameControl};
use crate::loading::TextureAssets;
use crate::logic::PlayerControl;
use crate::GameState;
use bevy::prelude::*;
use bevy::render::texture::ImageSettings;
use iyes_loopless::prelude::*;

pub struct PlayerPlugin;

#[derive(Component)]
pub struct Player;

/// This plugin handles player related stuff like movement
/// Player logic is only active during the State `GameState::Playing`
impl Plugin for PlayerPlugin {
    fn build(&self, app: &mut App) {
        app.add_enter_system(GameState::Playing, spawn_player)
            .add_system_set(
                ConditionSet::new()
                    .run_in_state(GameState::Playing)
                    .with_system(animate_sprite)
                    .with_system(move_player)
                    .into(),
            );
    }
}

#[derive(Component, Deref, DerefMut)]
struct AnimationTimer(Timer);

fn animate_sprite(
    time: Res<Time>,
    texture_atlases: Res<Assets<TextureAtlas>>,
    mut query: Query<(
        &mut AnimationTimer,
        &mut TextureAtlasSprite,
        &Handle<TextureAtlas>,
    )>,
) {
    for (mut timer, mut sprite, texture_atlas_handle) in &mut query {
        timer.tick(time.delta());
        if timer.just_finished() {
            let texture_atlas = texture_atlases.get(texture_atlas_handle).unwrap();
            sprite.index = (sprite.index + 1) % texture_atlas.textures.len();
        }
    }
}

fn spawn_player(
    mut commands: Commands,
    mut texture_atlases: ResMut<Assets<TextureAtlas>>,
    asset_server: Res<AssetServer>,
) {
    let texture_handle = asset_server.load("textures/duck_spritesheet.png");
    let texture_atlas = TextureAtlas::from_grid(texture_handle, Vec2::new(16.0, 16.0), 1, 4);
    let texture_atlas_handle = texture_atlases.add(texture_atlas);
    commands
        .spawn_bundle(SpriteSheetBundle {
            texture_atlas: texture_atlas_handle,
            transform: Transform::from_xyz(8., 8., 10.),
            // transform: Transform::from_scale(Vec3::splat(6.0)),
            ..default()
        })
        .insert(Player)
        .insert(AnimationTimer(Timer::from_seconds(0.1, true)));
}

fn move_player(
    time: Res<Time>,
    actions: Res<Actions>,
    mut player_control: ResMut<PlayerControl>, // mut player_query: Query<&mut Transform, With<Player>>,
) {
    if actions.next_move == GameControl::Idle {
        return;
    }

    player_control.move_player(actions.next_move);
    // let speed = 150.;
    // let movement = Vec3::new(
    //     actions.player_movement.unwrap().x * speed * time.delta_seconds(),
    //     actions.player_movement.unwrap().y * speed * time.delta_seconds(),
    //     0.,
    // );
    // for mut player_transform in &mut player_query {
    //     player_transform.translation += movement;
    // }
}
