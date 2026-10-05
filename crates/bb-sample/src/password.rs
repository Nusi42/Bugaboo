//! The random passwords of the sample people. They come from the operating system, never from the
//! seed: a password that the seed could make again would be a fixed password (SEC-SECRETS), and
//! the sample must not have one. They are written to the ignored file `sign-in.txt` of the run and
//! nowhere else.

use crate::error::SampleError;

/// Digits and capital letters without I, L, O and U (the alphabet of Crockford's base 32), so
/// nothing is mistaken for something else when a person reads a password from the file.
const ALPHABET: &[u8; 32] = b"0123456789ABCDEFGHJKMNPQRSTVWXYZ";
/// Characters in a group, and groups in a password: 4 × 5 letters is 100 bits of randomness.
const GROUP: usize = 5;
const GROUPS: usize = 4;

/// A new random password like `K7QMA-4XTRD-HZ2WJ-9M8VB`. It is long enough and on no list of
/// breached passwords (the use case checks both anyway).
///
/// # Errors
/// Fails when the system gives no secure random numbers.
pub(crate) fn random_password() -> Result<String, SampleError> {
    let mut bytes = [0_u8; GROUP * GROUPS];
    getrandom::fill(&mut bytes).map_err(|_| SampleError::Random)?;
    let mut password = String::with_capacity(bytes.len() + GROUPS);
    for (at, byte) in bytes.iter().enumerate() {
        if at > 0 && at % GROUP == 0 {
            password.push('-');
        }
        // 32 letters and a byte with 8 bits: the low 5 bits are uniform.
        password.push(char::from(ALPHABET[usize::from(byte & 0b1_1111)]));
    }
    Ok(password)
}

#[cfg(test)]
mod tests {
    use bb_auth::check_policy;

    use super::*;

    #[test]
    fn a_password_is_long_random_readable_and_accepted_by_the_policy() {
        let first = random_password().unwrap();
        assert_eq!(first.len(), GROUP * GROUPS + GROUPS - 1);
        assert_eq!(first.split('-').count(), GROUPS);
        assert!(
            first
                .chars()
                .all(|c| c == '-' || ALPHABET.contains(&u8::try_from(c).unwrap()))
        );
        assert!(check_policy(&first).is_ok());
        assert_ne!(first, random_password().unwrap(), "never the same twice");
    }
}
