#![cfg_attr(debug_assertions, allow(dead_code, unused_imports))]

use std::time::Duration;

use bevy::prelude::*;
use bevy_easings::EaseFunction;
use bevy_splash_screen::{SplashAssetType, SplashItem, SplashPlugin, SplashScreen};

use crate::{world::DISPLAY_RATIO, GameState};

pub struct SplashscreenPlugin;

/// This plugin handles the splashscreen
impl Plugin for SplashscreenPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(
            SplashPlugin::new(GameState::SplashScreen, GameState::Menu)
                .skipable()
                .add_screen(SplashScreen {
                    brands: vec![
                        SplashItem {
                            asset: SplashAssetType::SingleText(
                                Text::from_sections([
                                    TextSection::new(
                                        "Simple Test\n",
                                        TextStyle {
                                            font_size: 40.,
                                            color: Color::WHITE,
                                            ..default()
                                        },
                                    ),
                                    TextSection::new(
                                        "by\n",
                                        TextStyle {
                                            font_size: 24.,
                                            color: Color::WHITE.with_a(0.75),
                                            ..default()
                                        },
                                    ),
                                    TextSection::new(
                                        "Le Calicot",
                                        TextStyle {
                                            font_size: 32.,
                                            color: Color::WHITE,
                                            ..default()
                                        },
                                    ),
                                ])
                                .with_alignment(TextAlignment::Center),
                                "fonts/FiraSans-Bold.ttf".to_string(),
                            ),
                            tint: Color::SEA_GREEN,
                            width: Val::Percent(30.),
                            height: Val::Px(150.),
                            ease_function: EaseFunction::QuarticInOut.into(),
                            duration: Duration::from_secs_f32(2.),
                            is_static: false,
                        },
                        SplashItem {
                            asset: SplashAssetType::SingleImage(
                                "textures/bevy_logo.png".to_string(),
                            ),
                            tint: Color::WHITE,
                            width: Val::Px(1000.),
                            height: Val::Px(250.),
                            ease_function: EaseFunction::QuinticInOut.into(),
                            duration: Duration::from_secs_f32(3.),
                            is_static: true,
                        },
                    ],
                    background_color: BackgroundColor(Color::BLACK),
                    ..default()
                }),
        );
    }
}
