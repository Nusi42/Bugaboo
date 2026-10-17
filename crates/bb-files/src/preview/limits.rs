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

use std::alloc::System;
use std::io;

use bb_core::files::MEGABYTE;
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

/// Limits this process: the heap, and on Linux the address space. A limit that cannot be set is
/// no limit, and the child must not go on without it.
///
/// # Errors
/// The reason a limit could not be set.
pub(super) fn apply(allocator: &ChildAllocator) -> io::Result<()> {
    let heap = usize::try_from(MAX_CHILD_MEMORY_BYTES).map_err(io::Error::other)?;
    allocator
        .set_limit(heap)
        .map_err(|()| io::Error::other("more memory was in use already than the child may use"))?;
    #[cfg(target_os = "linux")]
    rlimit::setrlimit(
        rlimit::Resource::AS,
        MAX_CHILD_ADDRESS_SPACE_BYTES,
        MAX_CHILD_ADDRESS_SPACE_BYTES,
    )?;
    Ok(())
}
