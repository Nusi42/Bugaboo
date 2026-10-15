//! Two-step codes from a code app (AUTH-03): the shared secret, the address and the QR picture
//! for the setup, and checking a typed code.
//!
//! Codes are the standard six digits that change every [`STEP_SECONDS`] seconds (RFC 6238, SHA-1
//! as every code app expects). A code is accepted for the current step and the step before and
//! after it (clocks differ a little), and **only once**: the caller remembers the newest step
//! that was accepted and passes it back, and a code for that step or an older one is refused.

use jiff::Timestamp;
use totp_rs::{Algorithm, Builder, Secret, Totp};
use zeroize::Zeroizing;

use crate::error::{AuthError, random};

/// How long a code lasts, in seconds.
pub const STEP_SECONDS: u64 = 30;
/// The size of the shared secret in bytes (160 bits, RFC 4226).
const SECRET_LENGTH: usize = 20;
/// The steps before and after the current one that are still accepted.
const SKEW_STEPS: u64 = 1;

/// The secret that the app and the server share. Wiped from memory when dropped.
pub struct TotpSecret(Zeroizing<Vec<u8>>);

impl std::fmt::Debug for TotpSecret {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("TotpSecret(<hidden>)")
    }
}

/// What the setup screen shows: the QR picture, and the same secret as text for people who type
/// it into the app.
pub struct Enrolment {
    /// The `otpauth://` address the code app reads.
    pub otpauth_url: Zeroizing<String>,
    /// The QR picture of the address, a PNG as base64 text.
    pub qr_png_base64: String,
    /// The secret in base32, for typing.
    pub secret_base32: Zeroizing<String>,
}

impl TotpSecret {
    /// A new secret.
    ///
    /// # Errors
    /// Fails when the system gives no secure random bytes.
    pub fn generate() -> Result<Self, AuthError> {
        Ok(Self(Zeroizing::new(random::<SECRET_LENGTH>()?.to_vec())))
    }

    /// The secret made of stored bytes.
    #[must_use]
    pub fn from_bytes(bytes: &[u8]) -> Self {
        Self(Zeroizing::new(bytes.to_vec()))
    }

    /// The bytes, for storing them encrypted.
    #[must_use]
    pub fn as_bytes(&self) -> &[u8] {
        &self.0
    }

    fn totp(&self, issuer: Option<&str>, account: &str) -> Result<Totp, AuthError> {
        // The address format uses ':' to separate the two names.
        let plain = |text: &str| text.replace(':', "_");
        Builder::new()
            .with_algorithm(Algorithm::SHA1)
            .with_digits(6)
            .with_skew(0)
            .with_step_duration(STEP_SECONDS)
            .with_secret(Secret::from(self.0.as_slice()))
            .with_issuer(issuer.map(plain))
            .with_account_name(plain(account))
            .build()
            .map_err(|_| AuthError::Totp)
    }

    /// The address and the picture for the setup. `issuer` is the name of the product and
    /// `account` the name of the person, as the app shows them.
    ///
    /// # Errors
    /// Fails when the address or the picture cannot be made.
    pub fn enrolment(&self, issuer: &str, account: &str) -> Result<Enrolment, AuthError> {
        let totp = self.totp(Some(issuer), account)?;
        Ok(Enrolment {
            otpauth_url: Zeroizing::new(totp.to_url().map_err(|_| AuthError::Totp)?),
            qr_png_base64: totp.to_qr_base64().map_err(|_| AuthError::Totp)?,
            secret_base32: Zeroizing::new(totp.secret().to_base32()),
        })
    }

    /// The code for the step that `time` falls into (for tests and for the soft app they use).
    ///
    /// # Errors
    /// Fails when the secret is not usable.
    pub fn code_at(&self, time: Timestamp) -> Result<String, AuthError> {
        let seconds = u64::try_from(time.as_second()).map_err(|_| AuthError::Totp)?;
        Ok(self.totp(None, "")?.generate(seconds).to_string())
    }

    /// Checks a typed code at `time`. `last_step` is the newest step a code was accepted for
    /// before (0 for none). Returns the step the code belongs to when it is right and newer than
    /// `last_step`; the caller stores it. All three steps are always worked out, so the time does
    /// not show which one matched.
    ///
    /// # Errors
    /// Fails when the secret is not usable.
    pub fn check(
        &self,
        code: &str,
        time: Timestamp,
        last_step: i64,
    ) -> Result<Option<i64>, AuthError> {
        let seconds = u64::try_from(time.as_second()).map_err(|_| AuthError::Totp)?;
        let totp = self.totp(None, "")?;
        let now_step = seconds / STEP_SECONDS;
        let mut found = None;
        for step in now_step.saturating_sub(SKEW_STEPS)..=now_step + SKEW_STEPS {
            let step_time = step * STEP_SECONDS;
            // With no skew, `check` accepts exactly the code of this step.
            if totp.check(code, step_time).is_some() {
                let step = i64::try_from(step).map_err(|_| AuthError::Totp)?;
                if step > last_step {
                    found = Some(step);
                }
            }
        }
        Ok(found)
    }
}

#[cfg(test)]
mod tests {
    use bb_dev_words::accounts::CLOCK_START;

    use super::*;

    fn at(seconds: i64) -> Timestamp {
        Timestamp::from_second(seconds).unwrap()
    }

    #[test]
    fn auth_03_the_current_code_is_accepted_once() {
        let secret = TotpSecret::generate().unwrap();
        let now = at(CLOCK_START);
        let code = secret.code_at(now).unwrap();
        assert_eq!(code.len(), 6);
        let step = secret.check(&code, now, 0).unwrap().unwrap();
        assert_eq!(step, CLOCK_START / 30);
        // The same code again, or at any later moment of its step, is refused.
        assert_eq!(secret.check(&code, now, step).unwrap(), None);
        assert_eq!(secret.check(&code, at(1_800_000_010), step).unwrap(), None);
    }

    #[test]
    fn auth_03_a_code_from_a_neighbouring_step_is_accepted_but_a_far_one_is_not() {
        let secret = TotpSecret::generate().unwrap();
        let now = at(CLOCK_START);
        let before = secret.code_at(at(CLOCK_START - 30)).unwrap();
        let after = secret.code_at(at(CLOCK_START + 30)).unwrap();
        let far = secret.code_at(at(CLOCK_START + 90)).unwrap();
        assert!(secret.check(&before, now, 0).unwrap().is_some());
        assert!(secret.check(&after, now, 0).unwrap().is_some());
        assert_eq!(secret.check(&far, now, 0).unwrap(), None);
    }

    #[test]
    fn auth_03_wrong_text_and_other_secrets_are_refused() {
        let secret = TotpSecret::generate().unwrap();
        let other = TotpSecret::generate().unwrap();
        let now = at(CLOCK_START);
        for bad in ["", "12345", "1234567", "abcdef", "12 34 56"] {
            assert_eq!(secret.check(bad, now, 0).unwrap(), None, "{bad}");
        }
        let code = other.code_at(now).unwrap();
        assert_eq!(secret.check(&code, now, 0).unwrap(), None);
    }

    #[test]
    fn auth_03_the_setup_shows_a_picture_an_address_and_the_secret_as_text() {
        let secret = TotpSecret::generate().unwrap();
        let enrolment = secret.enrolment("Bugaboo", "ada@example.com").unwrap();
        assert!(enrolment.otpauth_url.starts_with("otpauth://totp/"));
        assert!(enrolment.otpauth_url.contains("issuer=Bugaboo"));
        assert!(!enrolment.qr_png_base64.is_empty());
        assert_eq!(enrolment.secret_base32.len(), 32);
        assert!(!format!("{secret:?}").contains(&*enrolment.secret_base32));
    }
}
