#![cfg_attr(debug_assertions, allow(dead_code, unused_imports))]

use crate::loading::TextureAssets;
use crate::logic::GridState;
use crate::player::Player;
use crate::ui::{MessBar, MessTile};
use crate::GameState;
use crate::{clouds::Cloud, loading::FontAssets};
use bevy::app::AppExit;
use bevy::prelude::*;
use bevy::text::BreakLineOn;
use bevy::window::close_on_esc;
use bevy_easings::Ease;
use bevy_kira_audio::prelude::*;
use bevy_kira_audio::{Audio, AudioEasing, AudioTween};
// use {AlignItems, BackgroundColor, JustifyContent, UiRect};
use crate::world::{Platform, Sky, CAMERA_LAYER, DISPLAY_RATIO};
use std::time::Duration;

#[cfg(debug_assertions)]
const AUTOSTART_TIME_MS: u64 = 1000;
const BACKGROUND_SPEED_S: u64 = 50;
// const BACKGROUND_OFFSET: [f32; 2] = [0., 0.];
const MAX_SHADOW: f32 = 0.6;
const SHADOW_PERIOD: std::time::Duration = Duration::from_secs(30);
const SLIDE_PERIOD: std::time::Duration = Duration::from_secs(20);
const SHADOW_LAYER: f32 = 10.;

// WIP:make the background cycle

#[cfg(debug_assertions)]
#[derive(Resource, Default)]
struct DebugVariables {
    has_playing: bool,
    has_game_over: bool,
}

pub struct MenuPlugin;

#[derive(Component)]
pub struct Background {
    speed: u64,
    x_pos: f32,
    y_pos: f32,
}

impl Default for Background {
    fn default() -> Self {
        Background {
            speed: BACKGROUND_SPEED_S,
            x_pos: 0.,
            y_pos: 0.,
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
            .add_system(click_quit_button.run_if(in_state(GameState::GameOver)))
            .add_system(spawn_background.in_schedule(OnEnter(GameState::Menu)));
        #[cfg(debug_assertions)]
        {
            app.init_resource::<DebugVariables>()
                .add_system(debug_start_auto.run_if(in_state(GameState::Menu)));
            // .add_system(debug_auto_loss.run_if(in_state(GameState::Playing)));

            // /.add_plugin(FrameTimeDiagnosticsPlugin::default())
            // .add_plugin(LogDiagnosticsPlugin::default())
        }
    }
}

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
    asset_server: Res<AssetServer>,
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
    commands
        .spawn(ButtonBundle {
            style: Style {
                size: Size::new(Val::Px(120.0), Val::Px(50.0)),
                margin: UiRect {
                    top: Val::Percent(5.),
                    left: Val::Percent(45.),
                    bottom: Val::Percent(5.),
                    ..default()
                },
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
    commands
        .spawn(SpriteBundle {
            texture: asset_server.load("textures/instructions.drawio.png"),
            transform: Transform::from_xyz(0., -10., 0.).with_scale(Vec3::splat(0.3)),
            ..default()
        })
        .insert(MainMenu);
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

fn cleanup_menu(mut commands: Commands, menu_elt: Query<(Entity,), (With<MainMenu>,)>) {
    for (entity,) in menu_elt.iter() {
        commands.entity(entity).despawn_recursive();
    }
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
fn game_over_clear(mut commands: Commands, audio: Res<Audio>) {
    audio.stop().fade_out(AudioTween::new(
        Duration::from_secs(1),
        AudioEasing::InOutPowi(2),
    ));
}

#[allow(clippy::type_complexity)]
fn exit_game_over_menu(
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
            With<Button>,
            With<GameOver>,
        )>,
    >,
) {
    for entity in query.iter_mut() {
        commands.entity(entity).despawn_recursive();
    }
}

fn spawn_background(
    mut commands: Commands,
    assets: Res<TextureAssets>,
    query: Query<&mut Window>,
    background_image: Res<Assets<Image>>,
) {
    let window = query.single();
    let window_width = window.resolution.width();
    let window_height = window.resolution.height();
    let image = assets.background.clone();
    let background_image = background_image.get(&image).unwrap();
    let image_size = background_image.size();

    let scale_factor = window_height / image_size[1] * DISPLAY_RATIO;

    let offset = Transform::from_translation(Vec3 {
        x: -(image_size[0] * DISPLAY_RATIO - window_width) / 2.,
        y: 0.,
        z: 0.,
    })
    .with_scale(Vec3::splat(scale_factor));

    println!(
        "{} {} {:?} {:?} {:?}",
        { colored::Colorize::blue("➤") },
        { colored::Colorize::blue("AAA:") },
        { window_width },
        { image_size[0] },
        { (image_size[0] - window_width) / 2. }
    );

    commands
        .spawn((
            SpriteBundle {
                texture: image,
                transform: offset,
                ..default()
            },
            // Add the background sliding
            offset.ease_to(
                Transform::from_translation(Vec3::new(
                    (image_size[0] * DISPLAY_RATIO - window_width) / 2.,
                    0.,
                    0.,
                ))
                .with_scale(Vec3::splat(scale_factor)),
                bevy_easings::EaseFunction::SineInOut,
                bevy_easings::EasingType::PingPong {
                    duration: SLIDE_PERIOD,
                    pause: None,
                },
            ),
        ))
        .insert(Background::default());

    commands.spawn((
        SpriteBundle {
            transform: Transform::from_translation(Vec3::new(0., 0., SHADOW_LAYER)),
            ..Default::default()
        },
        Sprite {
            custom_size: Some(Vec2::new(window_width, window_height)),
            color: Color::Rgba {
                red: 0.,
                green: 0.,
                blue: 0.,
                alpha: 0.,
            },
            ..Default::default()
        }
        .ease_to(
            Sprite {
                custom_size: Some(Vec2::new(window_width, window_height)),
                color: Color::Rgba {
                    red: 0.,
                    green: 0.,
                    blue: 0.,
                    alpha: MAX_SHADOW,
                },
                ..Default::default()
            },
            bevy_easings::EaseFunction::SineInOut,
            bevy_easings::EasingType::PingPong {
                duration: SHADOW_PERIOD,
                pause: None,
            },
        ),
    ));
}

// WIP: add black filter on the background darkening slowly the sky
