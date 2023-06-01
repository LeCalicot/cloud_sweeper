#![cfg_attr(debug_assertions, allow(dead_code, unused_imports))]

use crate::loading::TextureAssets;
use crate::logic::{GridState, LossCause};
use crate::player::{Player, TILE_SIZE};
use crate::ui::{MessBar, MessTile};
use crate::GameState;
use crate::{clouds::Cloud, loading::FontAssets};
use bevy::app::AppExit;
use bevy::prelude::*;
use bevy::text::BreakLineOn;
use bevy::window::close_on_esc;
use bevy_easings::{Ease, EasingType};
use bevy_ecs_tilemap::prelude::TilemapTextureSize;
use bevy_ecs_tilemap::tiles::{TileBundle, TilePos, TileVisible};
use bevy_kira_audio::prelude::*;
use bevy_kira_audio::{Audio, AudioEasing, AudioTween};
// use {AlignItems, BackgroundColor, JustifyContent, UiRect};
use crate::world::{Platform, Sky, CAMERA_LAYER, DISPLAY_RATIO, LEVEL_SIZE};
use std::time::Duration;

#[cfg(debug_assertions)]
const AUTOSTART_TIME_MS: u64 = 1000;
const BACKGROUND_SPEED_S: u64 = 50;
// const BACKGROUND_OFFSET: [f32; 2] = [0., 0.];
const MAX_SHADOW: f32 = 0.6;
const SHADOW_PERIOD: std::time::Duration = Duration::from_secs(30);
const SLIDE_PERIOD: std::time::Duration = Duration::from_secs(40);
const SHADOW_LAYER: f32 = 10.;

const GAMEOVER_EASING: bevy_easings::EaseFunction = bevy_easings::EaseFunction::SineInOut;
const GAMEOVER_EASING_SCALE_FACTOR: f32 = 2.;
const GAMEOVER_EASING_ROT: bevy_easings::EaseFunction = bevy_easings::EaseFunction::SineInOut;
const GAMEOVER_EASING_ROT_ANGLE: f32 = 10. * std::f32::consts::PI / 180.;
const GAMEOVER_EASING_DURATION: std::time::Duration = Duration::from_millis(500);
pub const GAMEOVER_MESS_BLINK_DURATION: f32 = 0.5;

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
#[derive(Component)]
pub struct Retry;
/// This plugin is responsible for the game menu (containing only one button...)
#[derive(Component)]
pub struct BackgroundTag;
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
            .add_system(highlight_mess_loss_condition.run_if(in_state(GameState::GameOver)))
            .add_system(highlight_cloud_lose_condition.in_schedule(OnEnter(GameState::GameOver)))
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
        .insert(GameOver)
        .insert(Retry);

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

// WIP:
//// - how to set the size for the mess bar tiles?
//// - change the easing for the size
//// - use the rotation easing as well & make it rotate around the center of the entity
// - create a single system for all the buttons to change color when hovered
// - Make sure that the previous easing (for cloud move) is finished
// - let the move_cloud system finish (just don't update the grid!)
// - Add a pause at the beginning of GameOver state
// - remove background when restarting (now there are 2 entities)
// - In the gameover menu, quit=return to main menu, retry=replay instantly

fn highlight_cloud_lose_condition(
    mut commands: Commands,
    mut query: Query<(&mut Sprite, &mut Transform, Entity), (With<LossCause>,)>,
    mut tile_query: Query<(&TilePos, &mut MessTile)>,
) {
    for (sprite, transfo, entity) in query.iter_mut() {
        let mut orig_sprite = sprite.clone();
        orig_sprite.custom_size = Some(Vec2::new(TILE_SIZE, TILE_SIZE));
        let mut bigger_sprite = sprite.clone();
        bigger_sprite.custom_size =
            Some(Vec2::new(TILE_SIZE, TILE_SIZE) * GAMEOVER_EASING_SCALE_FACTOR);
        commands.entity(entity).insert(orig_sprite.ease_to(
            bigger_sprite,
            GAMEOVER_EASING,
            EasingType::PingPong {
                duration: GAMEOVER_EASING_DURATION,
                pause: None,
            },
        ));

        let mut new_transfo_1 = *transfo;
        let mut new_transfo_2 = *transfo;
        new_transfo_1.rotate_local_z(-GAMEOVER_EASING_ROT_ANGLE);
        new_transfo_2.rotate_local_z(GAMEOVER_EASING_ROT_ANGLE);
        commands.entity(entity).insert(new_transfo_1.ease_to(
            new_transfo_2,
            GAMEOVER_EASING_ROT,
            EasingType::PingPong {
                duration: GAMEOVER_EASING_DURATION,
                pause: Some(GAMEOVER_EASING_DURATION / 2),
            },
        ));
    }

    // Add an offset to the timer to make it sliding
    for (pos, mut tile) in tile_query.iter_mut() {
        tile.blink_loss.set_elapsed(Duration::from_secs_f32(
            ((LEVEL_SIZE - pos.y - 1) as f32 * GAMEOVER_MESS_BLINK_DURATION / 10.)
                % GAMEOVER_MESS_BLINK_DURATION,
        ))
    }
}

fn highlight_mess_loss_condition(
    mut tile_query: Query<(&mut MessTile, &mut TileVisible), With<LossCause>>,
    time: Res<Time>,
) {
    for (mut tile, mut vis) in tile_query.iter_mut() {
        tile.blink_loss.tick(time.delta());
        if tile.blink_loss.just_finished() {
            // Switch the visibility:
            vis.0 = !vis.0;
        }
    }
}

#[allow(clippy::type_complexity)]
fn game_over_screen(
    mut interaction_query: Query<
        (&Interaction, &mut BackgroundColor),
        (Changed<Interaction>, With<Button>, With<Retry>),
    >,
    mut next_state: ResMut<NextState<GameState>>,
    button_colors: Res<ButtonColors>,
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
    mut background_query: Query<Entity, With<BackgroundTag>>,
) {
    // Remove the previous background if coming from the game:
    for entity in background_query.iter_mut() {
        commands.entity(entity).despawn();
    }
    let window = query.single();
    let window_width = window.resolution.width();
    let window_height = window.resolution.height();
    let image = assets.background.clone();
    let background_image = background_image.get(&image).unwrap();
    let image_size = background_image.size();

    let scale_factor = window_height / image_size[1];
    // let scale_factor = window_height / image_size[1];

    let offset = Transform::from_translation(Vec3 {
        x: -(image_size[0] * scale_factor - window_width) * scale_factor / 2. * DISPLAY_RATIO,
        // x: 0.,
        y: 0.,
        z: 0.,
    })
    .with_scale(Vec3::splat(scale_factor * DISPLAY_RATIO));

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
                    (image_size[0] * scale_factor - window_width) * scale_factor / 2.
                        * DISPLAY_RATIO,
                    0.,
                    0.,
                ))
                .with_scale(Vec3::splat(scale_factor * DISPLAY_RATIO)),
                bevy_easings::EaseFunction::SineInOut,
                bevy_easings::EasingType::PingPong {
                    duration: SLIDE_PERIOD,
                    pause: None,
                },
            ),
        ))
        .insert(Background::default())
        .insert(BackgroundTag);

    commands
        .spawn((
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
        ))
        .insert(BackgroundTag);
}
