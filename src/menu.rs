#![cfg_attr(debug_assertions, allow(dead_code, unused_imports))]

use crate::logic::GridState;
use crate::player::Player;
use crate::ui::{MessBar, MessTile};
use crate::GameState;
use crate::{clouds::Cloud, loading::FontAssets};
use bevy::prelude::*;
use bevy::window::close_on_esc;
use bevy_kira_audio::prelude::*;
use bevy_kira_audio::{Audio, AudioEasing, AudioTween};
use iyes_loopless::prelude::*;
#[cfg(debug_assertions)]
const AUTOSTART_TIME_MS: u64 = 1000;
use crate::world::{Platform, Sky, CAMERA_LAYER, DISPLAY_RATIO};
use std::time::Duration;

pub struct MenuPlugin;

/// This plugin is responsible for the game menu (containing only one button...)
/// The menu is only drawn during the State `GameState::Menu` and is removed when that state is exited
impl Plugin for MenuPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<ButtonColors>()
            .add_enter_system(GameState::Menu, setup_menu)
            .add_enter_system(GameState::Menu, click_play_button)
            .add_exit_system(GameState::Menu, cleanup_menu)
            .add_system_set(
                ConditionSet::new()
                    .run_in_state(GameState::Menu)
                    .with_system(click_play_button)
                    .with_system(close_on_esc)
                    .with_system(debug_start_auto)
                    .into(),
            )
            .add_enter_system(GameState::GameOver, setup_game_over_screen)
            .add_enter_system(GameState::GameOver, game_over_clear)
            .add_exit_system(GameState::GameOver, exit_game_over_menu)
            .add_system_set(
                ConditionSet::new()
                    .run_in_state(GameState::GameOver)
                    .with_system(game_over_screen)
                    .into(),
            );
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
                    alignment: Default::default(),
                },
                ..Default::default()
            });
        });
}

#[allow(clippy::type_complexity)]
fn click_play_button(
    button_colors: Res<ButtonColors>,
    mut interaction_query: Query<
        (&Interaction, &mut BackgroundColor),
        (Changed<Interaction>, With<MainMenu>),
    >,
    mut commands: Commands,
) {
    for (interaction, mut color) in &mut interaction_query {
        match *interaction {
            Interaction::Clicked => commands.insert_resource(NextState(GameState::Playing)),
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
fn debug_start_auto(mut commands: Commands, time: Res<Time>) {
    if time.elapsed() > Duration::from_millis(AUTOSTART_TIME_MS) {
        commands.insert_resource(NextState(GameState::Playing));
    };
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
                    alignment: Default::default(),
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
                    alignment: Default::default(),
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
    mut commands: Commands,
    button_colors: Res<ButtonColors>,
) {
    for (interaction, mut color) in &mut interaction_query {
        match *interaction {
            Interaction::Clicked => commands.insert_resource(NextState(GameState::Menu)),
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
) {
    for entity in query.iter_mut() {
        commands.entity(entity).despawn();
    }
}

fn exit_game_over_menu(
    mut commands: Commands,
    mut query: Query<Entity, (With<Button>, With<GameOver>)>,
    audio: Res<Audio>,
) {
    for entity in query.iter_mut() {
        commands.entity(entity).despawn_recursive();
        audio.stop().fade_out(AudioTween::new(
            Duration::from_secs(1),
            AudioEasing::InOutPowi(2),
        ));
    }
}
