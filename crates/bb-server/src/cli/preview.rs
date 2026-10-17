//! `bugaboo preview`: the child process that makes the preview of one file (ADR-0032 §4).
//!
//! It is the only place that limits the memory of the program, so it holds the allocator of the
//! program: nothing else can reach it, and nothing else can lower its limit. The program runs with
//! that allocator without a limit (it only counts); the child limits it before it reads a byte.
//! The child runs with no runtime at all (`main` looks for it first), so the limit counts the
//! memory of one thread and not the stacks of a pool of workers.

use std::process::ExitCode;

use bb_core::files::PreviewKind;
use bb_files::preview::{ChildAllocator, child_allocator};

/// The allocator of the program: the one of the system, wrapped so that the child of a preview can
/// limit its memory. Private to this module on purpose.
#[global_allocator]
static ALLOCATOR: ChildAllocator = child_allocator();

/// Makes the preview of the file on the standard input, writes it to the standard output, and
/// answers with the exit code that says why there is none (`PreviewRefusal`).
pub(super) fn run(kind: PreviewKind) -> ExitCode {
    let code = bb_files::preview::run(kind, &ALLOCATOR);
    u8::try_from(code).map_or(ExitCode::FAILURE, ExitCode::from)
}
