use crate::{logic::CloudControl, GameState};
use bevy::prelude::*;
use colored::*;
use iyes_loopless::prelude::*;

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

fn new_cloud(mut cloud_control: ResMut<CloudControl>) {
    if cloud_control.new_cloud.is_some() {
        println!("{} {} {:?}", { "➤".blue() }, { ":".blue() }, {
            cloud_control.new_cloud
        });
    };
    cloud_control.new_cloud = None;
}
