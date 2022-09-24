use crate::loading::TextureAssets;
use crate::GameState;
use bevy::prelude::*;
use bevy_prototype_debug_lines::{DebugLines, DebugLinesPlugin};
use iyes_loopless::prelude::*;

pub struct WorldPlugin;

#[derive(Component)]
pub struct World;

/// This plugin handles world related stuff: background, cloud movement,...
impl Plugin for WorldPlugin {
    fn build(&self, app: &mut App) {
        app.add_startup_system(setup_world)
            // .add_enter_system(GameState::Playing, spawn_world)
            .add_system_set(
                ConditionSet::new()
                    .run_in_state(GameState::Playing)
                    .with_system(update_world)
                    .with_system(draw_grid)
                    .into(),
            );
    }
}

fn setup_world(mut commands: Commands) {
    commands.spawn_bundle(Camera2dBundle::default());
}

fn draw_grid(mut lines: ResMut<DebugLines>) {
    lines.line(
        Vec3::new(-400.0, 200.0, 0.0),
        Vec3::new(400.0, 200.0, 0.0),
        0.0,
    );
}

fn update_world() {}
