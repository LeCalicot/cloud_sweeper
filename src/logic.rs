#![cfg_attr(debug_assertions, allow(dead_code, unused_imports))]

use crate::actions::Actions;
use crate::actions::GameControl;
use crate::loading::TextureAssets;
use crate::player::Player;
use crate::GameState;
use bevy::prelude::*;
use bevy::render::texture::ImageSettings;
use colored::*;
use iyes_loopless::prelude::*;

const MAX_BUFFER_INPUT: usize = 10;
const TIMER_DUR: f32 = 0.1;

pub struct LogicPlugin;

/// Contains the info about the player
///
/// The bufferis a FIFO, with the oldest element at index 0.
#[derive(Default)]
pub struct PlayerControl {
    pub player_pos: [i8; 2],
    input_buffer: [GameControl; MAX_BUFFER_INPUT],
    timer: Timer,
}

/// This plugin handles player related stuff like movement
/// Player logic is only active during the State `GameState::Playing`
impl Plugin for LogicPlugin {
    fn build(&self, app: &mut App) {
        app.add_enter_system(GameState::Playing, set_up_logic)
            .add_system_set(
                ConditionSet::new()
                    .run_in_state(GameState::Playing)
                    .label("fill_player_buffer")
                    .after("pop_player_buffer")
                    .with_system(fill_player_buffer)
                    .into(),
            )
            .add_system_set(
                ConditionSet::new()
                    .run_in_state(GameState::Playing)
                    .before("fill_player_buffer")
                    .label("pop_player_buffer")
                    .with_system(pop_player_buffer)
                    .into(),
            );
    }
}

// https://github.com/IyesGames/iyes_loopless

#[derive(Component, Deref, DerefMut)]
struct AnimationTimer(Timer);

fn set_up_logic(mut commands: Commands) {
    // Create our game rules resource
    commands.insert_resource(PlayerControl {
        player_pos: [0, 0],
        input_buffer: [GameControl::Idle; MAX_BUFFER_INPUT],
        timer: Timer::from_seconds(TIMER_DUR, true),
    });
}

/// Add all the actions (moves) to the buffer whose elements are going to be popped
pub fn fill_player_buffer(mut actions: ResMut<Actions>, mut player_control: ResMut<PlayerControl>) {
    let game_control = actions.next_move;
    let idle_ndx = player_control
        .input_buffer
        .iter()
        .position(|x| x == &GameControl::Idle);

    if game_control != GameControl::Idle {
        // The buffer is not full, we can replace the first idle element
        if let Some(x) = idle_ndx {
            player_control.input_buffer[x] = game_control;
        }
        // The buffer is full, replace the last element:
        else {
            let n = player_control.input_buffer.len() - 1;
            player_control.input_buffer[n] = game_control;
        }
    };
    actions.next_move = GameControl::Idle;
}

/// Pop and applies all the player moves when the timer expires
pub fn pop_player_buffer(mut player_control: ResMut<PlayerControl>, time: Res<Time>) {
    // timers gotta be ticked, to work
    player_control.timer.tick(time.delta());

    // if it finished, despawn the bomb
    if player_control.timer.finished() {
        let player_action = player_control.input_buffer[0];
        player_control.input_buffer[0] = GameControl::Idle;
        player_control.input_buffer.rotate_left(1);

        let player_move: [i8; 2] = match player_action {
            GameControl::Down => [0, -1],
            GameControl::Up => [0, 1],
            GameControl::Left => [-1, 0],
            GameControl::Right => [1, 0],
            GameControl::Idle => [0, 0],
        };
        let player_abs_pos = [
            player_control.player_pos[0] + player_move[0],
            player_control.player_pos[1] + player_move[1],
        ];
        player_control.player_pos = player_abs_pos;
        // TODO: forbid going out of the grid
        if player_action != GameControl::Idle {
            info!("pl. pos: {:?}", player_control.player_pos)
        }
    };
}
