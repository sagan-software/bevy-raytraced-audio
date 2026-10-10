//! Controlled listening presets, accessible in-scene buttons and live acoustic measurements.

use super::{
    audio::{Kind, Mix, Sound},
    controller::{Player, View},
    world::{Gate, RoomTreatment, Village},
};
use bevy::prelude::*;
use bevy_raytraced_audio_3d::{
    RaytracedAudioDebugDraw3d, RaytracedAudioListenerTrace3d, RaytracedAudioRayResponse3d,
};

/// Collapsible help and preset panel.
#[derive(Component)]
pub(super) struct Panel;

/// Short explanation at the top left.
#[derive(Component)]
pub(super) struct Hud;
/// Live acoustic measurements at the bottom.
#[derive(Component)]
pub(super) struct Metrics;
/// A test preset button.
#[derive(Component)]
pub(super) struct Preset(usize);
/// Crosshair visible only in first person.
#[derive(Component)]
pub(super) struct Crosshair;
/// Controlled comparison labels, with corresponding keyboard numbers.
pub(super) const SCENARIOS: [&str; 9] = [
    "Explore village",
    "Speech / double doors",
    "Footsteps above you",
    "Footsteps below you",
    "Stream / on bridge",
    "Stream / by water",
    "Speech / window",
    "Visit basement",
    "Visit upstairs",
];

/// Builds a compact guide and clickable listening presets.
pub(super) fn setup(mut commands: Commands<'_, '_>) {
    commands
        .spawn((
            Panel,
            Node {
                position_type: PositionType::Absolute,
                left: px(14),
                top: px(12),
                width: px(284),
                padding: UiRect::all(px(12)),
                flex_direction: FlexDirection::Column,
                row_gap: px(5),
                ..default()
            },
            BackgroundColor(Color::srgba(0.055, 0.085, 0.10, 0.91)),
        ))
        .with_children(|panel| {
            panel.spawn((
                Text::new("ACOUSTIC VILLAGE"),
                TextFont {
                    font_size: FontSize::Px(19.0),
                    ..default()
                },
                TextColor(Color::srgb(0.91, 0.82, 0.55)),
            ));
            panel.spawn((
                Hud,
                Text::new("Loading the listening scenarios…"),
                TextFont {
                    font_size: FontSize::Px(13.0),
                    ..default()
                },
            ));
            for (index, label) in SCENARIOS.iter().enumerate() {
                panel
                    .spawn((
                        Button,
                        Preset(index),
                        Node {
                            width: percent(100),
                            padding: UiRect::axes(px(8), px(5)),
                            ..default()
                        },
                        BackgroundColor(Color::srgba(0.18, 0.25, 0.27, 0.95)),
                    ))
                    .with_child((
                        Text::new(format!("{index}  {label}")),
                        TextFont {
                            font_size: FontSize::Px(13.0),
                            ..default()
                        },
                    ));
            }
        });
    commands.spawn((
        Metrics,
        Text::new(""),
        TextFont {
            font_size: FontSize::Px(14.0),
            ..default()
        },
        Node {
            position_type: PositionType::Absolute,
            left: px(14),
            bottom: px(12),
            max_width: percent(95),
            padding: UiRect::all(px(10)),
            ..default()
        },
        BackgroundColor(Color::srgba(0.055, 0.085, 0.10, 0.92)),
    ));
    commands.spawn((
        Crosshair,
        Text::new("+"),
        TextFont {
            font_size: FontSize::Px(23.0),
            ..default()
        },
        Node {
            position_type: PositionType::Absolute,
            left: percent(50),
            top: percent(49),
            ..default()
        },
        Visibility::Hidden,
    ));
}

/// Places the listener at a repeatable comparison point and isolates that sound category.
pub(super) fn select(index: usize, mix: &mut Mix, view: &mut View, player: &mut Transform) {
    let (position, yaw) = match index {
        1 => (Vec3::new(-8.0, 0.0, -2.0), core::f32::consts::FRAC_PI_2),
        2 | 3 => (Vec3::new(10.0, 0.0, -1.0), 0.0),
        4 => (
            Vec3::new(-0.8, 1.2, super::world::BRIDGE_Z),
            -core::f32::consts::FRAC_PI_2,
        ),
        5 => (
            Vec3::new(0.8, -1.1, super::world::BRIDGE_Z + 3.0),
            core::f32::consts::FRAC_PI_2,
        ),
        6 => (Vec3::new(-15.2, 0.0, -3.0), -core::f32::consts::FRAC_PI_2),
        7 => (Vec3::new(10.0, -2.5, 0.5), 0.0),
        8 => (Vec3::new(10.0, 2.5, 0.5), 0.0),
        _ => (Vec3::new(-9.0, 0.0, 5.0), 0.0),
    };
    mix.scenario = index;
    view.yaw = yaw;
    view.aim_active = false;
    view.pitch = 0.0;
    player.translation = position;
    player.rotation = Quat::from_rotation_y(yaw);
}

/// Keyboard and button controls share the same preset path.
pub(super) fn controls(
    keys: Res<'_, ButtonInput<KeyCode>>,
    mut buttons: Query<'_, '_, (&Interaction, &Preset, &mut BackgroundColor), Changed<Interaction>>,
    mut mix: ResMut<'_, Mix>,
    mut treatment: ResMut<'_, RoomTreatment>,
    mut view: ResMut<'_, View>,
    mut player: Query<'_, '_, &mut Transform, With<Player>>,
    mut gates: Query<'_, '_, &mut Gate>,
    mut rays: ResMut<'_, RaytracedAudioDebugDraw3d>,
    mut panels: Query<'_, '_, &mut Visibility, With<Panel>>,
) {
    if keys.just_pressed(KeyCode::KeyH) {
        for mut panel in &mut panels {
            *panel = if *panel == Visibility::Hidden {
                Visibility::Inherited
            } else {
                Visibility::Hidden
            };
        }
    }
    let mut chosen = None;
    for (index, key) in [
        KeyCode::Digit0,
        KeyCode::Digit1,
        KeyCode::Digit2,
        KeyCode::Digit3,
        KeyCode::Digit4,
        KeyCode::Digit5,
        KeyCode::Digit6,
        KeyCode::Digit7,
        KeyCode::Digit8,
    ]
    .into_iter()
    .enumerate()
    {
        if keys.just_pressed(key) {
            chosen = Some(index);
        }
    }
    for (interaction, preset, mut color) in &mut buttons {
        color.0 = if *interaction == Interaction::None {
            Color::srgba(0.18, 0.25, 0.27, 0.95)
        } else {
            Color::srgb(0.30, 0.41, 0.42)
        };
        if *interaction == Interaction::Pressed {
            chosen = Some(preset.0);
        }
    }
    if let Some(index) = chosen
        && let Ok(mut player) = player.single_mut()
    {
        select(index, &mut mix, &mut view, &mut player);
    }
    if keys.just_pressed(KeyCode::KeyB) {
        mix.headphones = !mix.headphones;
    }
    if keys.just_pressed(KeyCode::KeyT) {
        treatment.mode = (treatment.mode + 1) % 3;
    }
    if keys.just_pressed(KeyCode::Tab) {
        mix.processing = !mix.processing;
    }
    if keys.just_pressed(KeyCode::KeyM) {
        mix.muted = !mix.muted;
    }
    if keys.just_pressed(KeyCode::KeyN) {
        mix.rain = !mix.rain;
    }
    if keys.just_pressed(KeyCode::KeyG) {
        mix.steps = !mix.steps;
    }
    if keys.just_pressed(KeyCode::KeyP) {
        mix.paused = !mix.paused;
    }
    if keys.just_pressed(KeyCode::KeyV) {
        rays.enabled = !rays.enabled;
    }
    if keys.just_pressed(KeyCode::Minus) {
        mix.master = (mix.master - 0.1).max(0.0);
    }
    if keys.just_pressed(KeyCode::Equal) {
        mix.master = (mix.master + 0.1).min(1.0);
    }
    if keys.just_pressed(KeyCode::KeyO) || keys.just_pressed(KeyCode::KeyC) {
        for mut gate in &mut gates {
            gate.open = keys.just_pressed(KeyCode::KeyO);
        }
    }
}

/// Shows what is audible, which ears define left/right, and measured filter/reverb values.
pub(super) fn hud(
    mix: Res<'_, Mix>,
    view: Res<'_, View>,
    village: Res<'_, Village>,
    treatment: Res<'_, RoomTreatment>,
    trace: Res<'_, RaytracedAudioListenerTrace3d>,
    player: Query<'_, '_, &Transform, With<Player>>,
    sources: Query<'_, '_, (&Sound, &RaytracedAudioRayResponse3d, &Transform), Without<Player>>,
    mut texts: Query<'_, '_, &mut Text, With<Hud>>,
    mut metrics: Query<'_, '_, &mut Text, (With<Metrics>, Without<Hud>)>,
    mut crosshair: Query<'_, '_, &mut Visibility, With<Crosshair>>,
) {
    let Ok(player) = player.single() else { return };
    if let Ok(mut text) = texts.single_mut() {
        **text = format!(
            "{}  |  {}\nVolume {:.0}% {}  |  processing {}\nWASD walk | mouse face / look\nF first person | Esc overhead\nQ/R orbit / turn | wheel zoom\nE use | O open all | C close all\nX remove / rebuild nearby wall\nTab compare bypass | V rays\nN rain {} | G own steps {}\nP pause runners | M mute | -/+ level\nB headphones {} | T room treatment\n{}\nJ clap | K gunshot | H guide",
            if view.first_person {
                "FIRST PERSON"
            } else {
                "THIRD PERSON"
            },
            SCENARIOS
                .get(mix.scenario)
                .copied()
                .unwrap_or("Explore village"),
            mix.master * 100.0,
            if mix.muted { "MUTED" } else { "" },
            if mix.processing { "ON" } else { "BYPASSED" },
            if mix.rain { "on" } else { "off" },
            if mix.steps { "on" } else { "off" },
            if mix.headphones { "ON" } else { "OFF" },
            treatment.label()
        );
    }
    let focused = match mix.scenario {
        2 | 8 => Kind::Upstairs,
        3 | 7 => Kind::Basement,
        4 | 5 => Kind::Stream,
        _ => Kind::Voice,
    };
    let reading = sources
        .iter()
        .find(|(sound, _, _)| sound.kind == focused)
        .map_or_else(
            || "Loading source…".to_owned(),
            |(_, response, source)| {
                let local = player.rotation.inverse()
                    * (source.translation - player.translation - Vec3::Y * super::controller::EYE);
                let bearing = if local.x > 0.2 {
                    "RIGHT ear"
                } else if local.x < -0.2 {
                    "LEFT ear"
                } else {
                    "CENTER"
                };
                let filter = response.response().filter();
                format!(
                    "Source: {bearing} / {} | {} | bass {:.0}% / treble {:.0}%",
                    if local.y > 0.3 {
                        "ABOVE"
                    } else if local.y < -0.3 {
                        "BELOW"
                    } else {
                        "LEVEL"
                    },
                    if response.response().is_direct_visible() {
                        "direct path"
                    } else {
                        "blocked / indirect"
                    },
                    filter.gain_lf() * 100.0,
                    filter.gain_hf() * 100.0
                )
            },
        );
    let reverb = trace.trace().map_or(
        bevy_raytraced_audio::ReverbEstimate::DRY,
        bevy_raytraced_audio::ListenerTrace3d::reverb,
    );
    if let Ok(mut text) = metrics.single_mut() {
        **text = format!(
            "{}\n{reading}\nRoom: bass {:.2}s / treble {:.2}s | tail {:.0}% | escape {:.0}% | feet {:.2} m\nHeadphones recommended. B compares elevation cues; Tab compares room processing.",
            village.prompt,
            reverb.decay_low_s(),
            reverb.decay_high_s(),
            reverb.wet_gain() * 100.0,
            reverb.outdoor_fraction() * 100.0,
            player.translation.y
        );
    }
    for mut visibility in &mut crosshair {
        *visibility = if view.first_person {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
    }
}
