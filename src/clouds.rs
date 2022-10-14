use crate::{
    actions::GameControl,
    logic::{CloudControl, GridState},
    GameState,
};
use bevy::prelude::*;
use colored::*;
use iyes_loopless::prelude::*;

pub const CLOUD_LAYER: f32 = 9.;

pub struct CloudPlugin;

#[derive(Component)]
pub struct Cloud;

#[derive(Default, Eq, PartialEq, Debug, Copy, Clone)]
pub enum CloudDir {
    #[default]
    Up,
    Down,
    Left,
    Right,
}

impl Plugin for CloudPlugin {
    fn build(&self, app: &mut App) {
        app.add_system_set(
            ConditionSet::new()
                .run_in_state(GameState::Playing)
                .with_system(new_cloud)
                .into(),
        );
    }
}

fn new_cloud(
    mut cloud_control: ResMut<CloudControl>,
    mut grid_state: ResMut<GridState>,
    mut commands: Commands,
    asset_server: Res<AssetServer>,
) {
    if cloud_control.new_cloud.is_some() {
        println!("{} {} {:?}", { "➤".blue() }, { ":".blue() }, {
            cloud_control.new_cloud
        });

        let texture = match cloud_control.new_cloud.unwrap() {
            CloudDir::Down => "textures/down_cloud.png",
            CloudDir::Left => "textures/left_cloud.png",
            CloudDir::Right => "textures/right_cloud.png",
            CloudDir::Up => "textures/up_cloud.png",
        };

        if let Some(cloud_pos) = grid_state.new_cloud(cloud_control.new_cloud.unwrap()) {
            commands.spawn_bundle(SpriteBundle {
                texture: asset_server.load(texture),
                transform: Transform::from_translation(cloud_pos),
                ..default()
            });
        }
        cloud_control.new_cloud = None;
    }
}
