//! The sample makes people through the real use cases, which count failed tries. The sample has no
//! failed tries and no attacker, so it uses a limit that never limits.

use bb_app::auth::FailureLimit;

/// A [`FailureLimit`] that lets every try through.
#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct Unlimited;

impl FailureLimit for Unlimited {
    fn reserve(&self, _key: &str) -> Result<bool, u64> {
        Ok(false)
    }

    fn release(&self, _key: &str) {}

    fn clear(&self, _key: &str) {}

    fn tally(&self, _key: &str) -> u32 {
        0
    }

    fn forget(&self, _key: &str) {}
}
