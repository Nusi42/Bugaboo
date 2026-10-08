//! Bytes as lower-case hexadecimal text: the one encoder and decoder of the program (key files,
//! the log anchor, short digests in logs).

/// The sixteen digits.
const DIGITS: &[u8; 16] = b"0123456789abcdef";

/// The bytes as two lower-case hexadecimal digits each.
///
/// The text is built digit by digit, without a copy of any byte in a temporary string, so a
/// caller that wraps the result in `Zeroizing` leaves no trace of a key behind.
#[must_use]
pub fn encode(bytes: &[u8]) -> String {
    let mut text = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        text.push(char::from(DIGITS[usize::from(byte >> 4)]));
        text.push(char::from(DIGITS[usize::from(byte & 0x0f)]));
    }
    text
}

/// Reads the digits of `text` into `bytes`, which they must fill exactly. `false` (and `bytes`
/// half written) for another length or a character that is not a hexadecimal digit.
#[must_use]
pub fn decode_into(text: &str, bytes: &mut [u8]) -> bool {
    if text.len() != bytes.len() * 2 || !text.is_ascii() {
        return false;
    }
    for (at, slot) in bytes.iter_mut().enumerate() {
        // A sign (`+1`) would pass `from_str_radix`, so every digit is checked on its own.
        let pair = &text[at * 2..at * 2 + 2];
        if !pair.bytes().all(|digit| digit.is_ascii_hexdigit()) {
            return false;
        }
        match u8::from_str_radix(pair, 16) {
            Ok(value) => *slot = value,
            Err(_) => return false,
        }
    }
    true
}

/// The `N` bytes written in `text`, if it has exactly the digits for them.
#[must_use]
pub fn decode<const N: usize>(text: &str) -> Option<[u8; N]> {
    let mut bytes = [0_u8; N];
    decode_into(text, &mut bytes).then_some(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bytes_are_written_in_lower_case_and_read_back() {
        assert_eq!(encode(&[0x00, 0x0f, 0xa5, 0xff]), "000fa5ff");
        assert_eq!(decode::<4>("000fa5ff"), Some([0x00, 0x0f, 0xa5, 0xff]));
        assert_eq!(decode::<4>("000FA5FF"), Some([0x00, 0x0f, 0xa5, 0xff]));
        assert_eq!(encode(&[]), "");
    }

    #[test]
    fn text_of_the_wrong_length_or_with_other_characters_is_refused() {
        assert_eq!(decode::<2>("000"), None);
        assert_eq!(decode::<2>("000000"), None);
        assert_eq!(decode::<2>("00zz"), None);
        assert_eq!(decode::<2>("+1+1"), None);
        assert_eq!(decode::<1>("é"), None);
    }
}
