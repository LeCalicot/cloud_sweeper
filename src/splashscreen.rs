#![cfg_attr(debug_assertions, allow(dead_code, unused_imports))]

use std::time::Duration;

use bevy::prelude::*;
use bevy_easings::EaseFunction;
use bevy_splash_screen::{SplashAssetType, SplashItem, SplashPlugin, SplashScreen, SplashType};

use crate::{world::DISPLAY_RATIO, GameState};

pub struct SplashscreenPlugin;

/// This plugin handles the splashscreen
impl Plugin for SplashscreenPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(
            SplashPlugin::new(GameState::SplashScreen, GameState::Loading)
                .skipable()
                .add_screen(SplashScreen {
                    brands: vec![
                        SplashItem {
                            asset: SplashAssetType::SingleText(
                                Text::from_sections([
                                    TextSection::new(
                                        "Created by\n",
                                        TextStyle {
                                            font_size: 40.,
                                            color: Color::GOLD,
                                            ..default()
                                        },
                                    ),
                                    TextSection::new(
                                        "Le Calicot",
                                        TextStyle {
                                            font_size: 60.,
                                            color: Color::GOLD,
                                            ..default()
                                        },
                                    ),
                                ])
                                .with_alignment(TextAlignment::Center),
                                "fonts/FiraSans-Bold.ttf".to_string(),
                            ),
                            tint: Color::GOLD,
                            width: Val::Percent(30.),
                            height: Val::Px(150.),
                            ease_function: EaseFunction::QuarticInOut.into(),
                            duration: Duration::from_secs_f32(4.),
                            is_static: false,
                        },
                        SplashItem {
                            asset: SplashAssetType::SingleText(
                                Text::from_sections([
                                    TextSection::new(
                                        "Pixel Art by\n",
                                        TextStyle {
                                            font_size: 40.,
                                            color: Color::GOLD,
                                            ..default()
                                        },
                                    ),
                                    TextSection::new(
                                        "Dezilim",
                                        TextStyle {
                                            font_size: 60.,
                                            color: Color::GOLD,
                                            ..default()
                                        },
                                    ),
                                ])
                                .with_alignment(TextAlignment::Center),
                                "fonts/FiraSans-Bold.ttf".to_string(),
                            ),
                            tint: Color::GOLD,
                            width: Val::Percent(30.),
                            height: Val::Px(150.),
                            ease_function: EaseFunction::QuarticInOut.into(),
                            duration: Duration::from_secs_f32(4.5),
                            is_static: false,
                        },
                        SplashItem {
                            asset: SplashAssetType::SingleText(
                                Text::from_sections([
                                    TextSection::new(
                                        "Music by\n",
                                        TextStyle {
                                            font_size: 40.,
                                            color: Color::GOLD,
                                            ..default()
                                        },
                                    ),
                                    TextSection::new(
                                        "Hstick",
                                        TextStyle {
                                            font_size: 60.,
                                            color: Color::GOLD,
                                            ..default()
                                        },
                                    ),
                                ])
                                .with_alignment(TextAlignment::Center),
                                "fonts/FiraSans-Bold.ttf".to_string(),
                            ),
                            tint: Color::GOLD,
                            width: Val::Percent(30.),
                            height: Val::Px(150.),
                            ease_function: EaseFunction::QuarticInOut.into(),
                            duration: Duration::from_secs_f32(5.),
                            is_static: false,
                        },
                        SplashItem {
                            asset: SplashAssetType::SingleText(
                                Text::from_sections([TextSection::new(
                                    "Created with",
                                    TextStyle {
                                        font_size: 40.,
                                        color: Color::SILVER,
                                        ..default()
                                    },
                                )])
                                .with_alignment(TextAlignment::Center),
                                "fonts/FiraSans-Bold.ttf".to_string(),
                            ),
                            tint: Color::SILVER,
                            width: Val::Percent(30.),
                            height: Val::Px(150.),
                            ease_function: EaseFunction::QuarticInOut.into(),
                            duration: Duration::from_secs_f32(5.5),
                            is_static: false,
                        },
                        SplashItem {
                            asset: SplashAssetType::SingleImage(
                                "textures/bevy_logo.png".to_string(),
                            ),
                            tint: Color::WHITE,
                            width: Val::Px(500.),
                            height: Val::Px(125.),
                            ease_function: EaseFunction::QuinticInOut.into(),
                            duration: Duration::from_secs_f32(5.5),
                            is_static: true,
                        },
                    ],
                    splash_type: SplashType::Grid,
                    background_color: BackgroundColor(Color::BLACK),
                    ..default()
                })
                .add_screen(SplashScreen {
                    brands: vec![SplashItem {
                        asset: SplashAssetType::SingleText(
                            Text::from_sections([TextSection::new(
                                "Cloud Sweeper\n",
                                TextStyle {
                                    font_size: 200.,
                                    color: Color::GOLD,
                                    ..default()
                                },
                            )])
                            .with_alignment(TextAlignment::Center),
                            "fonts/FiraSans-Bold.ttf".to_string(),
                        ),
                        tint: Color::SALMON,
                        width: Val::Percent(40.),
                        height: Val::Percent(50.),
                        ease_function: EaseFunction::QuarticInOut.into(),
                        duration: Duration::from_secs_f32(4.),
                        is_static: false,
                    }],
                    splash_type: SplashType::Grid,
                    background_color: BackgroundColor(Color::BLACK),
                    ..default()
                }),
        );
    }
}
