#![cfg_attr(debug_assertions, allow(dead_code, unused_imports))]

use crate::GameState;
use bevy::prelude::*;
use colored::*;

pub struct ActionsPlugin;

// This plugin listens for keyboard input and converts the input into Actions
// Actions can then be used as a resource in other systems to act on the player input.
impl Plugin for ActionsPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            set_movement_actions.run_if(in_state(GameState::Playing)),
        )
        .insert_resource(Actions {
            next_action: GameControl::Idle,
        });
    }
}

#[derive(Default, Debug, Resource)]
pub struct Actions {
    pub next_action: GameControl,
}

fn set_movement_actions(mut actions: ResMut<Actions>, keyboard_input: Res<Input<KeyCode>>) {
    let received_input = match_input(keyboard_input);
    match received_input {
        // If Idle, do nothing
        GameControl::Idle => (),
        // Else, replace the input in the actions. So far the action contains
        // only one input at a time
        input => {
            actions.next_action = input;
        }
    };
}

///Enum for the direction. Idle has been added to be able to use an array buffer
/// instead of a vector.
#[derive(Default, Eq, PartialEq, Debug, Copy, Clone, Resource)]
pub enum GameControl {
    #[default]
    Idle,
    Up,
    Down,
    Left,
    Right,
    Special,
}

fn match_input(keyboard_input: Res<Input<KeyCode>>) -> GameControl {
    if keyboard_input.just_released(KeyCode::W) || keyboard_input.just_released(KeyCode::Up) {
        return GameControl::Up;
    }
    if keyboard_input.just_released(KeyCode::S) || keyboard_input.just_released(KeyCode::Down) {
        return GameControl::Down;
    }
    if keyboard_input.just_released(KeyCode::A) || keyboard_input.just_released(KeyCode::Left) {
        return GameControl::Left;
    }
    if keyboard_input.just_released(KeyCode::D) || keyboard_input.just_released(KeyCode::Right) {
        return GameControl::Right;
    }
    if keyboard_input.just_pressed(KeyCode::Space) {
        return GameControl::Special;
    }
    GameControl::Idle
}
