//! Streaming checks independent of rodio versions and audio devices.

use super::{ProcessedSamples, RaytracedAudioSource};
use bevy::audio::AudioSource;
use bevy_raytraced_audio::{AcousticDspParams, MuffleFilter, ReverbEstimate};
use std::sync::Arc;

/// Encoded PCM fixture exercises the actual decoder without an output device.
fn encoded_asset(looping: bool, binaural: bool) -> RaytracedAudioSource {
    let mut source = asset(looping);
    source.source.bytes =
        Arc::from(include_bytes!("../../assets/audio/bevy-raytraced-audio-chime.wav").as_slice());
    if binaural {
        source.binaural = Some(Arc::new(bevy_raytraced_audio::BinauralParams::new(1.0)));
    }
    source
}

/// Decoder wrappers preserve stream metadata and continue across internal loop boundaries.
#[test]
fn encoded_stream_metadata_and_looping() {
    use bevy::audio::{Decodable, Source};
    for binaural in [false, true] {
        let source = encoded_asset(true, binaural);
        let original = source.source.decoder();
        let count = original.count();
        let mut decoder = source.decoder();
        assert_eq!(
            decoder.sample_rate().to_string(),
            source.source.decoder().sample_rate().to_string()
        );
        assert_eq!(
            decoder.channels().to_string(),
            if binaural {
                "2".to_owned()
            } else {
                source.source.decoder().channels().to_string()
            }
        );
        assert_eq!(decoder.total_duration(), None);
        assert!(format!("{decoder:?}").contains("ProcessedDecoder"));
        for _ in 0..count * 2 + 2 {
            assert!(decoder.next().expect("loop must restart").is_finite());
        }
    }
}

/// Both dimensional plugins register the processing asset once when an asset server exists.
#[test]
fn dimensional_plugins_share_processing_registration() {
    use super::RaytracedAudioProcessingPlugin;
    use crate::audio_2d::RaytracedAudio2dPlugin;
    use crate::audio_3d::RaytracedAudio3dPlugin;
    use bevy::app::TaskPoolPlugin;
    use bevy::asset::{AssetApp, AssetPlugin, Assets};
    use bevy::prelude::App;
    use bevy_raytraced_audio::{AudioBackendPreference, RayTraceSettings};
    let mut app = App::new();
    app.add_plugins((TaskPoolPlugin::default(), AssetPlugin::default()));
    app.init_asset::<AudioSource>();
    app.add_plugins((
        RaytracedAudio2dPlugin::default()
            .with_backend_preference(AudioBackendPreference::Auto)
            .with_ray_tracing(RayTraceSettings::default())
            .without_ray_tracing(),
        RaytracedAudio3dPlugin::default()
            .with_backend_preference(AudioBackendPreference::Auto)
            .with_ray_tracing(RayTraceSettings::default())
            .without_ray_tracing(),
    ));
    assert!(app.is_plugin_added::<RaytracedAudioProcessingPlugin>());
    assert!(
        app.world()
            .contains_resource::<Assets<RaytracedAudioSource>>()
    );
}

/// Loaded, pending, looping and binaural players prepare independently, exactly once.
#[test]
fn preparation_preserves_live_parameters_and_waits_for_assets() {
    use super::{RaytracedAudioPlayer, RaytracedAudioPrepared, prepare_raytraced_players};
    use bevy::asset::{Assets, Handle};
    use bevy::audio::{AudioPlayer, PlaybackMode, PlaybackSettings};
    use bevy::prelude::{App, Update};
    let mut app = App::new();
    app.init_resource::<Assets<AudioSource>>()
        .init_resource::<Assets<RaytracedAudioSource>>()
        .add_systems(Update, prepare_raytraced_players);
    let handle = app
        .world_mut()
        .resource_mut::<Assets<AudioSource>>()
        .add(encoded_asset(false, false).source);
    for send in [f32::NAN, -1.0, 0.0, 0.5, 3.0] {
        let player = RaytracedAudioPlayer::new(handle.clone()).with_reverb_send(send);
        assert_eq!(player.source(), &handle);
        assert_eq!(
            player.reverb_send(),
            if send.is_finite() {
                send.clamp(0., 2.)
            } else {
                1.
            }
        );
        player.set_processing_enabled(false);
        assert!(!player.params().is_enabled());
    }
    let pending = app
        .world_mut()
        .spawn(RaytracedAudioPlayer::new(Handle::default()))
        .id();
    let normal = app
        .world_mut()
        .spawn(RaytracedAudioPlayer::new(handle.clone()))
        .id();
    let looped = app
        .world_mut()
        .spawn((
            RaytracedAudioPlayer::new(handle.clone()).with_binaural(1.0),
            PlaybackSettings::LOOP.with_spatial(true),
        ))
        .id();
    let binaural = app
        .world_mut()
        .spawn(RaytracedAudioPlayer::new(handle).with_binaural(1.0))
        .id();
    app.update();
    assert!(app.world().get::<RaytracedAudioPrepared>(pending).is_none());
    for (entity, looping, stereo) in [
        (normal, false, false),
        (looped, true, true),
        (binaural, false, true),
    ] {
        let player = app
            .world()
            .get::<AudioPlayer<RaytracedAudioSource>>(entity)
            .unwrap();
        let processed = app
            .world()
            .resource::<Assets<RaytracedAudioSource>>()
            .get(&player.0)
            .unwrap();
        assert_eq!(processed.looping, looping);
        assert_eq!(processed.binaural.is_some(), stereo);
        assert!(Arc::ptr_eq(
            &processed.params,
            app.world()
                .get::<RaytracedAudioPlayer>(entity)
                .unwrap()
                .params()
        ));
        if stereo {
            let settings = app.world().get::<PlaybackSettings>(entity).unwrap();
            assert!(!settings.spatial);
            assert!(matches!(settings.mode, PlaybackMode::Once));
        }
    }
    app.update();
    assert_eq!(
        app.world().resource::<Assets<RaytracedAudioSource>>().len(),
        3
    );
}

/// A two-sample impulse, also used as the restart fixture.
fn impulse(_: &AudioSource) -> core::array::IntoIter<f32, 2> {
    [1.0, 0.0].into_iter()
}

/// Builds a stream fixture with no encoded audio or device dependency.
fn asset(looping: bool) -> RaytracedAudioSource {
    RaytracedAudioSource {
        source: AudioSource {
            bytes: Arc::from([]),
        },
        params: Arc::new(AcousticDspParams::default()),
        looping,
        binaural: None,
    }
}

/// An impulse must survive the silent pre-delay at every supported stream layout.
#[test]
fn delayed_echo_is_not_cut_off_by_initial_silence() {
    for sample_rate in [8_000, 44_100, 48_000, 192_000] {
        for channels in [1, 2] {
            let asset = asset(false);
            asset
                .params
                .set_reverb(ReverbEstimate::from_parameters(1.0, 1.5, 0.3), 1.0);
            let mut samples = ProcessedSamples::new(
                impulse(&asset.source),
                impulse,
                &asset,
                channels,
                sample_rate,
            );
            let mut count = 0;
            let mut tail_energy = 0.0;
            while let Some(sample) = samples.next_sample() {
                if count > 2 {
                    tail_energy = sample.mul_add(sample, tail_energy);
                }
                count += 1;
            }
            assert!(
                tail_energy > 1.0e-4,
                "{sample_rate} Hz, {channels} channels: {tail_energy}"
            );
            assert_eq!(count % usize::from(channels), 0);
            assert!(count <= 2 + 6 * usize::try_from(sample_rate).unwrap() * usize::from(channels));
            assert!(samples.next_sample().is_none());
        }
    }
}

/// Internally looped audio reads live parameters after the first repetition.
#[test]
fn looping_audio_keeps_responding_to_filter_changes() {
    let asset = asset(true);
    let mut samples = ProcessedSamples::new(impulse(&asset.source), impulse, &asset, 1, 8_000);
    assert!(samples.next_sample().unwrap() > 0.9);
    asset.params.set_filter(MuffleFilter::SILENT);
    for _ in 0..8_000 {
        samples.next_sample().expect("loop continues");
    }
    for _ in 0..100 {
        assert!(samples.next_sample().unwrap().abs() < 1.0e-4);
    }
}
/// Mono and stereo assets both yield stereo HRTF output; the tail drains on frame boundaries.
#[test]
fn binaural_stream_downmixes_and_preserves_stereo_tail() {
    use bevy_raytraced_audio::BinauralParams;
    for channels in [1, 2] {
        let mut asset = asset(false);
        let params = Arc::new(BinauralParams::new(1.0));
        params.set_position([1.0, 0.0, 0.0]);
        asset.binaural = Some(params);
        let mut samples =
            ProcessedSamples::new(impulse(&asset.source), impulse, &asset, channels, 44_100);
        let mut ears = [0.0; 2];
        let mut count = 0;
        while let Some(left) = samples.next_sample() {
            let right = samples.next_sample().expect("complete stereo frame");
            ears[0] = left.mul_add(left, ears[0]);
            ears[1] = right.mul_add(right, ears[1]);
            count += 2;
            assert!(count <= 44_100 * 12 + 4);
        }
        assert!(ears[1] > ears[0] * 2.0, "{ears:?}");
        assert!(ears[0] > 0.001);
        assert!(samples.next_sample().is_none());
    }
}
