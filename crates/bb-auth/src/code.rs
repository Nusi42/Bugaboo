//! Codes a person reads and types: the setup code (AUTH-09) and backup codes (AUTH-03).
//!
//! A code is 16 symbols from a 32-symbol alphabet without similar-looking symbols (80 bits), written in four
//! groups: `k7q2-x9mf-3hd8-vt4w`. Typing it is forgiving: capital letters, spaces and hyphens do
//! not matter, and `o`, `i` and `l` are read as `0`, `1` and `1` (Crockford's base 32). Only a
//! hash is stored; with 80 bits of randomness a plain salted-by-purpose SHA-256 is enough.

use bb_core::security_log::Digest;
use zeroize::Zeroizing;

use crate::error::{AuthError, random};
use crate::token::Purpose;

/// The symbols of a code: digits and letters without `i`, `l`, `o` and `u`.
const ALPHABET: &[u8; 32] = b"0123456789abcdefghjkmnpqrstvwxyz";
/// How many symbols a code has.
const SYMBOLS: usize = 16;
/// How many symbols stand together before a hyphen.
const GROUP: usize = 4;

/// A code as it is shown once: `k7q2-x9mf-3hd8-vt4w`. Wiped from memory when dropped.
pub struct HumanCode(Zeroizing<String>);

impl std::fmt::Debug for HumanCode {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("HumanCode(<hidden>)")
    }
}

impl HumanCode {
    /// A new code.
    ///
    /// # Errors
    /// Fails when the system gives no secure random bytes.
    pub fn generate() -> Result<Self, AuthError> {
        let bytes = Zeroizing::new(random::<SYMBOLS>()?);
        let mut text = Zeroizing::new(String::with_capacity(SYMBOLS + SYMBOLS / GROUP));
        for (at, byte) in bytes.iter().enumerate() {
            if at > 0 && at % GROUP == 0 {
                text.push('-');
            }
            // 256 is a multiple of 32, so every symbol is equally likely.
            text.push(char::from(ALPHABET[usize::from(byte & 31)]));
        }
        Ok(Self(text))
    }

    /// The code as shown to the person.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// The hash that is stored in place of the code.
    #[must_use]
    pub fn hash(&self, purpose: Purpose) -> Digest {
        code_hash(purpose, &self.0).unwrap_or_else(|| purpose.hash(self.0.as_bytes()))
    }
}

/// The hash of a code a person typed, for `purpose`; `None` when the text cannot be a code
/// (wrong length or a symbol outside the alphabet), so nothing needs to be looked up.
#[must_use]
pub fn code_hash(purpose: Purpose, typed: &str) -> Option<Digest> {
    let mut symbols = Zeroizing::new(Vec::with_capacity(SYMBOLS));
    for character in typed.chars() {
        let symbol = match character.to_ascii_lowercase() {
            ' ' | '-' => continue,
            'o' => '0',
            'i' | 'l' => '1',
            other => other,
        };
        if !symbol.is_ascii() || !ALPHABET.contains(&(symbol as u8)) {
            return None;
        }
        symbols.push(symbol as u8);
    }
    (symbols.len() == SYMBOLS).then(|| purpose.hash(&symbols))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sec_12_a_code_has_four_groups_and_hashes_the_same_however_it_is_typed() {
        let code = HumanCode::generate().unwrap();
        let shown = code.as_str();
        assert_eq!(shown.len(), 19);
        assert_eq!(shown.matches('-').count(), 3);
        let expected = code.hash(Purpose::BackupCode);
        for typed in [
            shown.to_owned(),
            shown.to_uppercase(),
            shown.replace('-', " "),
            shown.replace('-', ""),
            format!("  {shown}  "),
        ] {
            assert_eq!(code_hash(Purpose::BackupCode, &typed), Some(expected));
        }
    }

    #[test]
    fn sec_12_look_alike_letters_are_read_as_the_digits_they_resemble() {
        let digits = code_hash(Purpose::SetupCode, "0110-0110-0110-0110");
        let letters = code_hash(Purpose::SetupCode, "olio-OLIO-oIlO-0ll0");
        assert!(digits.is_some());
        assert_eq!(digits, letters);
    }

    #[test]
    fn sec_12_text_that_cannot_be_a_code_has_no_hash() {
        for bad in [
            "",
            "abcd",
            "abcd-efgh-jkmn-pqrs-t",
            "abcd-efgh-jkmn-pqr!",
            "ünïcödé-ünïcödé",
            "uuuu-uuuu-uuuu-uuuu",
        ] {
            assert_eq!(code_hash(Purpose::SetupCode, bad), None, "{bad}");
        }
    }

    #[test]
    fn sec_12_codes_are_new_every_time_and_hash_per_purpose() {
        let first = HumanCode::generate().unwrap();
        let second = HumanCode::generate().unwrap();
        assert_ne!(first.as_str(), second.as_str());
        assert_ne!(
            first.hash(Purpose::BackupCode),
            first.hash(Purpose::SetupCode)
        );
        assert!(!format!("{first:?}").contains(first.as_str()));
    }
}
