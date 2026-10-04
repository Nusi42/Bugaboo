//! Passwords, passkeys, two-step codes, sessions, access keys, company sign-in and OAuth 2.1.
//!
//! This crate holds the *mechanisms* of signing in, with no database and no HTTP: hashing and
//! checking passwords against a bundled list of breached ones (AUTH-01), the secrets behind
//! sessions, invitations and the setup code and the CSRF token derived from a session (SEC-12),
//! two-step codes and backup codes (AUTH-03), passkeys (AUTH-02) and the small rules about e-mail
//! addresses and devices. The use cases that tie them together live in `bb-app`.

#![forbid(unsafe_code)]

mod breached;
mod code;
mod csrf;
mod device;
mod email;
mod error;
mod passkeys;
mod password;
mod token;
mod totp;

pub use code::{HumanCode, code_hash};
pub use csrf::CsrfKey;
pub use device::Device;
pub use email::{EmailAddress, domain_is_listed};
pub use error::AuthError;
pub use passkeys::{
    CHALLENGE_LIFETIME, ChallengeId, DiscoverableMatch, PasskeyCheck, Passkeys, RelyingParty,
    StoredCredential,
};
pub use password::{
    ARGON2_ITERATIONS, ARGON2_LANES, ARGON2_MEMORY_KIB, MAX_LENGTH, MIN_LENGTH, PasswordHasher,
    PasswordProblem, Verdict, check_policy,
};
pub use token::{Purpose, SecretToken, same_digest};
pub use totp::{Enrolment, STEP_SECONDS, TotpSecret};

// Used by the integration tests in `tests/` only (`unused_crate_dependencies`).
#[cfg(test)]
use webauthn_authenticator_rs as _;
