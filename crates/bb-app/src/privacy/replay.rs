//! The replay list (ADR-0044, decision 8): what a restore must not undo. This is the one place
//! that adds lines to it: a removal (with its choice about comments), a switch-off, and an access
//! key, an AI app grant or a passkey taken back each add one line (support accesses join with
//! support access). A line holds the kind, the ID and the moment, and nothing else
//! (`bb_core::backup::replay`).
//!
//! A line is written in the change it stands for, into the database ([`ReplayStore`]), so it
//! commits with the change or not at all. Right after the commit it goes to the file of the list
//! and leaves the database; a line the file did not take waits and goes with the next write and
//! before every backup, and the security log says that it waits. The file only ever grows: the
//! new lines are added to its text as it is, written to a new file that is renamed into place
//! ([`ReplayFile`]), so a crash leaves the old list or the new one, never half of one; a list with
//! a line that cannot be read is not written over at all.

use std::fmt::Display;
use std::io;
use std::sync::Mutex;

use bb_core::backup::replay::{self, ReplayKind, ReplayLine};
use bb_core::id::UserId;
use bb_core::sign_in::SecurityAction;
use jiff::Timestamp;

use crate::auth::{AuthStore, Ctx, locked};
use crate::ports::{BoxFuture, ChangeStore};
use crate::store_macros::{reads, writes};

/// Where the replay list is kept: `privacy/replay.list` in the data folder.
pub trait ReplayFile: Send + Sync {
    /// Adds `lines` (whole lines, each ending in a new line) to the end of the list, under one
    /// lock, so two changes that end at the same moment both keep their lines. The text that is
    /// there stays as it is.
    ///
    /// # Errors
    /// The error of the file system, and [`io::ErrorKind::InvalidData`] when the list holds a line
    /// that cannot be read: such a list is left as it is for a person to look at.
    fn append<'a>(&'a self, lines: &'a str) -> BoxFuture<'a, io::Result<()>>;
}

/// Who asks for a switch-off or a removal, which decides what it checks first and whom the
/// standing names as who did it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Asked {
    /// An administrator: only an active person is switched off, only a switched-off person is
    /// removed, each only once, and never the last person who decides roles.
    ByAdministrator(UserId),
    /// The replay list after a restore (ADR-0044, decision 8): the line runs again whatever the
    /// restored data says (no switch-off before a removal, no check of who decides roles, which
    /// the change passed when it was made), and a person it was done to already is left as they
    /// are.
    ByReplay,
}

impl Asked {
    /// The administrator who asked; none for the replay, whose line names nobody.
    #[must_use]
    pub const fn by(self) -> Option<UserId> {
        match self {
            Self::ByAdministrator(by) => Some(by),
            Self::ByReplay => None,
        }
    }
}

/// The text of a list with `lines` added, when the list can be read.
///
/// # Errors
/// [`io::ErrorKind::InvalidData`] when `text` holds a line that cannot be read.
pub fn appended(text: &str, lines: &str) -> io::Result<String> {
    if replay::read(text).1 > 0 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "the replay list holds a line that cannot be read",
        ));
    }
    let mut whole = text.to_owned();
    if !whole.is_empty() && !whole.ends_with('\n') {
        whole.push('\n');
    }
    whole.push_str(lines);
    Ok(whole)
}

/// The list of a server without a data folder (the services of a test): kept in memory.
#[derive(Debug, Default)]
pub struct ReplayInMemory(Mutex<String>);

impl ReplayInMemory {
    /// The text of the list.
    #[must_use]
    pub fn text(&self) -> String {
        locked(&self.0).clone()
    }

    /// Puts `text` in place of the list, as a test needs one that cannot be read.
    pub fn replace(&self, text: &str) {
        text.clone_into(&mut locked(&self.0));
    }
}

impl ReplayFile for ReplayInMemory {
    fn append<'a>(&'a self, lines: &'a str) -> BoxFuture<'a, io::Result<()>> {
        Box::pin(async move {
            let mut text = locked(&self.0);
            *text = appended(&text, lines)?;
            Ok(())
        })
    }
}

/// The lines of the replay list that wait in the database for the file.
pub trait ReplayStore: ChangeStore + 'static {
    writes! {
        /// Writes `lines` in the change they stand for; a line that waits already stays once.
        fn wait_to_replay(lines: &[ReplayLine]) -> ();
    }

    reads! {
        /// Every line that waits for the file, oldest first.
        fn waiting_replay_lines() -> Vec<String>;
        /// Forgets the lines the file now holds. It deletes rows outside a change, and with no
        /// event, on purpose: a line is a copy of a change that has its event already, and the
        /// file holds it now; a row left by a failure only makes the line go to the file twice,
        /// which a restore merges.
        fn replay_lines_written(lines: &[String]) -> ();
    }
}

/// One line of the list: `kind` happened to `id` at `moment`, with the choice of a removal.
#[must_use]
pub fn line(
    moment: Timestamp,
    kind: ReplayKind,
    id: impl Display,
    choice: Option<&str>,
) -> ReplayLine {
    ReplayLine {
        moment,
        kind,
        id: id.to_string(),
        choice: choice.map(str::to_owned),
    }
}

/// Writes a line for each of `ids`, of `kind`, taken back at `now` (an access key, an AI app
/// grant or a passkey), inside the change that takes them back.
///
/// # Errors
/// The error of the store.
pub(crate) async fn taken_back_in<S: AuthStore, I: Display>(
    transaction: &mut S::Transaction,
    now: Timestamp,
    kind: ReplayKind,
    ids: impl IntoIterator<Item = I>,
) -> Result<(), S::Error> {
    let lines: Vec<ReplayLine> = ids
        .into_iter()
        .map(|id| line(now, kind, id, None))
        .collect();
    if lines.is_empty() {
        return Ok(());
    }
    S::wait_to_replay(transaction, &lines).await
}

impl<S: AuthStore> Ctx<'_, S> {
    /// Moves the lines that wait in the database into the file of the list: at server start (the
    /// lines a stop left behind), after every change that wrote one, and before every backup. A
    /// list that cannot be written does not undo the change, which stands: its lines wait for the
    /// next try, and the security log and the server log say so, so an administrator learns that
    /// a restore could undo it until then.
    pub async fn write_replay_list(&self) {
        let waiting = match self.store().waiting_replay_lines().await {
            Ok(waiting) if waiting.is_empty() => return,
            Ok(waiting) => waiting,
            Err(error) => {
                tracing::error!(
                    cause = %S::describe(&error),
                    "the lines that wait for the replay list could not be read"
                );
                self.note(SecurityAction::ReplayListNotWritten, None, None)
                    .await;
                return;
            }
        };
        let text = waiting.iter().fold(String::new(), |mut text, line| {
            text.push_str(line);
            text.push('\n');
            text
        });
        let failure = match self.services.replay.append(&text).await {
            Ok(()) => match self.store().replay_lines_written(&waiting).await {
                Ok(()) => return,
                // The file has the lines; they are added again at the next write, and a restore
                // merges lines that are there twice.
                Err(error) => S::describe(&error),
            },
            Err(error) => error.to_string(),
        };
        tracing::error!(
            cause = %failure,
            lines = waiting.len(),
            "the replay list could not be written; its lines wait in the database"
        );
        self.note(SecurityAction::ReplayListNotWritten, None, None)
            .await;
    }
}
