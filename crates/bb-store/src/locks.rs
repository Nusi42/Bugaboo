//! The advisory locks of the program: their kinds and numbers, the one order they are taken in, the
//! guard that keeps the order, and the key a lock gets from the ID of a record (ADR-0003).

use crate::error::StoreError;

/// The kinds of advisory lock of the program, the one home of their numbers: PostgreSQL takes
/// the two-key form `pg_advisory_xact_lock(class, key)`, so locks of different kinds never meet.
///
/// **The order.** A change that takes more than one lock takes them in this order, and no other:
/// `Policy`, `TagsAndLabels`, `Workflows`, `IssueCodes`, then the others by their numbers. Two
/// changes that take two of them in different orders can each hold one and wait for the other
/// (more so behind an exclusive taker such as a change of the workflows or of the offers, which
/// waits for every shared one and is waited for by every new one). Writing an issue takes the
/// workflows (shared) before the codes of its project, and so does moving one; a change of an issue
/// type that retires it takes the policy and then the workflows (exclusive), so an issue of the
/// type is never created after the retire has looked.
///
/// The order is kept by [`Held`], which every transaction carries and which refuses a lower class
/// than one already taken (in debug and test builds), also on SQLite, where a lock does nothing and
/// a wrong order would otherwise show up only as a PostgreSQL deadlock under load. A class is added
/// to [`LockClass::rank`] where it belongs in the order.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(i32)]
pub(crate) enum LockClass {
    /// Processes that migrate one after the other (key 0).
    Migration = 1,
    /// Changes of roles and rows that could leave no administrator (key 0).
    Policy = 2,
    /// The codes of the issues of one project (key: four bytes of the project's ID).
    IssueCodes = 3,
    /// Changes of which issue sits under which (key 0): two changes at the same moment cannot
    /// make a circle between them.
    IssueParents = 4,
    /// Adding relations (key 0): two relations added at the same moment cannot both pass the
    /// limit of relations of an issue.
    IssueRelations = 5,
    /// Changes of the names of tags, labels and groups of labels (key 0): two changes at the same
    /// moment cannot both take a name, and a *Pick one* group is judged on what is stored.
    TagsAndLabels = 6,
    /// Changes of the handles of one person (key: four bytes of the person's ID): two changes at
    /// the same moment cannot both pass the rule of how often a handle changes.
    Handles = 7,
    /// The workflows that count (key 0): a change to workflows takes it alone, and an issue that is
    /// written with a status takes it together with the others who do, so an issue is never
    /// written into a status that a workflow being published drops.
    Workflows = 8,
    /// Starting uploads (key: four bytes of the person's ID): two uploads of one person started at
    /// the same moment cannot both pass the limits of what a person may have open.
    Uploads = 9,
    /// Finishing an upload of an issue (key: four bytes of the issue's ID): two finishes at the same
    /// moment cannot both take the last place for a file of the issue.
    IssueFiles = 10,
}

impl LockClass {
    /// The place of the class in the order, lowest first. Two changes that take two classes in
    /// different orders can wait for each other, so a transaction may take a class only when no
    /// class of a higher place is held. The order is the one written above; the others follow by
    /// their numbers. A `match` without a catch-all, so a new class must be given its place here.
    pub(crate) const fn rank(self) -> u8 {
        match self {
            Self::Migration => 0,
            Self::Policy => 1,
            Self::TagsAndLabels => 2,
            Self::Workflows => 3,
            Self::IssueCodes => 4,
            Self::IssueParents => 5,
            Self::IssueRelations => 6,
            Self::Handles => 7,
            Self::Uploads => 8,
            Self::IssueFiles => 9,
        }
    }
}

/// How a lock is held: alone, or together with others who hold it shared.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Mode {
    Alone,
    Shared,
}

/// Whether a wrong order is a failure of the call. It is in a debug build, in the tests of this
/// crate, and in every build that has the `testing` feature (the test builds of the server, also
/// `cargo test --release`); the program a person runs only logs it, so a mistake that slipped
/// through never stops a change that would have worked.
const STRICT: bool = cfg!(any(debug_assertions, test, feature = "testing"));

/// The advisory locks a transaction has taken so far, each with how it holds it. Taking a class
/// that is held already (another key, or the same again) is allowed whatever else is held; a class
/// that is new has to rank above every class held; and a class held shared cannot be taken alone
/// by the same transaction, because two transactions that do that wait for each other for ever.
/// The order of keys inside one class is the caller's.
#[derive(Debug, Default)]
pub(crate) struct Held {
    taken: Vec<(LockClass, Mode)>,
}

impl Held {
    /// Notes that the transaction takes `class` in this `mode`.
    ///
    /// # Errors
    /// `InvalidInput` when a class of a higher place is held and this one is new, or when the class
    /// is held shared and is taken alone: a bug in the code that took the locks, never a case for a
    /// person. The program only logs it (see `STRICT`).
    pub(crate) fn take(&mut self, class: LockClass, mode: Mode) -> Result<(), StoreError> {
        let held = |wanted: Mode| self.taken.contains(&(class, wanted));
        let again = held(Mode::Alone) || held(Mode::Shared);
        let lower = !again
            && self
                .taken
                .iter()
                .any(|(each, _)| each.rank() > class.rank());
        let upgrade = mode == Mode::Alone && held(Mode::Shared) && !held(Mode::Alone);
        if lower || upgrade {
            let why = if upgrade {
                "an advisory lock held shared was taken alone by the same transaction"
            } else {
                "an advisory lock was taken out of the order of `LockClass`"
            };
            if STRICT {
                return Err(StoreError::InvalidInput(why));
            }
            tracing::error!(class = ?class, "{why}");
        }
        if !self.taken.contains(&(class, mode)) {
            self.taken.push((class, mode));
        }
        Ok(())
    }
}

/// The key of a lock that is named by a record: four of the random bytes at the end of its ID, so
/// the lock of one record does not meet the lock of another (a clash only makes two wait for each
/// other for a moment).
pub(crate) fn lock_key(id: [u8; 16]) -> i32 {
    let mut tail = [0_u8; 4];
    tail.copy_from_slice(&id[12..]);
    i32::from_be_bytes(tail)
}

#[cfg(test)]
mod tests {
    use super::*;

    const ORDER: [LockClass; 10] = [
        LockClass::Migration,
        LockClass::Policy,
        LockClass::TagsAndLabels,
        LockClass::Workflows,
        LockClass::IssueCodes,
        LockClass::IssueParents,
        LockClass::IssueRelations,
        LockClass::Handles,
        LockClass::Uploads,
        LockClass::IssueFiles,
    ];

    #[test]
    fn arch_lock_order_the_classes_in_the_written_order_and_the_same_class_again_are_allowed() {
        let mut held = Held::default();
        for class in ORDER {
            assert!(held.take(class, Mode::Alone).is_ok(), "{class:?}");
            assert!(held.take(class, Mode::Alone).is_ok(), "{class:?} again");
        }
        // A class that is held is taken again even though higher ones are held.
        assert!(held.take(LockClass::Policy, Mode::Alone).is_ok());
        assert!(held.take(LockClass::IssueFiles, Mode::Alone).is_ok());
    }

    #[test]
    fn arch_lock_order_a_class_taken_after_a_higher_one_is_refused() {
        for (low, high) in ORDER.iter().zip(ORDER.iter().skip(1)) {
            for mode in [Mode::Alone, Mode::Shared] {
                let mut held = Held::default();
                held.take(*high, Mode::Alone).unwrap();
                assert!(held.take(*low, mode).is_err(), "{low:?} after {high:?}");
            }
        }
    }

    #[test]
    fn arch_lock_order_a_class_held_shared_cannot_be_taken_alone() {
        let mut held = Held::default();
        held.take(LockClass::Workflows, Mode::Shared).unwrap();
        // Shared again, with others in between, is fine; alone is the deadlock of two upgrades.
        held.take(LockClass::IssueCodes, Mode::Alone).unwrap();
        assert!(held.take(LockClass::Workflows, Mode::Shared).is_ok());
        assert!(held.take(LockClass::Workflows, Mode::Alone).is_err());
        // Alone first and shared after is not an upgrade, and alone again is not one either.
        let mut held = Held::default();
        held.take(LockClass::Workflows, Mode::Alone).unwrap();
        assert!(held.take(LockClass::Workflows, Mode::Shared).is_ok());
        assert!(held.take(LockClass::Workflows, Mode::Alone).is_ok());
    }

    #[test]
    fn arch_lock_order_a_wrong_order_fails_in_every_test_build_release_ones_included() {
        // `cargo test --release` has no debug assertions, and must not let a wrong order pass.
        const { assert!(STRICT) };
    }

    #[test]
    fn arch_lock_order_the_order_is_the_written_one_and_covers_every_class() {
        let ranks: Vec<u8> = ORDER.iter().map(|class| class.rank()).collect();
        assert_eq!(ranks, (0..10).collect::<Vec<u8>>());
        // The numbers of the classes are the ones PostgreSQL sees: none may repeat.
        let mut numbers: Vec<i32> = ORDER.iter().map(|class| *class as i32).collect();
        numbers.sort_unstable();
        numbers.dedup();
        assert_eq!(numbers.len(), ORDER.len());
    }

    #[tokio::test]
    async fn arch_lock_order_a_transaction_refuses_a_lower_class_than_one_it_holds()
    -> bb_testing::TestResult {
        bb_testing::each_database("arch_lock_order", async |db| {
            let database = crate::test_support::open(db).await?;
            let mut transaction = database.begin().await?;
            transaction.advisory_lock(LockClass::Workflows, 0).await?;
            transaction.advisory_lock(LockClass::IssueCodes, 7).await?;
            // The same class again, shared or alone, and a higher one, go on.
            transaction
                .advisory_lock_shared(LockClass::IssueCodes, 8)
                .await?;
            // A lower class is refused on SQLite too, where a lock does nothing.
            assert!(matches!(
                transaction.advisory_lock(LockClass::Policy, 0).await,
                Err(StoreError::InvalidInput(_))
            ));
            Ok(())
        })
        .await
    }

    #[test]
    fn arch_lock_order_the_key_is_the_last_four_bytes_of_the_id() {
        let mut id = [0_u8; 16];
        id[12..].copy_from_slice(&[0, 0, 1, 2]);
        assert_eq!(lock_key(id), 258);
    }
}
