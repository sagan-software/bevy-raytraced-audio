//! Shared GPU result validation compiled against each supported Bevy minor.

use bevy_raytraced_audio::{BandGain, GeometryError};

/// One direct-path gain record decoded from a GPU readback batch.
#[derive(Clone, Copy, Debug)]
pub(crate) struct RawGpuDirectResult {
    /// Ordered emitter index assigned before dispatch.
    pub(crate) emitter_index: u32,
    /// Low, mid, and high amplitude transmission returned by the shader.
    pub(crate) gains: [f32; 3],
}

/// A direct-path result whose emitter index and band gains passed validation.
#[derive(Clone, Copy, Debug)]
pub(crate) struct ValidatedGpuDirectResult {
    /// Ordered emitter index matched to the submitted batch.
    pub(crate) emitter_index: u32,
    /// Validated low, mid, and high amplitude transmission.
    pub(crate) gain: BandGain,
}

/// A malformed GPU result batch that must not update any acoustic response.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum GpuOutputError {
    /// The shader result count does not match the submitted emitter count.
    WrongResultCount {
        /// Expected results in the submitted batch.
        expected: usize,
        /// Results returned by the shader.
        actual: usize,
    },
    /// The submitted batch exceeds the shader's `u32` emitter-index range.
    TooManyEmitters {
        /// Emitter count that could not be represented by the shader ABI.
        count: usize,
    },
    /// A result record does not occupy its submitted emitter index.
    WrongEmitterIndex {
        /// Index expected from the ordered batch.
        expected: u32,
        /// Index returned by the shader.
        actual: u32,
    },
    /// A result contains a non-finite or out-of-range band gain.
    InvalidGain {
        /// Emitter index associated with the invalid gain.
        emitter_index: u32,
        /// Core validation category for the invalid gain.
        source: GeometryError,
    },
}

/// Validates a whole ordered batch before the adapter applies any response.
///
/// The output vector is caller-owned scratch storage. It retains its capacity across batches and
/// is cleared on every error, so a valid prefix cannot escape as a partially accepted batch.
pub(crate) fn validate_gpu_results(
    expected_emitter_count: usize,
    raw_results: &[RawGpuDirectResult],
    accepted_results: &mut Vec<ValidatedGpuDirectResult>,
) -> Result<(), GpuOutputError> {
    accepted_results.clear();

    if u32::try_from(expected_emitter_count).is_err() {
        return Err(GpuOutputError::TooManyEmitters {
            count: expected_emitter_count,
        });
    }

    if raw_results.len() != expected_emitter_count {
        return Err(GpuOutputError::WrongResultCount {
            expected: expected_emitter_count,
            actual: raw_results.len(),
        });
    }

    // The checked count bounds this u32 range to valid shader indices.
    for (expected_index, raw_result) in (0_u32..).zip(raw_results) {
        if raw_result.emitter_index != expected_index {
            accepted_results.clear();
            return Err(GpuOutputError::WrongEmitterIndex {
                expected: expected_index,
                actual: raw_result.emitter_index,
            });
        }

        let gain = match BandGain::try_new(
            raw_result.gains[0],
            raw_result.gains[1],
            raw_result.gains[2],
        ) {
            Ok(gain) => gain,
            Err(source) => {
                accepted_results.clear();
                return Err(GpuOutputError::InvalidGain {
                    emitter_index: expected_index,
                    source,
                });
            }
        };

        accepted_results.push(ValidatedGpuDirectResult {
            emitter_index: expected_index,
            gain,
        });
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{
        GpuOutputError, RawGpuDirectResult, ValidatedGpuDirectResult, validate_gpu_results,
    };
    use bevy_raytraced_audio::{BandGain, GeometryError};

    /// Valid ordered gains become core-validated band values.
    #[test]
    fn accepts_valid_ordered_results() {
        let output = [RawGpuDirectResult {
            emitter_index: 0,
            gains: [0.25, 0.5, 0.75],
        }];
        let mut accepted = Vec::new();

        let result = validate_gpu_results(1, &output, &mut accepted);

        assert_eq!(result, Ok(()));
        let accepted_result = accepted.first().expect("one valid result is accepted");
        assert_eq!(accepted_result.emitter_index, 0);
        assert_eq!(
            accepted_result.gain,
            BandGain::try_new(0.25, 0.5, 0.75).expect("test gains are valid")
        );
    }

    /// A malformed batch cannot leave a partial accepted response prefix.
    #[test]
    fn wrong_output_count_rejects_the_complete_batch() {
        let output = [
            RawGpuDirectResult {
                emitter_index: 0,
                gains: [0.5, 0.6, 0.7],
            },
            RawGpuDirectResult {
                emitter_index: 1,
                gains: [0.8, 0.9, 1.0],
            },
        ];
        let mut accepted = Vec::new();

        let result = validate_gpu_results(1, &output, &mut accepted);

        assert_eq!(
            result,
            Err(GpuOutputError::WrongResultCount {
                expected: 1,
                actual: 2,
            })
        );
        assert!(accepted.is_empty());
    }

    /// A later index mismatch clears a valid prefix before returning an error.
    #[test]
    fn wrong_emitter_index_clears_the_valid_prefix() {
        let output = [
            RawGpuDirectResult {
                emitter_index: 0,
                gains: [0.5, 0.6, 0.7],
            },
            RawGpuDirectResult {
                emitter_index: 3,
                gains: [0.8, 0.9, 1.0],
            },
        ];
        let mut accepted = vec![ValidatedGpuDirectResult {
            emitter_index: 9,
            gain: BandGain::UNITY,
        }];

        let result = validate_gpu_results(2, &output, &mut accepted);

        assert_eq!(
            result,
            Err(GpuOutputError::WrongEmitterIndex {
                expected: 1,
                actual: 3,
            })
        );
        assert!(accepted.is_empty());
    }

    /// A non-finite shader coefficient rejects the entire batch.
    #[test]
    fn non_finite_gain_rejects_the_complete_batch() {
        let output = [RawGpuDirectResult {
            emitter_index: 0,
            gains: [f32::NAN, 0.5, 0.75],
        }];
        let mut accepted = Vec::new();

        let result = validate_gpu_results(1, &output, &mut accepted);

        assert_eq!(
            result,
            Err(GpuOutputError::InvalidGain {
                emitter_index: 0,
                source: GeometryError::NonFiniteValue,
            })
        );
        assert!(accepted.is_empty());
    }

    /// An out-of-range shader coefficient rejects the complete batch.
    #[test]
    fn out_of_range_gain_rejects_the_complete_batch() {
        let output = [RawGpuDirectResult {
            emitter_index: 0,
            gains: [0.5, 1.01, 0.75],
        }];
        let mut accepted = Vec::new();

        let result = validate_gpu_results(1, &output, &mut accepted);

        assert_eq!(
            result,
            Err(GpuOutputError::InvalidGain {
                emitter_index: 0,
                source: GeometryError::CoefficientOutOfRange,
            })
        );
        assert!(accepted.is_empty());
    }

    /// A 32-bit shader index cannot represent a larger emitter batch.
    #[cfg(target_pointer_width = "64")]
    #[test]
    fn emitter_count_outside_shader_index_range_is_rejected() {
        let emitter_count = usize::try_from(u64::from(u32::MAX) + 1)
            .expect("the 64-bit test target represents a count above u32::MAX");
        let mut accepted = Vec::new();

        let result = validate_gpu_results(emitter_count, &[], &mut accepted);

        assert_eq!(
            result,
            Err(GpuOutputError::TooManyEmitters {
                count: emitter_count,
            })
        );
        assert!(accepted.is_empty());
    }
}
