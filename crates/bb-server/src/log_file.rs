//! The log file of a server that runs as a service, which starts a new file when it is big
//! (M1-F61, ADR-0017): `logs/server.log` is written to for as long as the service runs, and the
//! job `server-log.rotate` asks this file to rotate. The file is held open by the process, so the
//! rotation renames it and opens a new one while the writers wait for the lock.
//!
//! `server.log`, `server.log.1`, … `server.log.N`: the newest old file is `.1`, the oldest that is
//! kept `.N`; the one that would be `.N+1` is deleted.

use std::fs::{File, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use bb_app::server_log::ServerLogFile;
use tracing_subscriber::fmt::MakeWriter;

/// The log file, shared between the writers of the log and the job that rotates it.
pub(crate) struct RotatingFile {
    path: PathBuf,
    file: Mutex<File>,
}

impl RotatingFile {
    /// Opens the file for appending, making it when it is not there.
    ///
    /// # Errors
    /// The error of the file system.
    pub(crate) fn open(path: &Path) -> io::Result<Arc<Self>> {
        Ok(Arc::new(Self {
            path: path.to_owned(),
            file: Mutex::new(open_appending(path)?),
        }))
    }

    fn locked(&self) -> MutexGuard<'_, File> {
        self.file.lock().unwrap_or_else(PoisonError::into_inner)
    }

    /// The name of the old file number `number`.
    fn numbered(&self, number: u32) -> PathBuf {
        let mut name = self.path.clone().into_os_string();
        name.push(format!(".{number}"));
        PathBuf::from(name)
    }
}

fn open_appending(path: &Path) -> io::Result<File> {
    OpenOptions::new().create(true).append(true).open(path)
}

impl ServerLogFile for RotatingFile {
    fn rotate(&self, max_bytes: u64, keep_files: u32) -> io::Result<bool> {
        let mut file = self.locked();
        file.flush()?;
        if file.metadata()?.len() <= max_bytes {
            return Ok(false);
        }
        let keep = keep_files.max(1);
        // The oldest goes, the others move up by one, and the current file becomes the newest.
        match std::fs::remove_file(self.numbered(keep)) {
            Err(error) if error.kind() != io::ErrorKind::NotFound => return Err(error),
            _ => {}
        }
        for number in (1..keep).rev() {
            match std::fs::rename(self.numbered(number), self.numbered(number + 1)) {
                Err(error) if error.kind() != io::ErrorKind::NotFound => return Err(error),
                _ => {}
            }
        }
        std::fs::rename(&self.path, self.numbered(1))?;
        *file = open_appending(&self.path)?;
        Ok(true)
    }
}

/// One write to the log: holds the lock of the file while the line is written.
pub(crate) struct LogWriter<'a>(MutexGuard<'a, File>);

impl Write for LogWriter<'_> {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        self.0.write(bytes)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.0.flush()
    }
}

impl<'a> MakeWriter<'a> for RotatingHandle {
    type Writer = LogWriter<'a>;

    fn make_writer(&'a self) -> LogWriter<'a> {
        LogWriter(self.0.locked())
    }
}

/// What the logger writes through: the shared file.
#[derive(Clone)]
pub(crate) struct RotatingHandle(pub(crate) Arc<RotatingFile>);

#[cfg(test)]
mod tests {
    use super::*;

    fn text_of(path: &Path) -> String {
        std::fs::read_to_string(path).unwrap_or_default()
    }

    #[test]
    fn m1_f61_a_file_that_is_not_big_is_left_alone() -> io::Result<()> {
        let folder = tempfile::tempdir()?;
        let path = folder.path().join("server.log");
        let log = RotatingFile::open(&path)?;
        writeln!(RotatingHandle(Arc::clone(&log)).make_writer(), "one line")?;
        assert!(!log.rotate(1000, 3)?);
        assert!(!folder.path().join("server.log.1").exists());
        Ok(())
    }

    #[test]
    fn m1_f61_a_big_file_becomes_the_newest_old_file_and_a_new_one_goes_on() -> io::Result<()> {
        let folder = tempfile::tempdir()?;
        let path = folder.path().join("server.log");
        let log = RotatingFile::open(&path)?;
        let handle = RotatingHandle(Arc::clone(&log));
        writeln!(handle.make_writer(), "first round")?;
        assert!(log.rotate(3, 3)?);
        writeln!(handle.make_writer(), "second round")?;
        assert!(log.rotate(3, 3)?);
        writeln!(handle.make_writer(), "third round")?;
        assert_eq!(text_of(&path), "third round\n");
        assert_eq!(
            text_of(&folder.path().join("server.log.1")),
            "second round\n"
        );
        assert_eq!(
            text_of(&folder.path().join("server.log.2")),
            "first round\n"
        );
        Ok(())
    }

    #[test]
    fn m1_f61_only_as_many_old_files_as_asked_are_kept_and_it_is_safe_to_run_again()
    -> io::Result<()> {
        let folder = tempfile::tempdir()?;
        let path = folder.path().join("server.log");
        let log = RotatingFile::open(&path)?;
        let handle = RotatingHandle(Arc::clone(&log));
        for round in 0..5 {
            writeln!(handle.make_writer(), "round {round}")?;
            assert!(log.rotate(3, 2)?);
        }
        assert!(folder.path().join("server.log.1").exists());
        assert!(folder.path().join("server.log.2").exists());
        assert!(!folder.path().join("server.log.3").exists());
        assert_eq!(text_of(&folder.path().join("server.log.1")), "round 4\n");
        assert_eq!(text_of(&folder.path().join("server.log.2")), "round 3\n");
        // The new file is empty, so another go changes nothing.
        assert!(!log.rotate(3, 2)?);
        Ok(())
    }
}
