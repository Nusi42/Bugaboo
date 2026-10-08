//! Passwords (AUTH-01): the rules, Argon2id hashing and checking.
//!
//! - **Rules** (NIST 800-63B): length over composition. At least [`MIN_LENGTH`] characters, no
//!   rules about capitals or symbols, and not on the bundled list of breached passwords.
//! - **Hashing:** Argon2id with the OWASP parameters (architecture §7): 19 MiB of memory, 2
//!   iterations, 1 lane. The result is a PHC string that carries its own parameters and salt, so a
//!   password hashed with older parameters still verifies and is hashed again on the next
//!   sign-in ([`Verdict::needs_rehash`]).
//! - **Equal timing:** [`PasswordHasher::verify`] does the same work for an account that does not
//!   exist (it checks against a hash made at start-up), so the time does not tell whether an
//!   account exists.
//! - **Cost control:** hashing runs on the blocking threads of the runtime, and at most as many
//!   hashes run at once as the computer has processors, so a burst of sign-ins cannot use up the
//!   memory (19 MiB each). Rate limits turn requests away before they get here (SEC-14).

use std::sync::Arc;

use argon2::password_hash::phc::PasswordHash;
use argon2::password_hash::{PasswordHasher as _, PasswordVerifier as _};
use argon2::{Algorithm, Argon2, Params, Version};
use tokio::sync::Semaphore;
use zeroize::Zeroizing;

use crate::breached;
use crate::error::{AuthError, random};

/// The fewest characters a password has.
pub const MIN_LENGTH: usize = 12;
/// The most characters a password has. Longer text is refused before any work is done: hashing a
/// megabyte is a way to waste the server's time.
pub const MAX_LENGTH: usize = 256;
/// Argon2 memory in KiB (19 MiB, OWASP).
pub const ARGON2_MEMORY_KIB: u32 = 19 * 1024;
/// Argon2 iterations (OWASP).
pub const ARGON2_ITERATIONS: u32 = 2;
/// Argon2 lanes (OWASP).
pub const ARGON2_LANES: u32 = 1;
/// The length of the hash in bytes.
const OUTPUT_LENGTH: usize = 32;
/// The length of the salt in bytes.
const SALT_LENGTH: usize = 16;

/// Why a password is not accepted as a new password.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PasswordProblem {
    /// Shorter than [`MIN_LENGTH`] characters.
    TooShort,
    /// Longer than [`MAX_LENGTH`] characters.
    TooLong,
    /// On the list of breached passwords.
    Breached,
}

/// Checks a new password against the rules.
///
/// # Errors
/// The first rule it breaks.
pub fn check_policy(password: &str) -> Result<(), PasswordProblem> {
    let length = password.chars().count();
    if length < MIN_LENGTH {
        Err(PasswordProblem::TooShort)
    } else if length > MAX_LENGTH {
        Err(PasswordProblem::TooLong)
    } else if breached::is_breached(password) {
        Err(PasswordProblem::Breached)
    } else {
        Ok(())
    }
}

/// What checking a password found out.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Verdict {
    /// The password is the right one for an account that exists.
    pub matches: bool,
    /// The right password, but the stored hash was made with other parameters: store a new hash.
    pub needs_rehash: bool,
}

fn argon2() -> Result<Argon2<'static>, AuthError> {
    let params = Params::new(
        ARGON2_MEMORY_KIB,
        ARGON2_ITERATIONS,
        ARGON2_LANES,
        Some(OUTPUT_LENGTH),
    )
    .map_err(|_| AuthError::Hashing)?;
    Ok(Argon2::new(Algorithm::Argon2id, Version::V0x13, params))
}

fn hash_now(password: &str) -> Result<String, AuthError> {
    let salt = random::<SALT_LENGTH>()?;
    let hash = argon2()?
        .hash_password_with_salt(password.as_bytes(), &salt)
        .map_err(|_| AuthError::Hashing)?;
    Ok(hash.to_string())
}

fn uses_current_parameters(hash: &PasswordHash) -> bool {
    Params::try_from(hash).is_ok_and(|found| {
        found.m_cost() == ARGON2_MEMORY_KIB
            && found.t_cost() == ARGON2_ITERATIONS
            && found.p_cost() == ARGON2_LANES
            && found.output_len() == Some(OUTPUT_LENGTH)
    }) && hash.algorithm.as_str() == Algorithm::Argon2id.as_str()
}

fn verify_now(password: &str, stored: &str, dummy: &str, account_exists: bool) -> Verdict {
    // A stored hash that cannot be read counts as a wrong password, after the same work.
    let parsed = PasswordHash::new(stored).ok();
    let readable = parsed.is_some();
    let Some(hash) = parsed.or_else(|| PasswordHash::new(dummy).ok()) else {
        return Verdict {
            matches: false,
            needs_rehash: false,
        };
    };
    let right = Argon2::default()
        .verify_password(password.as_bytes(), &hash)
        .is_ok();
    let matches = right && account_exists && readable;
    Verdict {
        matches,
        needs_rehash: matches && !uses_current_parameters(&hash),
    }
}

/// Hashes and checks passwords without blocking the runtime.
#[derive(Clone, Debug)]
pub struct PasswordHasher {
    permits: Arc<Semaphore>,
    dummy: Arc<str>,
}

impl PasswordHasher {
    /// A hasher. It makes one hash right away (the one unknown accounts are checked against), so
    /// it takes a moment.
    ///
    /// # Errors
    /// Fails when the system gives no secure random bytes.
    pub fn new() -> Result<Self, AuthError> {
        let processors = std::thread::available_parallelism().map_or(2, std::num::NonZero::get);
        // A password nobody can type: 16 random bytes in hex.
        let unknown = Zeroizing::new(bb_core::hex::encode(&random::<16>()?));
        Ok(Self {
            permits: Arc::new(Semaphore::new(processors.max(2))),
            dummy: Arc::from(hash_now(&unknown)?),
        })
    }

    /// The PHC string for `password`, with a new random salt.
    ///
    /// # Errors
    /// Fails when hashing fails.
    pub async fn hash(&self, password: Zeroizing<String>) -> Result<String, AuthError> {
        let _permit = self
            .permits
            .acquire()
            .await
            .map_err(|_| AuthError::Hashing)?;
        tokio::task::spawn_blocking(move || hash_now(&password))
            .await
            .map_err(|_| AuthError::Hashing)?
    }

    /// Checks `password` against the stored hash of an account, or, with `None`, against the hash
    /// of nobody (the account does not exist): the work and so the time are the same.
    ///
    /// # Errors
    /// Fails when the work could not be done at all.
    pub async fn verify(
        &self,
        password: Zeroizing<String>,
        stored: Option<String>,
    ) -> Result<Verdict, AuthError> {
        let _permit = self
            .permits
            .acquire()
            .await
            .map_err(|_| AuthError::Hashing)?;
        let dummy = Arc::clone(&self.dummy);
        tokio::task::spawn_blocking(move || {
            let exists = stored.is_some();
            let stored = stored.unwrap_or_else(|| dummy.to_string());
            verify_now(&password, &stored, &dummy, exists)
        })
        .await
        .map_err(|_| AuthError::Hashing)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn secret(text: &str) -> Zeroizing<String> {
        Zeroizing::new(text.to_owned())
    }

    #[test]
    fn auth_01_length_counts_and_composition_does_not() {
        assert_eq!(check_policy("short"), Err(PasswordProblem::TooShort));
        assert_eq!(check_policy("elevenchars"), Err(PasswordProblem::TooShort));
        // Characters, not bytes: twelve letters with accents are twelve characters.
        assert_eq!(check_policy("äöüäöüäöüäöü"), Ok(()));
        assert_eq!(check_policy("all lower case words"), Ok(()));
        assert_eq!(
            check_policy(&"x".repeat(MAX_LENGTH + 1)),
            Err(PasswordProblem::TooLong)
        );
        assert_eq!(check_policy(&"ab".repeat(MAX_LENGTH / 2)), Ok(()));
    }

    #[test]
    fn auth_01_breached_passwords_are_refused_whatever_the_capitals() {
        assert_eq!(check_policy("1qaz2wsx3edc"), Err(PasswordProblem::Breached));
        assert_eq!(check_policy("QWERTYQWERTY"), Err(PasswordProblem::Breached));
        assert_eq!(check_policy("a fine, long passphrase"), Ok(()));
    }

    #[tokio::test]
    async fn auth_01_a_hash_carries_the_owasp_parameters_and_verifies() {
        let hasher = PasswordHasher::new().unwrap();
        let hash = hasher.hash(secret("a long passphrase here")).await.unwrap();
        assert!(
            hash.starts_with("$argon2id$v=19$m=19456,t=2,p=1$"),
            "{hash}"
        );
        let right = hasher
            .verify(secret("a long passphrase here"), Some(hash.clone()))
            .await
            .unwrap();
        assert_eq!(
            right,
            Verdict {
                matches: true,
                needs_rehash: false
            }
        );
        let wrong = hasher
            .verify(secret("another passphrase!"), Some(hash.clone()))
            .await
            .unwrap();
        assert!(!wrong.matches);
        // Every hash has its own salt.
        let second = hasher.hash(secret("a long passphrase here")).await.unwrap();
        assert_ne!(hash, second);
    }

    #[tokio::test]
    async fn auth_01_a_hash_with_other_parameters_verifies_and_asks_for_a_new_hash() {
        let hasher = PasswordHasher::new().unwrap();
        let weaker = Argon2::new(
            Algorithm::Argon2id,
            Version::V0x13,
            Params::new(8 * 1024, 1, 1, Some(OUTPUT_LENGTH)).unwrap(),
        )
        .hash_password_with_salt(b"a long passphrase here", &[7_u8; SALT_LENGTH])
        .unwrap()
        .to_string();
        let verdict = hasher
            .verify(secret("a long passphrase here"), Some(weaker))
            .await
            .unwrap();
        assert_eq!(
            verdict,
            Verdict {
                matches: true,
                needs_rehash: true
            }
        );
    }

    #[tokio::test]
    async fn auth_01_an_unknown_account_and_a_broken_hash_never_match() {
        let hasher = PasswordHasher::new().unwrap();
        let unknown = hasher
            .verify(secret("a long passphrase here"), None)
            .await
            .unwrap();
        assert!(!unknown.matches && !unknown.needs_rehash);
        let broken = hasher
            .verify(
                secret("a long passphrase here"),
                Some("not a hash".to_owned()),
            )
            .await
            .unwrap();
        assert!(!broken.matches);
    }
}
