//! The generator's own random source: the same numbers for the same seed on every machine, which
//! the system's random numbers (and a library whose output may change between versions) cannot
//! promise. `SplitMix64` is small, passes the usual statistical tests and needs no state but one
//! number. It makes names and roles, never a secret: passwords come from the operating system.

/// A deterministic stream of numbers, made from a seed.
#[derive(Clone, Debug)]
pub(crate) struct SeededRandom(u64);

impl SeededRandom {
    pub(crate) const fn new(seed: u64) -> Self {
        Self(seed)
    }

    fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut mixed = self.0;
        mixed = (mixed ^ (mixed >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        mixed = (mixed ^ (mixed >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        mixed ^ (mixed >> 31)
    }

    /// A number from `0` up to but not including `bound`; `0` when `bound` is `0`.
    pub(crate) fn below(&mut self, bound: usize) -> usize {
        let wide = u128::from(self.next_u64()) * u128::try_from(bound).unwrap_or(0);
        usize::try_from(wide >> 64).unwrap_or(0)
    }

    /// A number from `0` up to but not including `bound`; `0` when `bound` is not above `0`.
    pub(crate) fn below_i64(&mut self, bound: i64) -> i64 {
        i64::try_from(self.below(usize::try_from(bound).unwrap_or(0))).unwrap_or(0)
    }

    /// Whether a chance of `percent` out of a hundred came up.
    pub(crate) fn chance(&mut self, percent: u32) -> bool {
        self.below(100) < usize::try_from(percent).unwrap_or(0)
    }

    /// A number from `0` up to but not including `bound` that is mostly small: the cube of an
    /// even draw, so most delays are short and a few are long.
    pub(crate) fn skewed(&mut self, bound: i64) -> i64 {
        let part = i128::try_from(self.below(1000)).unwrap_or(0);
        let cube = part * part * part;
        i64::try_from(i128::from(bound.max(0)) * cube / 1_000_000_000).unwrap_or(0)
    }

    /// One of `items`; `None` when there are none.
    pub(crate) fn pick<'a, T>(&mut self, items: &'a [T]) -> Option<&'a T> {
        items.get(self.below(items.len()))
    }

    /// Puts `items` in a new order (Fisher-Yates).
    pub(crate) fn shuffle<T>(&mut self, items: &mut [T]) {
        for last in (1..items.len()).rev() {
            let other = self.below(last + 1);
            items.swap(last, other);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_same_seed_gives_the_same_numbers_and_another_seed_another_ones() {
        let numbers = |seed| {
            let mut random = SeededRandom::new(seed);
            (0..8).map(|_| random.below(1000)).collect::<Vec<_>>()
        };
        assert_eq!(numbers(42), numbers(42));
        assert_ne!(numbers(42), numbers(43));
        // Written down once, so a change of the generator shows up as a failing test: the sample
        // organization of a seed would change with it.
        assert_eq!(numbers(42), [741, 159, 278, 344, 38, 868, 218, 800]);
    }

    #[test]
    fn numbers_stay_below_the_bound_and_a_shuffle_keeps_every_item() {
        let mut random = SeededRandom::new(7);
        assert!((0..500).all(|_| random.below(5) < 5));
        assert_eq!(random.below(0), 0);
        let mut items: Vec<u32> = (0..20).collect();
        random.shuffle(&mut items);
        assert_ne!(items, (0..20).collect::<Vec<u32>>());
        items.sort_unstable();
        assert_eq!(items, (0..20).collect::<Vec<u32>>());
    }
}
