//! The secret of an access key (AUTH-07): `bb_` + a public prefix + a 256-bit secret.
//!
//! A key looks like `bb_k7q2x9mf_<52 symbols>`: the prefix is 8 random symbols that tell the keys
//! of a person apart in their list and that the secret scan can recognize; the secret is 256
//! random bits in the same symbols as the codes of the second step (Crockford's base 32, written
//! in lower case). The key is shown once. Only the hash of the secret is stored, in its own hash
//! domain, so the hash of a key can never be taken for the hash of a session or an invitation.
//! The prefix is not part of what is hashed: it is public, and the secret alone is what has to
//! be guessed.

use bb_core::security_log::Digest;
use zeroize::Zeroizing;

use crate::code::ALPHABET;
use crate::error::{AuthError, random};
use crate::token::Purpose;

/// What every access key starts with. The secret scan looks for it (`.gitleaks.toml`).
pub const KEY_START: &str = "bb_";
/// How many symbols the public prefix has.
pub const PREFIX_SYMBOLS: usize = 8;
/// Bytes in the secret (256 bits).
const SECRET_BYTES: usize = 32;
/// Symbols the secret takes: 256 bits in groups of 5, the last group padded with zeros.
const SECRET_SYMBOLS: usize = 52;

/// An access key as it is shown once and as a person types or sends it. Wiped from memory when
/// dropped, and never shown in `Debug` output.
pub struct AccessKeySecret {
    prefix: String,
    secret: Zeroizing<[u8; SECRET_BYTES]>,
}

impl std::fmt::Debug for AccessKeySecret {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("AccessKeySecret(<hidden>)")
    }
}

/// Whether `text` starts like an access key. It says nothing about whether it is one that works.
#[must_use]
pub fn looks_like_access_key(text: &str) -> bool {
    text.starts_with(KEY_START)
}

fn symbol(value: u8) -> char {
    char::from(ALPHABET[usize::from(value & 31)])
}

fn value_of(symbol: u8) -> Option<u8> {
    ALPHABET
        .iter()
        .position(|candidate| *candidate == symbol)
        .and_then(|at| u8::try_from(at).ok())
}

impl AccessKeySecret {
    /// A new key.
    ///
    /// # Errors
    /// Fails when the system gives no secure random bytes.
    pub fn generate() -> Result<Self, AuthError> {
        let prefix_bytes = random::<PREFIX_SYMBOLS>()?;
        Ok(Self {
            // 256 is a multiple of 32, so every symbol is equally likely.
            prefix: prefix_bytes.iter().map(|byte| symbol(*byte)).collect(),
            secret: Zeroizing::new(random()?),
        })
    }

    /// The public prefix: 8 symbols.
    #[must_use]
    pub fn prefix(&self) -> &str {
        &self.prefix
    }

    /// The key as shown once: `bb_`, the prefix, `_` and the secret.
    #[must_use]
    pub fn encode(&self) -> Zeroizing<String> {
        let mut text = Zeroizing::new(String::with_capacity(
            KEY_START.len() + PREFIX_SYMBOLS + 1 + SECRET_SYMBOLS,
        ));
        text.push_str(KEY_START);
        text.push_str(&self.prefix);
        text.push('_');
        let mut buffer = 0_u16;
        let mut bits = 0_u32;
        for byte in self.secret.iter() {
            buffer = (buffer << 8) | u16::from(*byte);
            bits += 8;
            while bits >= 5 {
                bits -= 5;
                text.push(symbol(u8::try_from((buffer >> bits) & 31).unwrap_or(0)));
            }
        }
        if bits > 0 {
            text.push(symbol(
                u8::try_from((buffer << (5 - bits)) & 31).unwrap_or(0),
            ));
        }
        text
    }

    /// The key written by [`Self::encode`]; `None` for anything else. Only that exact spelling
    /// is a key: no other case, no spaces, no look-alike letters.
    #[must_use]
    pub fn parse(text: &str) -> Option<Self> {
        let rest = text.strip_prefix(KEY_START)?;
        if !rest.is_ascii() || rest.len() != PREFIX_SYMBOLS + 1 + SECRET_SYMBOLS {
            return None;
        }
        let (prefix, rest) = rest.split_at(PREFIX_SYMBOLS);
        let secret_text = rest.strip_prefix('_')?;
        if !prefix.bytes().all(|byte| value_of(byte).is_some()) {
            return None;
        }
        let mut secret = Zeroizing::new([0_u8; SECRET_BYTES]);
        let mut buffer = 0_u16;
        let mut bits = 0_u32;
        let mut at = 0;
        for byte in secret_text.bytes() {
            buffer = (buffer << 5) | u16::from(value_of(byte)?);
            bits += 5;
            if bits >= 8 {
                bits -= 8;
                *secret.get_mut(at)? = u8::try_from((buffer >> bits) & 0xFF).unwrap_or(0);
                at += 1;
            }
        }
        // The last symbol only has room for 1 bit of the secret; the other 4 must be zero, so
        // there is one way to write a key.
        let spare = buffer & ((1 << bits) - 1);
        (at == SECRET_BYTES && spare == 0).then(|| Self {
            prefix: prefix.to_owned(),
            secret,
        })
    }

    /// The hash that is stored in place of the secret.
    #[must_use]
    pub fn hash(&self) -> Digest {
        Purpose::AccessKey.hash(self.secret.as_slice())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::token::same_digest;

    #[test]
    fn auth_07_a_key_has_the_start_the_prefix_and_the_secret() {
        let key = AccessKeySecret::generate().unwrap();
        let text = key.encode();
        assert!(text.starts_with("bb_"));
        assert_eq!(text.len(), 3 + 8 + 1 + 52);
        assert_eq!(&text[3..11], key.prefix());
        assert_eq!(&text[11..12], "_");
        assert!(looks_like_access_key(&text));
        assert!(!looks_like_access_key("Bearer bb_x"));
    }

    #[test]
    fn auth_07_a_key_survives_its_text_and_is_different_every_time() {
        let first = AccessKeySecret::generate().unwrap();
        let second = AccessKeySecret::generate().unwrap();
        assert_ne!(*first.encode(), *second.encode());
        assert_ne!(first.prefix(), second.prefix());
        let again = AccessKeySecret::parse(&first.encode()).unwrap();
        assert_eq!(again.prefix(), first.prefix());
        assert!(same_digest(&first.hash(), &again.hash()));
        assert!(!same_digest(&first.hash(), &second.hash()));
        assert_eq!(*again.encode(), *first.encode());
    }

    #[test]
    fn auth_07_text_that_is_not_a_key_is_refused() {
        let good = AccessKeySecret::generate().unwrap().encode();
        let upper = good.to_uppercase();
        let spaced = format!(" {} ", good.as_str());
        let longer = format!("{}0", good.as_str());
        let shorter = good[..good.len() - 1].to_owned();
        let other_start = good.replacen("bb_", "bx_", 1);
        let other_separator = format!("{}-{}", &good[..11], &good[12..]);
        let look_alike = format!("{}o", &good[..good.len() - 1]);
        // The last symbol holds 1 bit of the secret: the other 4 bits must be zero.
        let spare_bits = {
            let mut text = good[..good.len() - 1].to_owned();
            text.push(if good.ends_with('z') { 'y' } else { 'z' });
            text
        };
        for bad in [
            "",
            "bb_",
            &upper,
            &spaced,
            &longer,
            &shorter,
            &other_start,
            &other_separator,
            &look_alike,
            "bb_ünïcödé_ünïcödé",
        ] {
            assert!(AccessKeySecret::parse(bad).is_none(), "{bad}");
        }
        // 'y' and 'z' both have set bits among the 4 spare ones, so neither is the one spelling.
        assert!(AccessKeySecret::parse(&spare_bits).is_none());
    }

    #[test]
    fn auth_07_the_hash_has_its_own_domain_and_ignores_the_prefix() {
        let key = AccessKeySecret::generate().unwrap();
        // The hash of the same bytes for another purpose is another hash.
        assert_ne!(key.hash(), Purpose::Session.hash(key.secret.as_slice()));
        assert_ne!(key.hash(), Purpose::Invitation.hash(key.secret.as_slice()));
        let text = key.encode();
        let other_prefix = format!("bb_{}{}", "0".repeat(8), &text[11..]);
        let moved = AccessKeySecret::parse(&other_prefix).unwrap();
        assert!(same_digest(&key.hash(), &moved.hash()));
    }

    #[test]
    fn sec_16_a_key_never_shows_in_debug_output() {
        let key = AccessKeySecret::generate().unwrap();
        let shown = format!("{key:?}");
        assert!(!shown.contains(&*key.encode()));
        assert!(!shown.contains(key.prefix()));
    }
}
