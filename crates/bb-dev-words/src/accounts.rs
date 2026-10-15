//! What the tests of sign-in share: the public address the passkeys of the tests belong to, two
//! passwords and the moment the clock of the tests starts at. They are test data only; no program
//! uses them to sign anybody in.

/// The start of the clock of the tests, in seconds since 1970: 2027-01-15 08:00:00 UTC. One start
/// for every test, so the dates are always the same.
pub const CLOCK_START: i64 = 1_800_000_000;
/// The public address the passkeys of the tests belong to.
pub const PUBLIC_URL: &str = "https://bugaboo.test:47200";
/// A password that is not on the list of breached ones and has enough characters.
pub const PASSWORD: &str = "Tr4mpoline-Quiet-Harbor-81";
/// Another one, for changing it.
pub const OTHER_PASSWORD: &str = "Violet-Lantern-Marsh-5520";
