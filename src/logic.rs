use crate::actions::Actions;
use crate::actions::GameControl;
use crate::loading::TextureAssets;
use crate::GameState;
use bevy::prelude::*;
use bevy::render::texture::ImageSettings;
use iyes_loopless::prelude::*;

const MAX_BUFFER_INPUT: usize = 10;

pub struct LogicPlugin;

#[derive(Default)]
pub struct PlayerControl {
    player_pos: [u8; 2],
    input_buffer: [GameControl; MAX_BUFFER_INPUT],
}

/// This plugin handles player related stuff like movement
/// Player logic is only active during the State `GameState::Playing`
impl Plugin for LogicPlugin {
    fn build(&self, app: &mut App) {
        app.add_enter_system(GameState::Playing, set_up_logic);
    }
}

#[derive(Component, Deref, DerefMut)]
struct AnimationTimer(Timer);

fn set_up_logic(mut commands: Commands) {
    // Create our game rules resource
    commands.insert_resource(PlayerControl::default());
}

impl PlayerControl {
    pub fn move_player(&mut self, game_control: GameControl) {
        let non_idle_ndx = self
            .input_buffer
            .iter()
            .position(|x| x != &GameControl::Idle);
        println!("{game_control:?}");
        if let Some(x) = non_idle_ndx {
            self.input_buffer[x] = game_control;
        } else {
            let n = self.input_buffer.len() - 1;
            self.input_buffer[n] = game_control;
        }
    }
}
