//! IDs and random numbers of the large dataset: every ID is made from the seed and the place of
//! the thing in the dataset, so two runs with one seed make the same IDs (the records come in the
//! same order, and the IDs sort by time like those of the use cases).

use bb_core::id::{IdError, IdSource};
use jiff::Timestamp;

use crate::error::{SampleError, Step};
use crate::random::SeededRandom;

/// A random source for one kind of thing at one place, the same for the same seed.
pub(crate) fn rng_for(seed: u64, kind: u64, at: u64) -> SeededRandom {
    SeededRandom::new(
        seed.wrapping_mul(0x9E37_79B9_7F4A_7C15)
            ^ kind.wrapping_mul(0xC2B2_AE3D_27D4_EB4F)
            ^ at.wrapping_mul(0x1656_67B1_9E37_79F9),
    )
}

fn byte(random: &mut SeededRandom) -> u8 {
    u8::try_from(random.below(256)).unwrap_or(0)
}

/// The ten random bytes of an ID that sorts by time.
pub(crate) fn ten(random: &mut SeededRandom) -> [u8; 10] {
    std::array::from_fn(|_| byte(random))
}

/// The sixteen random bytes of a person's ID.
pub(crate) fn random16(random: &mut SeededRandom) -> [u8; 16] {
    std::array::from_fn(|_| byte(random))
}

/// An ID of a kind that sorts by time, built for the moment `at`.
///
/// # Errors
/// [`SampleError`] when the moment is before 1970.
pub(crate) fn ordered<T>(
    build: impl Fn(Timestamp, [u8; 10]) -> Result<T, IdError>,
    at: Timestamp,
    random: &mut SeededRandom,
) -> Result<T, SampleError> {
    build(at, ten(random)).map_err(|error| SampleError::step(Step::Dataset, &error))
}

/// The source of the random codes of readable issue IDs: the numbers of the dataset's own random
/// source, so the codes are the same for the same seed.
pub(crate) struct Draws<'a>(pub(crate) &'a mut SeededRandom);

impl IdSource for Draws<'_> {
    fn now(&mut self) -> Timestamp {
        Timestamp::UNIX_EPOCH
    }

    fn random(&mut self) -> [u8; 16] {
        random16(self.0)
    }
}
