//! A stand-in for the C compiler and archiver of another system, used only while LINT-RUST checks
//! code for a target other than this machine's.
//!
//! Crates such as `ring` and SQLite compile C in their build scripts, and the other system's
//! compiler is rarely installed. Linting never links, so a compiler that writes an empty output
//! file is enough. The lint run starts xtask itself under the compiler names (via `cc`'s
//! `CC_<target>` / `AR_<target>` variables) with [`VARIABLE`] set.

use std::path::Path;
use std::process::ExitCode;

/// Set in the environment of the lint run; its presence switches xtask into this mode.
pub(crate) const VARIABLE: &str = "BB_XTASK_FAKE_C_COMPILER";

/// Whether this process was started as the stand-in compiler.
pub(crate) fn requested() -> bool {
    std::env::var_os(VARIABLE).is_some()
}

/// Creates the output file the compiler or archiver was asked for, and nothing else.
pub(crate) fn run() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if let Some(output) = output_file(&args)
        && std::fs::write(&output, b"").is_err()
    {
        return ExitCode::FAILURE;
    }
    ExitCode::SUCCESS
}

/// The file named by `-o <file>` / `-o<file>` (compiler) or the first `*.a` / `*.lib` argument
/// (archiver). Probes such as `--version` name no file.
fn output_file(args: &[String]) -> Option<String> {
    let mut iter = args.iter();
    while let Some(arg) = iter.next() {
        if arg == "-o" {
            return iter.next().cloned();
        }
        if let Some(rest) = arg.strip_prefix("-o")
            && !rest.is_empty()
        {
            return Some(rest.to_owned());
        }
    }
    args.iter()
        .find(|arg| {
            !arg.starts_with('-')
                && Path::new(arg)
                    .extension()
                    .is_some_and(|ext| ext == "a" || ext == "lib")
        })
        .cloned()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(list: &[&str]) -> Vec<String> {
        list.iter().map(|a| (*a).to_owned()).collect()
    }

    #[test]
    fn the_compiler_output_is_found() {
        assert_eq!(
            output_file(&args(&["-O0", "-c", "a.c", "-o", "a.o"])).as_deref(),
            Some("a.o")
        );
        assert_eq!(
            output_file(&args(&["-c", "a.c", "-oa.o"])).as_deref(),
            Some("a.o")
        );
    }

    #[test]
    fn the_archive_is_found_and_probes_name_nothing() {
        assert_eq!(
            output_file(&args(&["cq", "libx.a", "a.o"])).as_deref(),
            Some("libx.a")
        );
        assert_eq!(output_file(&args(&["--version"])), None);
    }
}
