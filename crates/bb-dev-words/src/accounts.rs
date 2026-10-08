//! What the tests of sign-in share: the public address the passkeys of the tests belong to, and two
//! passwords. They are test data only; no program uses them to sign anybody in.

/// The public address the passkeys of the tests belong to.
pub const PUBLIC_URL: &str = "https://bugaboo.test:47200";
/// A password that is not on the list of breached ones and has enough characters.
pub const PASSWORD: &str = "Tr4mpoline-Quiet-Harbor-81";
/// Another one, for changing it.
pub const OTHER_PASSWORD: &str = "Violet-Lantern-Marsh-5520";
