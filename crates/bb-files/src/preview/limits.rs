//! What the child of a preview holds itself to before it reads a single byte of the file
//! (ADR-0032 §4): the memory it may use. The server also stops it by time and reads only a small
//! PNG from it; this is the part the child does on its own.
//!
//! Two limits, one for each thing that can go wrong:
//!
//! - **the heap**, on every system: the allocator of the program counts what Rust code asks for
//!   and refuses what would pass [`MAX_CHILD_MEMORY_BYTES`]; a refused allocation ends the child
//!   (the standard library aborts), and the server reads that as no preview. This is all there is
//!   on Windows, where no safe way to ask the system for more exists in this program (the crates
//!   for job objects cannot set a limit on memory). It counts what Rust asks for, not the address
//!   space of the process, which also holds stacks, code and what the system maps;
//! - **the address space**, on Linux: the system itself refuses what would pass
//!   [`MAX_CHILD_ADDRESS_SPACE_BYTES`] (`RLIMIT_AS`), whoever asks, also a library that does not go
//!   through the allocator of Rust.
//!
//! On Linux the child also shuts itself in ([`lock_down`]) before it reads a byte, so code that a
//! crafted file makes it run cannot use the rights of the server's account: it may start no
//! process and no thread (`RLIMIT_NPROC` 0), write no file (`RLIMIT_FSIZE` 0) and no core dump
//! (`RLIMIT_CORE` 0), and open no new descriptor (`RLIMIT_NOFILE` at the lowest free number, so
//! only standard input, output and error stay usable): no file, no pipe and no socket of any kind,
//! so neither a TCP or UDP port nor a Unix socket, such as the one of a local PostgreSQL server,
//! can be reached. Landlock with no rule lets it open no file and, where the kernel knows it
//! (Landlock ABI 4 and later), bind or connect no TCP port. A kernel without Landlock, or with an
//! older version, keeps what it supports, and the limits of the system still hold; the child tells
//! the server how far Landlock shut it in ([`SHUT_IN_REPORT`]), and the log and the status page
//! say so when it is not all. On Windows none of this exists in this program; the child holds the
//! rights of the server's account there.

use std::alloc::System;
use std::io;

use bb_core::files::{MEGABYTE, ShutIn};
use cap::Cap;

/// The most memory the child may take from the allocator, in bytes: twice what the decoder of
/// pictures may allocate ([`MAX_DECODER_BYTES`](super::MAX_DECODER_BYTES)), so the file read into
/// memory, its copy and a picture of the biggest size all fit.
pub const MAX_CHILD_MEMORY_BYTES: u64 = 1024 * MEGABYTE;

/// The address space the system allows the child on Linux, in bytes: twice its heap, for what the
/// heap does not hold (stacks, the program, the libraries).
pub const MAX_CHILD_ADDRESS_SPACE_BYTES: u64 = 2 * MAX_CHILD_MEMORY_BYTES;

/// The allocator the program runs with: the one of the system, wrapped so that the child of a
/// preview can limit it. The server runs with no limit (a limit of every byte there is).
pub type ChildAllocator = Cap<System>;

/// The allocator for `#[global_allocator]`, with no limit yet.
#[must_use]
pub const fn child_allocator() -> ChildAllocator {
    Cap::new(System, usize::MAX)
}

/// What the child writes on its standard error once it is shut in, followed by the word of
/// [`ShutIn`] and a line break: the server reads it there, apart from the preview on standard
/// output.
pub const SHUT_IN_REPORT: &str = "bugaboo-preview-shut-in=";

/// The line a child writes when it is shut in as far as `shut_in` says.
#[must_use]
pub fn shut_in_report(shut_in: ShutIn) -> String {
    format!("{SHUT_IN_REPORT}{}\n", shut_in.code())
}

/// How far a child was shut in, from what it wrote on its standard error: the word of the first
/// report line. `None` when there is no such line or its word is not known.
#[must_use]
pub fn shut_in_in(said: &str) -> Option<ShutIn> {
    said.lines()
        .find_map(|line| line.trim_end().strip_prefix(SHUT_IN_REPORT))
        .and_then(ShutIn::from_code)
}

/// Limits this process: the heap, and on Linux the rest of [`lock_down`]. A limit that cannot be
/// set is no limit, and the child must not go on without it. The answer says how far Landlock
/// shut the child in, on Linux.
///
/// # Errors
/// The reason a limit could not be set.
pub(super) fn apply(allocator: &ChildAllocator) -> io::Result<Option<ShutIn>> {
    let heap = usize::try_from(MAX_CHILD_MEMORY_BYTES).map_err(io::Error::other)?;
    allocator
        .set_limit(heap)
        .map_err(|()| io::Error::other("more memory was in use already than the child may use"))?;
    lock_down()
}

/// The limits of the system that a child holds itself to on Linux, each as its value: the address
/// space, no new process or thread, no file written, no core dump. The limit of descriptors comes
/// last and on its own ([`lock_descriptors`]): Landlock needs a descriptor to be set up.
#[cfg(target_os = "linux")]
const SYSTEM_LIMITS: [(rlimit::Resource, u64); 4] = [
    (rlimit::Resource::AS, MAX_CHILD_ADDRESS_SPACE_BYTES),
    (rlimit::Resource::NPROC, 0),
    (rlimit::Resource::FSIZE, 0),
    (rlimit::Resource::CORE, 0),
];

/// Shuts this process in for good, before it reads a byte (see the module): the limits of the
/// system, then Landlock with no rule, so nothing of the file system and no TCP port is allowed,
/// then no new descriptor. What only a newer kernel knows is left out where the kernel is older,
/// and the answer says how far Landlock went (`None` on other systems); a call that fails is an error, and the child must
/// not go on without it. On other systems this does nothing.
///
/// # Errors
/// The reason a limit could not be set.
#[cfg(target_os = "linux")]
pub fn lock_down() -> io::Result<Option<ShutIn>> {
    use landlock::{Access as _, AccessFs, AccessNet, Ruleset, RulesetAttr as _, RulesetCreated};
    for (resource, value) in SYSTEM_LIMITS {
        rlimit::setrlimit(resource, value, value)?;
    }
    let status = Ruleset::default()
        .handle_access(AccessFs::from_all(LANDLOCK_ABI))
        .and_then(|ruleset| ruleset.handle_access(AccessNet::from_all(LANDLOCK_ABI)))
        .and_then(Ruleset::create)
        .and_then(RulesetCreated::restrict_self)
        .map_err(io::Error::other)?;
    lock_descriptors()?;
    Ok(Some(shut_in_of(&status.ruleset)))
}

/// Allows no descriptor beyond those open now (`RLIMIT_NOFILE` at the lowest free number), so no
/// file, pipe or socket of any kind (TCP, UDP, Unix) can be opened, whatever Landlock allows. The
/// lowest free number is the one a copy of standard input gets; the copy is closed again at once.
/// Standard input, output and error stay open and usable.
#[cfg(target_os = "linux")]
fn lock_descriptors() -> io::Result<()> {
    use std::os::fd::{AsFd as _, AsRawFd as _};
    let copy = io::stdin().as_fd().try_clone_to_owned()?;
    let lowest_free = u64::try_from(copy.as_raw_fd()).map_err(io::Error::other)?;
    drop(copy);
    rlimit::setrlimit(rlimit::Resource::NOFILE, lowest_free, lowest_free)
}

/// How far the kernel shut the child in, by what Landlock answered.
#[cfg(target_os = "linux")]
const fn shut_in_of(status: &landlock::RulesetStatus) -> ShutIn {
    match status {
        landlock::RulesetStatus::FullyEnforced => ShutIn::Fully,
        landlock::RulesetStatus::PartiallyEnforced => ShutIn::Partly,
        landlock::RulesetStatus::NotEnforced => ShutIn::Not,
    }
}

/// The newest version of Landlock this program asks for: its rights of files and of TCP ports.
#[cfg(target_os = "linux")]
const LANDLOCK_ABI: landlock::ABI = landlock::ABI::V5;

/// On other systems the child is not shut in (ADR-0032 §4, the accepted risk of Windows), and
/// there is nothing to tell.
///
/// # Errors
/// Never.
#[cfg(not(target_os = "linux"))]
pub const fn lock_down() -> io::Result<Option<ShutIn>> {
    Ok(None)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn att_04_the_report_of_a_child_is_read_back_and_nothing_else_is() {
        for shut_in in ShutIn::ALL {
            let said = format!("something else\n{}", shut_in_report(*shut_in));
            assert_eq!(shut_in_in(&said), Some(*shut_in));
        }
        assert_eq!(shut_in_in(""), None);
        assert_eq!(shut_in_in("thread 'main' panicked"), None);
        assert_eq!(shut_in_in(&format!("{SHUT_IN_REPORT}maybe\n")), None);
        // Only a line that starts with the report counts, and the first report.
        assert_eq!(shut_in_in(&format!("x{SHUT_IN_REPORT}fully\n")), None);
        let twice = format!(
            "{}{}",
            shut_in_report(ShutIn::Not),
            shut_in_report(ShutIn::Fully)
        );
        assert_eq!(shut_in_in(&twice), Some(ShutIn::Not));
        // A line break of Windows does not hide the word.
        assert_eq!(
            shut_in_in(&format!("{SHUT_IN_REPORT}partly\r\n")),
            Some(ShutIn::Partly)
        );
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn att_04_what_landlock_answers_is_named_as_it_is() {
        use landlock::RulesetStatus;
        assert_eq!(shut_in_of(&RulesetStatus::FullyEnforced), ShutIn::Fully);
        assert_eq!(
            shut_in_of(&RulesetStatus::PartiallyEnforced),
            ShutIn::Partly
        );
        assert_eq!(shut_in_of(&RulesetStatus::NotEnforced), ShutIn::Not);
    }
}
