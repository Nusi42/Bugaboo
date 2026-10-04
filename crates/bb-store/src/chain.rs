//! The hash chain of the security log (SEC-15), without any database.
//!
//! The hash of an entry covers the hash of the entry before it and every field of the entry, in
//! a fixed encoding where each variable part is preceded by its length (so two different entries
//! can never be encoded to the same bytes). Changing, removing, inserting or reordering an entry
//! breaks the link to its successor. What the chain cannot show is that entries were cut off at
//! the *end*: for that, the head (place and hash) is shown on the status page so an administrator
//! can compare it with an earlier note.

use bb_core::security_log::{Digest, SecurityLogEntry};
use sha2::{Digest as _, Sha256};

/// Starts every hash, so a hash made here is never also a hash of something else.
const DOMAIN: &[u8] = b"bugaboo.security-log.entry.v1";
/// Starts every signed checkpoint.
pub(crate) const CHECKPOINT_DOMAIN: &[u8] = b"bugaboo.security-log.checkpoint.v1";

/// The `previous_hash` of the very first entry.
pub(crate) const START: Digest = Digest::new([0; Digest::LENGTH]);

/// Entries are deleted in blocks of this many, counted from the first entry ever.
pub(crate) const BLOCK_SIZE: i64 = 1000;

fn text(hasher: &mut Sha256, value: &str) {
    // A length that does not fit in four bytes cannot happen for technical names and shortened
    // addresses; the clamp only keeps the encoding total.
    let length = u32::try_from(value.len()).unwrap_or(u32::MAX);
    hasher.update(length.to_be_bytes());
    hasher.update(value.as_bytes());
}

fn optional<T>(hasher: &mut Sha256, value: Option<&T>, write: impl FnOnce(&mut Sha256, &T)) {
    match value {
        Some(value) => {
            hasher.update([1]);
            write(hasher, value);
        }
        None => hasher.update([0]),
    }
}

/// The hash `entry` must have when the entry before it has the hash `previous`. The hash fields
/// of `entry` itself are not used.
pub(crate) fn hash_of(previous: &Digest, entry: &SecurityLogEntry) -> Digest {
    let mut hasher = Sha256::new();
    hasher.update(DOMAIN);
    hasher.update(previous.as_bytes());
    hasher.update(entry.seq.to_be_bytes());
    hasher.update(entry.time.as_microsecond().to_be_bytes());
    optional(&mut hasher, entry.person.as_ref(), |hasher, person| {
        hasher.update(person.to_bytes());
    });
    text(&mut hasher, entry.action.as_str());
    optional(&mut hasher, entry.subject.as_ref(), |hasher, subject| {
        text(hasher, subject.kind.as_str());
        hasher.update(subject.id.to_bytes());
    });
    optional(&mut hasher, entry.network.as_ref(), |hasher, network| {
        text(hasher, network.as_str());
    });
    Digest::new(hasher.finalize().into())
}

/// What a checkpoint signs: the place and the hash of the last deleted entry.
pub(crate) fn checkpoint_message(seq: i64, hash: &Digest) -> [u8; 8 + Digest::LENGTH] {
    let mut message = [0_u8; 8 + Digest::LENGTH];
    message[..8].copy_from_slice(&seq.to_be_bytes());
    message[8..].copy_from_slice(hash.as_bytes());
    message
}

/// Why a chain does not verify.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BreakReason {
    /// An entry is missing, duplicated or out of order.
    Gap,
    /// An entry does not point to the hash of the entry before it.
    BrokenLink,
    /// An entry's content does not match its hash.
    ChangedContent,
    /// The checkpoint's signature is wrong, or entries were deleted without a checkpoint.
    Checkpoint,
}

/// The result of verifying the security log.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Verification {
    /// Every entry verifies.
    Valid {
        /// How many entries were checked.
        entries: u64,
        /// The place and hash of the last entry (for the status page), if there is one.
        head: Option<(i64, Digest)>,
    },
    /// The chain breaks at this entry.
    Broken {
        /// The place of the first entry that does not verify.
        seq: i64,
        /// What is wrong with it.
        reason: BreakReason,
    },
}

/// Checks entries one after the other, so a long log never has to be in memory at once.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Verifier {
    expected_seq: i64,
    previous: Digest,
    entries: u64,
}

impl Verifier {
    /// Starts where the log starts: after the checkpoint, or at the very beginning.
    pub(crate) const fn new(expected_seq: i64, previous: Digest) -> Self {
        Self {
            expected_seq,
            previous,
            entries: 0,
        }
    }

    /// Checks the next entry.
    pub(crate) fn check(&mut self, entry: &SecurityLogEntry) -> Result<(), BreakReason> {
        if entry.seq != self.expected_seq {
            return Err(BreakReason::Gap);
        }
        if entry.previous_hash != self.previous {
            return Err(BreakReason::BrokenLink);
        }
        if hash_of(&self.previous, entry) != entry.hash {
            return Err(BreakReason::ChangedContent);
        }
        self.previous = entry.hash;
        self.expected_seq += 1;
        self.entries += 1;
        Ok(())
    }

    /// How the log ended, when every entry checked.
    pub(crate) fn finish(self) -> Verification {
        Verification::Valid {
            entries: self.entries,
            head: (self.entries > 0).then(|| (self.expected_seq - 1, self.previous)),
        }
    }
}

/// Verifies a whole list of entries that starts after `start_seq - 1` with `previous`.
#[cfg(test)]
pub(crate) fn verify_all(
    start_seq: i64,
    previous: Digest,
    entries: &[SecurityLogEntry],
) -> Verification {
    let mut verifier = Verifier::new(start_seq, previous);
    for entry in entries {
        if let Err(reason) = verifier.check(entry) {
            return Verification::Broken {
                seq: entry.seq,
                reason,
            };
        }
    }
    verifier.finish()
}

#[cfg(test)]
mod tests {
    use bb_core::event::{EntityKind, EntityRef};
    use bb_core::id::{EntityId, UserId};
    use bb_core::network::NetworkPrefix;
    use bb_core::technical::TechnicalName;
    use jiff::Timestamp;
    use proptest::prelude::*;

    use super::*;

    /// The fields a test chooses; everything else is made by [`build`].
    #[derive(Clone, Debug)]
    struct Fields {
        micros: i64,
        person: Option<[u8; 16]>,
        action: String,
        subject: Option<(String, [u8; 16])>,
        network: Option<[u8; 3]>,
    }

    fn fields() -> impl Strategy<Value = Fields> {
        (
            0_i64..4_000_000_000_000_000,
            proptest::option::of(any::<[u8; 16]>()),
            "[a-z][a-z0-9._-]{0,20}",
            proptest::option::of(("[a-z][a-z0-9]{0,5}(_[a-z0-9]{1,5}){0,2}", any::<[u8; 16]>())),
            proptest::option::of(any::<[u8; 3]>()),
        )
            .prop_map(|(micros, person, action, subject, network)| Fields {
                micros,
                person,
                action,
                subject,
                network,
            })
    }

    fn entry(seq: i64, fields: &Fields) -> SecurityLogEntry {
        SecurityLogEntry {
            seq,
            time: Timestamp::from_microsecond(fields.micros).unwrap(),
            person: fields.person.map(UserId::from_random),
            action: TechnicalName::new(&fields.action).unwrap(),
            subject: fields.subject.as_ref().map(|(kind, id)| EntityRef {
                kind: EntityKind::new(kind).unwrap(),
                id: EntityId::from_bytes(*id).unwrap(),
            }),
            network: fields.network.map(|[a, b, c]| {
                NetworkPrefix::from_address(core::net::IpAddr::from([a, b, c, 9]))
            }),
            previous_hash: START,
            hash: START,
        }
    }

    /// A correct chain of these entries, starting at `first_seq` after `previous`.
    fn build(first_seq: i64, mut previous: Digest, all: &[Fields]) -> Vec<SecurityLogEntry> {
        let mut chain = Vec::new();
        for (offset, fields) in all.iter().enumerate() {
            let mut next = entry(first_seq + i64::try_from(offset).unwrap(), fields);
            next.previous_hash = previous;
            next.hash = hash_of(&previous, &next);
            previous = next.hash;
            chain.push(next);
        }
        chain
    }

    #[test]
    fn good_example_a_correct_chain_verifies_and_reports_its_head() {
        let all: Vec<Fields> = (0..3)
            .map(|n| Fields {
                micros: n,
                person: None,
                action: "sign_in.ok".to_owned(),
                subject: None,
                network: Some([203, 0, 113]),
            })
            .collect();
        let chain = build(1, START, &all);
        let verdict = verify_all(1, START, &chain);
        assert_eq!(
            verdict,
            Verification::Valid {
                entries: 3,
                head: Some((3, chain[2].hash))
            }
        );
        assert_eq!(
            verify_all(1, START, &[]),
            Verification::Valid {
                entries: 0,
                head: None
            }
        );
    }

    #[test]
    fn the_encoding_cannot_be_confused_between_neighbouring_fields() {
        // "ab" + "c" and "a" + "bc" must not hash alike.
        let first = Fields {
            micros: 1,
            person: None,
            action: "ab".to_owned(),
            subject: Some(("c".to_owned(), [0; 16])),
            network: None,
        };
        let second = Fields {
            action: "a".to_owned(),
            subject: Some(("bc".to_owned(), [0; 16])),
            ..first.clone()
        };
        assert_ne!(
            build(1, START, &[first])[0].hash,
            build(1, START, &[second])[0].hash
        );
    }

    proptest! {
        /// A correct chain always verifies, from any starting place (also after a checkpoint).
        #[test]
        fn sec_15_a_correct_chain_verifies(
            all in proptest::collection::vec(fields(), 1..12),
            first_seq in 1_i64..5000,
            start in any::<[u8; 32]>(),
        ) {
            let chain = build(first_seq, Digest::new(start), &all);
            let verdict = verify_all(first_seq, Digest::new(start), &chain);
            let valid = matches!(verdict, Verification::Valid { .. });
            prop_assert!(valid, "{:?}", verdict);
        }

        /// Whatever is changed in any one entry, the chain no longer verifies, and it breaks at
        /// that entry or the next one.
        #[test]
        fn sec_15_any_changed_row_breaks_the_chain(
            all in proptest::collection::vec(fields(), 2..12),
            at in any::<proptest::sample::Index>(),
            what in 0_usize..9,
            replacement in fields(),
        ) {
            let mut chain = build(1, START, &all);
            let index = at.index(chain.len());
            let row = &mut chain[index];
            match what {
                0 => row.time = Timestamp::from_microsecond(row.time.as_microsecond() + 1).unwrap(),
                1 => row.person = match row.person {
                    Some(_) => None,
                    None => Some(UserId::from_random([7; 16])),
                },
                2 => row.action = TechnicalName::new(&format!("{}x", row.action)).unwrap(),
                3 => row.subject = match row.subject {
                    Some(_) => None,
                    None => replacement.subject.as_ref().map(|(kind, id)| EntityRef {
                        kind: EntityKind::new(kind).unwrap(),
                        id: EntityId::from_bytes(*id).unwrap(),
                    }).or(Some(EntityRef {
                        kind: EntityKind::new("issue").unwrap(),
                        id: EntityId::from_bytes([1; 16]).unwrap(),
                    })),
                },
                4 => row.network = match row.network {
                    Some(_) => None,
                    None => Some(NetworkPrefix::from_address(core::net::IpAddr::from([10, 0, 0, 1]))),
                },
                5 => {
                    let mut bytes = *row.hash.as_bytes();
                    bytes[0] ^= 1;
                    row.hash = Digest::new(bytes);
                }
                6 => {
                    let mut bytes = *row.previous_hash.as_bytes();
                    bytes[31] ^= 1;
                    row.previous_hash = Digest::new(bytes);
                }
                7 => {
                    // Take an entry out of the middle or the start (the end cannot be seen).
                    if index + 1 == chain.len() {
                        chain.remove(0);
                    } else {
                        chain.remove(index);
                    }
                }
                _ => {
                    // Put an entry in twice.
                    let copy = chain[index].clone();
                    chain.insert(index, copy);
                }
            }
            let verdict = verify_all(1, START, &chain);
            let broken = matches!(verdict, Verification::Broken { .. });
            prop_assert!(broken, "{:?}", verdict);
        }
    }
}
