#![cfg_attr(debug_assertions, allow(dead_code, unused_imports))]

use crate::logic::GridState;
use crate::player::Player;
use crate::ui::{MessBar, MessTile};
use crate::GameState;
use crate::{clouds::Cloud, loading::FontAssets};
use bevy::app::AppExit;
use bevy::prelude::*;
use bevy::text::BreakLineOn;
use bevy::window::close_on_esc;
use bevy_kira_audio::prelude::*;
use bevy_kira_audio::{Audio, AudioEasing, AudioTween};
use {AlignItems, BackgroundColor, JustifyContent, UiRect};

#[cfg(debug_assertions)]
const AUTOSTART_TIME_MS: u64 = 1000;
use crate::world::{Platform, Sky, CAMERA_LAYER, DISPLAY_RATIO};
use std::time::Duration;

#[cfg(debug_assertions)]
#[derive(Resource, Default)]
struct DebugVariables {
    has_playing: bool,
    has_game_over: bool,
}

pub struct MenuPlugin;

/// This plugin is responsible for the game menu (containing only one button...)
/// The menu is only drawn during the State `GameState::Menu` and is removed when that state is exited
impl Plugin for MenuPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<ButtonColors>()
            .add_system(setup_menu.in_schedule(OnEnter(GameState::Menu)))
            .add_system(click_play_button.run_if(in_state(GameState::Menu)))
            // .add_system(click_play_button.in_schedule(OnEnter(GameState::Menu)))
            .add_system(cleanup_menu.in_schedule(OnExit(GameState::Menu)))
            .add_systems((
                click_play_button.run_if(in_state(GameState::Menu)),
                close_on_esc.run_if(in_state(GameState::Menu)),
            ))
            .add_system(setup_game_over_screen.in_schedule(OnEnter(GameState::GameOver)))
            .add_system(game_over_clear.in_schedule(OnEnter(GameState::GameOver)))
            .add_system(exit_game_over_menu.in_schedule(OnExit(GameState::GameOver)))
            .add_system(game_over_screen.run_if(in_state(GameState::GameOver)))
            .add_system(click_quit_button.run_if(in_state(GameState::GameOver)));
        #[cfg(debug_assertions)]
        {
            app.init_resource::<DebugVariables>()
                .add_system(debug_start_auto.run_if(in_state(GameState::Menu)))
                .add_system(debug_auto_loss.run_if(in_state(GameState::Playing)));

            // /.add_plugin(FrameTimeDiagnosticsPlugin::default())
            // .add_plugin(LogDiagnosticsPlugin::default())
        }
    }
}

#[derive(Resource)]
struct ButtonColors {
    normal: BackgroundColor,
    hovered: BackgroundColor,
}

#[derive(Component)]
pub struct GameOver;
#[derive(Component)]
pub struct MainMenu;
#[derive(Component)]
pub struct QuitGame;

impl Default for ButtonColors {
    fn default() -> Self {
        ButtonColors {
            normal: Color::rgb(0.15, 0.15, 0.15).into(),
            hovered: Color::rgb(0.25, 0.25, 0.25).into(),
        }
    }
}

fn setup_menu(
    mut commands: Commands,
    font_assets: Res<FontAssets>,
    // button_colors: Res<ButtonColors>,
    query: Query<Entity, With<Camera2d>>,
) {
    if query.into_iter().count() == 0 {
        commands.spawn(Camera2dBundle::default()).insert(
            Transform::from_xyz(0., 0., CAMERA_LAYER).with_scale(Vec3::new(
                DISPLAY_RATIO,
                DISPLAY_RATIO,
                1.,
            )),
        );
    }
    // .insert(Transform::from_scale(Vec3::new(1. / 4., 1. / 4., 1.)));
    // commands.spawn_bundle(Camera2dBundle::default());
    commands
        .spawn(ButtonBundle {
            style: Style {
                size: Size::new(Val::Px(120.0), Val::Px(50.0)),
                margin: UiRect::all(Val::Auto),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..Default::default()
            },
            // color: button_colors.normal,
            ..Default::default()
        })
        .insert(MainMenu)
        .with_children(|parent| {
            parent.spawn(TextBundle {
                text: Text {
                    sections: vec![TextSection {
                        value: "Play".to_string(),
                        style: TextStyle {
                            font: font_assets.fira_sans.clone(),
                            font_size: 40.0,
                            color: Color::rgb(0.9, 0.9, 0.9),
                        },
                    }],
                    alignment: TextAlignment::Center,
                    linebreak_behaviour: BreakLineOn::WordBoundary,
                },
                ..Default::default()
            });
        });
}

#[allow(clippy::type_complexity)]
fn click_play_button(
    button_colors: Res<ButtonColors>,
    mut next_state: ResMut<NextState<GameState>>,
    mut interaction_query: Query<
        (&Interaction, &mut BackgroundColor),
        (Changed<Interaction>, With<MainMenu>),
    >,
) {
    for (interaction, mut color) in &mut interaction_query {
        match *interaction {
            Interaction::Clicked => next_state.set(GameState::Playing),
            Interaction::Hovered => {
                *color = button_colors.hovered;
            }
            Interaction::None => {
                *color = button_colors.normal;
            }
        }
    }
}

#[allow(clippy::type_complexity)]
fn click_quit_button(
    button_colors: Res<ButtonColors>,
    mut interaction_query: Query<
        (&Interaction, &mut BackgroundColor),
        (Changed<Interaction>, With<QuitGame>),
    >,
    mut exit: EventWriter<AppExit>,
) {
    for (interaction, mut color) in &mut interaction_query {
        match *interaction {
            Interaction::Clicked => {
                exit.send(AppExit);
            }
            Interaction::Hovered => {
                *color = button_colors.hovered;
            }
            Interaction::None => {
                *color = button_colors.normal;
            }
        }
    }
}

#[cfg(debug_assertions)]
fn debug_start_auto(
    time: Res<Time>,
    mut next_state: ResMut<NextState<GameState>>,
    mut debug_var: ResMut<DebugVariables>,
) {
    use colored::Colorize;

    if time.elapsed() > Duration::from_millis(AUTOSTART_TIME_MS) && !debug_var.has_playing {
        println!(
            "{} {} {:?}",
            { colored::Colorize::blue("➤") },
            { "AAA:".blue() },
            { "enter playing state" }
        );
        debug_var.has_playing = true;
        next_state.set(GameState::Playing);
    }
}

#[cfg(debug_assertions)]
fn debug_auto_loss(
    time: Res<Time>,
    mut next_state: ResMut<NextState<GameState>>,
    mut debug_var: ResMut<DebugVariables>,
) {
    use colored::Colorize;

    if time.elapsed() > Duration::from_millis(5 * AUTOSTART_TIME_MS) && !debug_var.has_game_over {
        println!(
            "{} {} {:?}",
            { colored::Colorize::blue("➤") },
            { "CCC:".blue() },
            { "Automatic game over" }
        );
        debug_var.has_game_over = true;
        next_state.set(GameState::GameOver);
    }
}

fn cleanup_menu(mut commands: Commands, button: Query<Entity, With<MainMenu>>) {
    commands.entity(button.single()).despawn_recursive();
}

fn setup_game_over_screen(
    mut commands: Commands,
    // button_colors: Res<ButtonColors>,
    font_assets: Res<FontAssets>,
) {
    commands
        .spawn(ButtonBundle {
            style: Style {
                size: Size::new(Val::Px(120.0), Val::Px(50.0)),
                margin: UiRect::all(Val::Auto),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..Default::default()
            },
            // color: button_colors.normal,/
            ..Default::default()
        })
        .with_children(|parent| {
            parent.spawn(TextBundle {
                text: Text {
                    sections: vec![TextSection {
                        value: "Retry".to_string(),
                        style: TextStyle {
                            font: font_assets.fira_sans.clone(),
                            font_size: 40.0,
                            color: Color::rgb(0.9, 0.9, 0.9),
                        },
                    }],
                    alignment: TextAlignment::Center,
                    linebreak_behaviour: BreakLineOn::WordBoundary,
                },
                ..Default::default()
            });
        })
        .insert(GameOver);

    commands
        .spawn(ButtonBundle {
            style: Style {
                size: Size::new(Val::Px(120.0), Val::Px(50.0)),
                margin: UiRect::all(Val::Auto),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..Default::default()
            },
            // color: button_colors.normal,
            ..Default::default()
        })
        .insert(QuitGame)
        .with_children(|parent| {
            parent.spawn(TextBundle {
                text: Text {
                    sections: vec![TextSection {
                        value: "Quit".to_string(),
                        style: TextStyle {
                            font: font_assets.fira_sans.clone(),
                            font_size: 40.0,
                            color: Color::rgb(0.9, 0.9, 0.9),
                        },
                    }],
                    alignment: TextAlignment::Center,
                    linebreak_behaviour: BreakLineOn::WordBoundary,
                },
                ..Default::default()
            });
        })
        .insert(GameOver);
}

#[allow(clippy::type_complexity)]
fn game_over_screen(
    mut interaction_query: Query<
        (&Interaction, &mut BackgroundColor),
        (Changed<Interaction>, With<Button>, With<GameOver>),
    >,
    mut next_state: ResMut<NextState<GameState>>,
    button_colors: Res<ButtonColors>,
) {
    for (interaction, mut color) in &mut interaction_query {
        match *interaction {
            Interaction::Clicked => next_state.set(GameState::Menu),
            Interaction::Hovered => {
                *color = button_colors.hovered;
            }
            Interaction::None => {
                *color = button_colors.normal;
            }
        }
    }
}

#[allow(clippy::type_complexity)]
fn game_over_clear(
    mut commands: Commands,
    mut query: Query<
        Entity,
        Or<(
            With<Cloud>,
            With<Player>,
            With<Sky>,
            With<Platform>,
            With<MessBar>,
            With<MessTile>,
        )>,
    >,
    audio: Res<Audio>,
) {
    for entity in query.iter_mut() {
        commands.entity(entity).despawn();
    }
    audio.stop().fade_out(AudioTween::new(
        Duration::from_secs(1),
        AudioEasing::InOutPowi(2),
    ));
}

fn exit_game_over_menu(
    mut commands: Commands,
    mut query: Query<Entity, (With<Button>, With<GameOver>)>,
) {
    for entity in query.iter_mut() {
        commands.entity(entity).despawn_recursive();
    }
}
