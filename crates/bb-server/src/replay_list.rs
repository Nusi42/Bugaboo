//! The replay list in the data folder (`privacy/replay.list`, ADR-0044, decision 8): the file
//! behind `bb_app::privacy::replay::ReplayFile`. Which lines go in is decided by `bb-app`; this
//! file adds them to the text that is there, unchanged, and writes the whole to a private file that
//! is flushed to the disk and renamed into place, so a crash leaves the old list or the new one.
//! The work on the file runs on a thread for blocking work, not on a worker of the server.

use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use bb_app::ports::BoxFuture;
use bb_app::privacy::replay::{ReplayFile, appended};

use crate::data_folder::{Existing, create_private, write_private_with};

/// The replay list of one data folder.
pub(crate) struct ReplayListOnDisk {
    path: PathBuf,
    /// One change of the list at a time, so two that end at the same moment both keep their lines.
    writing: Arc<Mutex<()>>,
}

impl ReplayListOnDisk {
    /// The list at `path`, made with its folder at the first line.
    pub(crate) fn new(path: PathBuf) -> Self {
        Self {
            path,
            writing: Arc::new(Mutex::new(())),
        }
    }
}

/// Adds `lines` to the list at `path`, holding `writing`.
fn append_to(path: &std::path::Path, writing: &Mutex<()>, lines: &str) -> std::io::Result<()> {
    let _one_at_a_time = writing
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let text = match std::fs::read_to_string(path) {
        Ok(text) => text,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => String::new(),
        Err(error) => return Err(error),
    };
    let whole = appended(&text, lines)?;
    if let Some(folder) = path.parent() {
        create_private(folder).map_err(|error| std::io::Error::other(error.message()))?;
    }
    write_private_with(path, Existing::Replace, |file| {
        std::io::Write::write_all(file, whole.as_bytes())?;
        file.sync_all()
    })
}

impl ReplayFile for ReplayListOnDisk {
    fn append<'a>(&'a self, lines: &'a str) -> BoxFuture<'a, std::io::Result<()>> {
        let (path, writing, lines) = (
            self.path.clone(),
            Arc::clone(&self.writing),
            lines.to_owned(),
        );
        Box::pin(async move {
            tokio::task::spawn_blocking(move || append_to(&path, &writing, &lines))
                .await
                .map_err(std::io::Error::other)?
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A line of a switch-off of a made-up person, as the list writes it.
    fn a_line(seed: u8) -> String {
        format!("2026-11-02T10:00:00Z switch-off 0195f7a0-0000-4000-8000-00000000000{seed}\n")
    }

    #[tokio::test]
    async fn priv_04_the_replay_list_keeps_every_line_across_writes() -> Result<(), std::io::Error>
    {
        let folder = tempfile::tempdir()?;
        let path = folder.path().join("privacy").join("replay.list");
        let list = ReplayListOnDisk::new(path.clone());
        let one = a_line(1);
        list.append(&one).await?;
        let two = a_line(2);
        list.append(&two).await?;
        assert_eq!(std::fs::read_to_string(&path)?, format!("{one}{two}"));
        Ok(())
    }

    #[tokio::test]
    async fn priv_04_a_list_with_a_line_that_cannot_be_read_is_not_written_over()
    -> Result<(), std::io::Error> {
        let folder = tempfile::tempdir()?;
        let path = folder.path().join("replay.list");
        std::fs::write(&path, "not a line\n")?;
        let list = ReplayListOnDisk::new(path.clone());
        let refused = list.append(&a_line(1)).await;
        assert_eq!(
            refused.err().map(|error| error.kind()),
            Some(std::io::ErrorKind::InvalidData)
        );
        assert_eq!(std::fs::read_to_string(&path)?, "not a line\n");
        Ok(())
    }
}
