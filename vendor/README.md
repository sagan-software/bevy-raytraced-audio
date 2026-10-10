# Rodio spatial panning backport

`rodio/` is the published **rodio 0.22.2** crate, including its MIT and Apache
licenses. The spatial correction changes the two distance-difference signs in
`src/source/spatial.rs`, backported from the
[upstream correction](https://github.com/RustAudio/rodio/commit/d6eea94f4ac374efd2f8066c9799066077dc2016).
The original calculation can make a source on the right louder in the left
channel even with correctly positioned ears.

The workspace Cargo patch uses this copy for the Bevy 0.19/0.20 audio backend.
It avoids pulling in unrelated unreleased rodio and CPAL changes. Remove the
patch when upgrading to a compatible released rodio containing the correction.
The sandbox's stereo regression tests render actual rodio samples for both
sides and multiple listener headings, pitches, and source distances.

The village audit also fixes `src/source/channel_volume.rs`: its discarded
`Option::map` result summed input channels instead of averaging them. Stereo
recordings could become twice as loud during positional downmixing. A regression
test in `audio_fixture.rs` checks mono, stereo, and six-channel inputs.

Third-party files are excluded from repository formatting. Keep this copy
otherwise identical to the crates.io package.
