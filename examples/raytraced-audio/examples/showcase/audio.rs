//! Quiet, independently switchable sources and continuous NPC footsteps with no voice churn.

use super::controller::{Leg, Player, View, humanoid};
use super::world::Cutaway;
use bevy::{audio::Volume, prelude::*};
use bevy_raytraced_audio::{MuffleFilter, ReverbEstimate};
use bevy_raytraced_audio_3d::{
    RaytracedAudioEmitter3d, RaytracedAudioListenerTrace3d, RaytracedAudioPlayer,
};

/// Sound groups can be isolated without modifying the acoustic solver.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum Kind {
    /// Intelligible recorded speech.
    Voice,
    /// Upstairs runner.
    Upstairs,
    /// Basement runner.
    Basement,
    /// Running water.
    Stream,
    /// Outdoor rainfall.
    Rain,
    /// Quiet listener footsteps.
    OwnSteps,
    /// Repeatable clap or gunshot for room comparisons.
    Probe,
}

/// Gain ramp state for one source; mute remains effective when processing is bypassed.
#[derive(Component)]
pub(super) struct Sound {
    /// Mixer group.
    pub(super) kind: Kind,
    /// Nominal conservative level before master volume.
    pub(super) level: f32,
    /// Current smoothed volume.
    gain: f32,
}

/// NPC walking phase, shared with continuous footsteps and visible leg movement.
#[derive(Component)]
pub(super) struct Runner {
    /// Floor under their feet.
    pub(super) floor: f32,
    /// Elapsed animation time, frozen by pause.
    pub(super) phase: f32,
}

/// Independent scene controls and quiet category defaults.
#[derive(Resource)]
#[expect(
    clippy::struct_excessive_bools,
    reason = "independent user-facing mixer switches are not mutually exclusive states"
)]
pub(super) struct Mix {
    /// Master gain in the 0..1 range.
    pub(super) master: f32,
    /// Master mute, independent of tracing.
    pub(super) muted: bool,
    /// Enable filter/reverb processing.
    pub(super) processing: bool,
    /// Measured headphone cues, independently switchable for elevation comparisons.
    pub(super) headphones: bool,
    /// Current controlled scenario; zero means free exploration.
    pub(super) scenario: usize,
    /// Rain enabled during exploration.
    pub(super) rain: bool,
    /// Listener's own footsteps enabled.
    pub(super) steps: bool,
    /// Freeze both runners for static comparisons.
    pub(super) paused: bool,
    /// Last listener distance at which a footstep sounded.
    stride: f32,
    /// Alternating recording number.
    step: usize,
}
impl Default for Mix {
    /// Starts well below full scale with rain disabled to make speech comparisons easier.
    fn default() -> Self {
        Self {
            master: 0.55,
            muted: false,
            processing: true,
            headphones: true,
            scenario: 0,
            rain: false,
            steps: true,
            paused: false,
            stride: 0.0,
            step: 0,
        }
    }
}
impl Mix {
    /// Category selection is independent from geometry, ears and processing bypass.
    pub(super) fn enabled(&self, kind: Kind) -> bool {
        if self.muted {
            return false;
        }
        if kind == Kind::Probe {
            return true;
        }
        if kind == Kind::OwnSteps {
            return self.steps && self.scenario == 0;
        }
        if kind == Kind::Rain {
            return self.rain && self.scenario == 0;
        }
        if matches!(kind, Kind::Upstairs | Kind::Basement) && self.paused {
            return false;
        }
        match self.scenario {
            1 | 6 => kind == Kind::Voice,
            2 | 8 => kind == Kind::Upstairs,
            3 | 7 => kind == Kind::Basement,
            4 | 5 => kind == Kind::Stream,
            _ => true,
        }
    }
}

/// Spawns one persistent processed positional source.
fn source(
    commands: &mut Commands<'_, '_>,
    server: &AssetServer,
    kind: Kind,
    path: &str,
    position: Vec3,
    level: f32,
) -> Entity {
    commands
        .spawn((
            Sound {
                kind,
                level,
                gain: 0.0,
            },
            RaytracedAudioEmitter3d,
            RaytracedAudioPlayer::new(server.load(format!("audio/{path}.ogg")))
                .with_reverb_send(0.65)
                .with_binaural(0.22),
            PlaybackSettings::LOOP
                .with_spatial(true)
                .with_volume(Volume::Linear(0.0)),
            Transform::from_translation(position),
            Visibility::default(),
        ))
        .id()
}

/// Adds spoken words, running water, quiet rain and one persistent footstep track per runner.
pub(super) fn setup(
    mut commands: Commands<'_, '_>,
    server: Res<'_, AssetServer>,
    mut meshes: ResMut<'_, Assets<Mesh>>,
    mut materials: ResMut<'_, Assets<StandardMaterial>>,
) {
    source(
        &mut commands,
        &server,
        Kind::Voice,
        "voice",
        Vec3::new(-12.5, 1.5, -2.0),
        0.7,
    );
    let speaker = humanoid(
        &mut commands,
        &mut meshes,
        &mut materials,
        Color::srgb(0.85, 0.44, 0.14),
    );
    commands.entity(speaker).insert((
        Transform::from_xyz(-12.5, 0.0, -2.0)
            .with_rotation(Quat::from_rotation_y(-core::f32::consts::FRAC_PI_2)),
        Cutaway(0.0),
    ));
    source(
        &mut commands,
        &server,
        Kind::Stream,
        "stream_loop",
        Vec3::new(0.0, -0.98, 0.0),
        0.4,
    );
    source(
        &mut commands,
        &server,
        Kind::Rain,
        "rain_loop",
        Vec3::new(-9.0, 8.0, 5.0),
        0.055,
    );
    for (floor, kind, color) in [
        (2.5, Kind::Upstairs, Color::srgb(0.72, 0.24, 0.22)),
        (-2.5, Kind::Basement, Color::srgb(0.35, 0.25, 0.65)),
    ] {
        let root = source(
            &mut commands,
            &server,
            kind,
            "footsteps_walk_loop",
            Vec3::new(10.0, floor + 0.08, -2.0),
            0.13,
        );
        commands
            .entity(root)
            .insert((Runner { floor, phase: 0.0 }, Cutaway(floor)));
        let body = humanoid(&mut commands, &mut meshes, &mut materials, color);
        commands
            .entity(body)
            .insert((Transform::from_xyz(0.0, -0.08, 0.0), ChildOf(root)));
    }
}

/// Back-and-forth runners keep their sounds at foot height, never on the listener's floor.
pub(super) fn runners(
    time: Res<'_, Time<Real>>,
    mix: Res<'_, Mix>,
    mut runners: Query<'_, '_, (&mut Runner, &mut Transform, &Children)>,
    children: Query<'_, '_, &Children>,
    mut legs: Query<'_, '_, (&Leg, &mut Transform), Without<Runner>>,
) {
    for (mut runner, mut transform, body_children) in &mut runners {
        if !mix.paused {
            runner.phase += time.delta_secs().min(0.1);
        }
        let cycle = (runner.phase * 1.65).rem_euclid(8.0);
        let x = if cycle < 4.0 {
            8.0 + cycle
        } else {
            16.0 - cycle
        };
        transform.translation = Vec3::new(x, runner.floor + 0.08, -2.0);
        transform.rotation = Quat::from_rotation_y(if cycle < 4.0 {
            -core::f32::consts::FRAC_PI_2
        } else {
            core::f32::consts::FRAC_PI_2
        });
        for body in body_children {
            if let Ok(parts) = children.get(*body) {
                for part in parts {
                    if let Ok((leg, mut pose)) = legs.get_mut(*part) {
                        pose.rotation = Quat::from_rotation_x(
                            (runner.phase * core::f32::consts::TAU).sin() * leg.side * 0.4,
                        );
                    }
                }
            }
        }
    }
}

/// Emits gentle one-shots only when the listener actually walks, with at most one per frame.
pub(super) fn footsteps(
    mut commands: Commands<'_, '_>,
    server: Res<'_, AssetServer>,
    view: Res<'_, View>,
    mut mix: ResMut<'_, Mix>,
    trace: Res<'_, RaytracedAudioListenerTrace3d>,
    player: Query<'_, '_, &Transform, With<Player>>,
    mut legs: Query<'_, '_, (&Leg, &mut Transform), Without<Player>>,
    bodies: Query<'_, '_, &Children, With<super::controller::Body>>,
) {
    for body in &bodies {
        for child in body {
            if let Ok((leg, mut pose)) = legs.get_mut(*child) {
                pose.rotation = Quat::from_rotation_x(
                    (view.distance * core::f32::consts::PI / 0.85).sin() * leg.side * 0.4,
                );
            }
        }
    }
    if view.distance - mix.stride < 0.85 {
        return;
    }
    mix.stride = view.distance;
    if !mix.enabled(Kind::OwnSteps) {
        return;
    }
    let Ok(player) = player.single() else { return };
    let wood = ((-14.0..-6.0).contains(&player.translation.x)
        || (6.0..14.0).contains(&player.translation.x))
        && (-6.0..2.0).contains(&player.translation.z)
        || ((player.translation.z - super::world::BRIDGE_Z).abs() < 1.0
            && player.translation.x.abs() < 6.0
            && player.translation.y >= 0.0);
    let path = format!(
        "audio/footstep_{}_{}.ogg",
        if wood { "wood" } else { "grass" },
        mix.step % 4 + 1
    );
    mix.step += 1;
    let sound = RaytracedAudioPlayer::new(server.load(path)).with_reverb_send(0.2);
    sound.params().set_filter(MuffleFilter::CLEAR);
    sound.params().set_reverb(
        trace.trace().map_or(
            ReverbEstimate::DRY,
            bevy_raytraced_audio::ListenerTrace3d::reverb,
        ),
        0.2,
    );
    sound.set_processing_enabled(mix.processing);
    // Own feet are centered; they must not become another emitter traced for six seconds of tail.
    commands.spawn((
        sound,
        Sound {
            kind: Kind::OwnSteps,
            level: 0.08,
            gain: 0.08 * mix.master,
        },
        PlaybackSettings::DESPAWN.with_volume(Volume::Linear(0.08 * mix.master)),
    ));
}

/// Smooths category/master gains and moves diffuse exterior sources without teleporting the ears.
pub(super) fn mix_sounds(
    time: Res<'_, Time<Real>>,
    mix: Res<'_, Mix>,
    player: Query<'_, '_, &Transform, (With<Player>, Without<Sound>)>,
    mut sources: Query<
        '_,
        '_,
        (
            &mut Sound,
            &RaytracedAudioPlayer,
            Option<&mut SpatialAudioSink>,
            Option<&mut AudioSink>,
            Option<&mut Transform>,
        ),
    >,
) {
    let rate = 1.0 - (-time.delta_secs().min(0.1) / 0.12).exp();
    let position = player.single().map_or(Vec3::ZERO, |p| p.translation);
    for (mut sound, processor, spatial, plain, pose) in &mut sources {
        processor.set_processing_enabled(mix.processing);
        if let Some(params) = processor.binaural_params() {
            params.set_enabled(mix.headphones);
        }
        let target = if mix.enabled(sound.kind) {
            mix.master * sound.level
        } else {
            0.0
        };
        sound.gain = rate.mul_add(target - sound.gain, sound.gain);
        if (sound.gain - target).abs() < 0.0001 {
            sound.gain = target;
        }
        if let Some(mut sink) = spatial {
            sink.set_volume(Volume::Linear(sound.gain));
        }
        if let Some(mut sink) = plain {
            sink.set_volume(Volume::Linear(sound.gain));
        }
        if let Some(mut pose) = pose {
            let target = match sound.kind {
                Kind::Stream => Some(Vec3::new(0.0, -0.98, position.z.clamp(-16.0, 16.0))),
                Kind::Rain => Some(Vec3::new(position.x, 8.0, position.z + 3.0)),
                _ => None,
            };
            if let Some(target) = target {
                pose.translation = pose.translation.lerp(target, rate);
            }
        }
    }
}

/// Plays the same dry impulse recording at a fixed distance in front of the listener.
pub(super) fn probe(
    mut commands: Commands<'_, '_>,
    server: Res<'_, AssetServer>,
    keys: Res<'_, ButtonInput<KeyCode>>,
    mix: Res<'_, Mix>,
    player: Query<'_, '_, &Transform, With<Player>>,
) {
    let path = if keys.just_pressed(KeyCode::KeyJ) {
        "clap"
    } else if keys.just_pressed(KeyCode::KeyK) {
        "gunshot_1"
    } else {
        return;
    };
    if mix.muted {
        return;
    }
    let Ok(pose) = player.single() else {
        return;
    };
    let position =
        pose.translation + Vec3::Y * super::controller::EYE + pose.rotation * Vec3::NEG_Z * 0.5;
    let sound = RaytracedAudioPlayer::new(server.load(format!("audio/{path}.ogg")))
        .with_binaural(0.22)
        .with_reverb_send(0.85);
    sound.set_processing_enabled(mix.processing);
    if let Some(params) = sound.binaural_params() {
        params.set_enabled(mix.headphones);
    }
    let level = if path == "clap" { 0.22 } else { 0.16 };
    commands.spawn((
        sound,
        RaytracedAudioEmitter3d,
        Sound {
            kind: Kind::Probe,
            level,
            gain: level * mix.master,
        },
        PlaybackSettings::DESPAWN.with_volume(Volume::Linear(level * mix.master)),
        Transform::from_translation(position),
    ));
}
