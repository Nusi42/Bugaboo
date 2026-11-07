//! The port of the AI setup: what it needs from storage (AI-01, PRIV-08).
//!
//! The settings of AI are ordinary settings of the registry (`SettingsStore`); this port holds the
//! two things that are not: the key of the provider, a stored secret (SEC-16), and the agreement.

use std::future::Future;

use bb_core::ai::Agreement;
use jiff::Timestamp;
use zeroize::Zeroizing;

use crate::settings::SettingsStore;
use crate::store_macros::{reads, writes};

/// What the setup of AI needs from storage.
pub trait AiStore: SettingsStore + 'static {
    reads! {
        /// The key of the provider, stored encrypted (SEC-16). The bytes are wiped from memory
        /// when dropped.
        fn ai_key() -> Option<Zeroizing<Vec<u8>>>;
        /// Stores the key of the provider, replacing the one there. Bookkeeping of a secret, not a
        /// domain change: the use case writes the event of it.
        fn put_ai_key(secret: &[u8], at: Timestamp) -> ();
        /// Deletes the key of the provider.
        fn delete_ai_key() -> ();
        /// The agreement an administrator confirmed, if there is one.
        fn ai_agreement() -> Option<Agreement>;
        /// Notes that the reference with this random word is used, and forgets the words of the
        /// references that ran out before `now`. Answers whether it was the first use: a word
        /// that is there already answers `false`.
        fn use_assistant_reference(nonce: &str, now: Timestamp, expires_at: Timestamp) -> bool;
    }

    writes! {
        /// Stores the agreement, replacing the one there.
        fn put_ai_agreement(agreement: &Agreement) -> ();
        /// Deletes the agreement: the provider changed, so it must be confirmed again.
        fn delete_ai_agreement() -> ();
        /// Deletes the key of the provider inside a change: it belongs to the provider that was
        /// there before, and is never given to another (SEC-16).
        fn forget_ai_key() -> ();
    }
}
