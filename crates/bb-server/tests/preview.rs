//! `bugaboo preview`, the child process that creates the preview of a file (ATT-04, ADR-0032 §4),
//! started as the server starts it: the real program, with limits on pixels, size and time.
//!
//! A file that is refused, broken, too slow or that makes the child end badly gets no preview, and
//! the next file gets its preview as if nothing had happened.

// Dependencies of the program that this test does not use itself (`unused_crate_dependencies`).
use anyhow as _;
use argon2 as _;
use axum as _;
use base64ct as _;
use bb_ai as _;
use bb_api as _;
use bb_api_words as _;
use bb_app as _;
use bb_auth as _;
use bb_dev_words as _;
use bb_jobs as _;
use bb_load_check as _;
use bb_mcp as _;
use bb_notify as _;
use bb_policy as _;
use bb_store as _;
use bb_testing as _;
use clap as _;
use getrandom as _;
use h2 as _;
use jiff as _;
#[cfg(windows)]
use known_folders as _;
#[cfg(unix)]
use nix as _;
use rustls as _;
use serde_json as _;
use sha2 as _;
use tokio_rustls as _;
use toml as _;
use tower as _;
use tracing as _;
use tracing_subscriber as _;
use webauthn_authenticator_rs as _;
#[cfg(windows)]
use windows_service as _;
use zeroize as _;
use zip as _;

#[cfg(test)] // Clippy lets tests unwrap only inside test modules (clippy.toml).
mod preview {
    use std::time::Duration;

    use bb_app::context::SystemIds;
    use bb_app::files::FileStore as _;
    use bb_core::files::{LinkTarget, PreviewKind, PreviewStatus};
    use bb_core::id::UploadId;
    use bb_core::security_log::Digest;
    use bb_files::fixtures::{Format, PDF, pdf_bomb, picture, pretending_png};
    use bb_files::preview::{MAX_CHILD_MEMORY_BYTES, PreviewOutcome, TIME_LIMIT};
    use bb_files::{DiskFiles, PreviewCommand, PreviewRefusal};

    /// The exit code of a Windows program that aborted (a refused allocation): the fast-fail code
    /// 0xC0000409 as the signed number the system gives back.
    #[cfg(windows)]
    const WINDOWS_ABORT: i32 = -1_073_740_791;
    #[cfg(not(windows))]
    const WINDOWS_ABORT: i32 = 0;

    /// The program the server starts for a preview.
    const PROGRAM: &str = env!("CARGO_BIN_EXE_bugaboo");

    /// The files of a new installation, previews made by the real program.
    fn files_with(command: PreviewCommand) -> (tempfile::TempDir, DiskFiles) {
        let folder = tempfile::tempdir().unwrap();
        let files = DiskFiles::new(folder.path(), command);
        (folder, files)
    }

    fn files() -> (tempfile::TempDir, DiskFiles) {
        files_with(PreviewCommand::new(PROGRAM, &["preview"]))
    }

    /// Puts the bytes into the store the way an upload does; the hash.
    async fn stored(files: &DiskFiles, bytes: &[u8]) -> Digest {
        let upload = UploadId::generate(&mut SystemIds).unwrap();
        files.begin_upload(upload).await.unwrap();
        files.write_part(upload, 0, bytes.to_vec()).await.unwrap();
        let hash = files.inspect_upload(upload).await.unwrap().hash;
        files.keep_upload(upload, hash).await.unwrap();
        hash
    }

    async fn preview_png(files: &DiskFiles, hash: Digest) -> Option<Vec<u8>> {
        let mut body = files.open(hash, LinkTarget::Preview).await.unwrap()?;
        let mut all = Vec::new();
        while let Some(chunk) = body.reader.next_chunk().await.unwrap() {
            all.extend(chunk);
        }
        Some(all)
    }

    #[tokio::test]
    async fn att_04_the_program_creates_the_preview_of_a_picture_and_of_a_pdf() {
        let (_folder, files) = files();
        for (kind, bytes) in [
            (PreviewKind::Image, picture(900, 300, Format::Png)),
            (PreviewKind::Pdf, PDF.to_vec()),
        ] {
            let hash = stored(&files, &bytes).await;
            assert_eq!(
                files.create_preview(hash, kind).await.unwrap(),
                PreviewStatus::Ready
            );
            let png = preview_png(&files, hash).await.unwrap();
            assert!(png.starts_with(&[0x89, b'P', b'N', b'G']));
        }
    }

    #[tokio::test]
    async fn att_04_a_decompression_bomb_gets_no_preview_and_the_server_goes_on() {
        let (folder, files) = files();
        let bomb = stored(&files, &pretending_png(100_000, 100_000)).await;
        assert_eq!(
            files
                .create_preview(bomb, PreviewKind::Image)
                .await
                .unwrap(),
            PreviewStatus::Without
        );
        assert!(preview_png(&files, bomb).await.is_none());
        // Nothing is left of the try, and the next file is previewed as usual.
        let work = folder.path().join("previews").join("work");
        assert_eq!(std::fs::read_dir(&work).map_or(0, Iterator::count), 0);
        let next = stored(&files, &picture(50, 50, Format::Jpeg)).await;
        assert_eq!(
            files
                .create_preview(next, PreviewKind::Image)
                .await
                .unwrap(),
            PreviewStatus::Ready
        );
    }

    #[tokio::test]
    async fn att_04_a_pdf_bomb_costs_the_child_its_memory_limit_and_the_preview_only() {
        let (folder, files) = files();
        // A PDF of a few megabytes whose picture unpacks to one and a half times what the child
        // may use: the child cannot get the memory, ends, and the server reads that as no preview.
        let bytes = pdf_bomb(MAX_CHILD_MEMORY_BYTES + MAX_CHILD_MEMORY_BYTES / 2);
        let bomb = stored(&files, &bytes).await;
        assert_eq!(
            files.create_preview(bomb, PreviewKind::Pdf).await.unwrap(),
            PreviewStatus::Without
        );
        // How it ended is read from the outcome of the child, not from how long it took, which a
        // busy computer stretches: the same bomb with a clock long enough that only the memory
        // can stop it. A refused allocation aborts the child: no exit code on Unix (a signal),
        // the abort code of Windows there.
        let input = folder.path().join("bomb.pdf");
        std::fs::write(&input, &bytes).unwrap();
        let outcome = bb_files::preview::create(
            &PreviewCommand::new(PROGRAM, &["preview"]).within(TIME_LIMIT * 10),
            PreviewKind::Pdf,
            &input,
            &folder.path().join("bomb.png"),
        )
        .await
        .unwrap();
        let aborted = if cfg!(windows) {
            PreviewOutcome::Without(Some(WINDOWS_ABORT))
        } else {
            PreviewOutcome::Without(None)
        };
        assert_eq!(outcome, aborted, "it ended by its limit, not by the clock");
        assert!(preview_png(&files, bomb).await.is_none());
        let work = folder.path().join("previews").join("work");
        assert_eq!(std::fs::read_dir(&work).map_or(0, Iterator::count), 0);
        // The server goes on: the next file is previewed as usual.
        let next = stored(&files, PDF).await;
        assert_eq!(
            files.create_preview(next, PreviewKind::Pdf).await.unwrap(),
            PreviewStatus::Ready
        );
    }

    #[tokio::test]
    async fn att_04_a_big_picture_and_a_page_still_get_their_preview_under_the_limits() {
        // A picture of 24 megapixels (a good camera) is decoded in about 100 MB: far under the
        // limit of the heap and, on Linux, of the address space (`RLIMIT_AS`), which the child
        // sets before it reads a byte and which counts one thread, not a pool of workers.
        let (_folder, files) = files();
        let big = stored(&files, &picture(6000, 4000, Format::Png)).await;
        assert_eq!(
            files.create_preview(big, PreviewKind::Image).await.unwrap(),
            PreviewStatus::Ready
        );
        let page = stored(&files, PDF).await;
        assert_eq!(
            files.create_preview(page, PreviewKind::Pdf).await.unwrap(),
            PreviewStatus::Ready
        );
    }

    /// The program that plays a child for the tests below: this test program itself, started with
    /// one of its ignored tests, which only run so.
    fn this_test_as_a_child(test: &str) -> PreviewCommand {
        let program = std::env::current_exe().unwrap();
        PreviewCommand::new(
            program,
            &[
                "--ignored",
                "--exact",
                &format!("preview::{test}"),
                "--nocapture",
            ],
        )
    }

    /// A child that never reads what it is sent and goes on for half a minute.
    #[test]
    #[ignore = "plays a child of the tests that start it, never run by itself"]
    fn child_that_never_reads() {
        std::thread::sleep(Duration::from_secs(30));
    }

    /// A child that shuts itself in as `bugaboo preview` does, and then tries to get out: it starts
    /// a program, opens a socket of each kind and writes a file, and says on its standard output
    /// how far it got, line by line, so the test sees that it reached every try.
    #[test]
    #[ignore = "plays a child of the tests that start it, never run by itself"]
    fn child_that_tries_to_get_out() {
        use std::io::Write as _;
        let shut_in = bb_files::preview::lock_down().unwrap();
        let mut out = std::io::stdout().lock();
        let mut say = |line: String| {
            writeln!(out, "{line}").unwrap();
            out.flush().unwrap();
        };
        say(format!("shut-in={shut_in:?}"));
        let started = std::process::Command::new(std::env::current_exe().unwrap())
            .arg("--list")
            .output()
            .is_ok();
        say(format!("started={started}"));
        let tcp = std::net::TcpListener::bind("127.0.0.1:0").is_ok();
        say(format!("tcp={tcp}"));
        let udp = std::net::UdpSocket::bind("127.0.0.1:0").is_ok();
        say(format!("udp={udp}"));
        #[cfg(unix)]
        {
            let unix = std::os::unix::net::UnixDatagram::unbound().is_ok();
            say(format!("unix={unix}"));
        }
        say("writing".to_owned());
        // Writing a byte past the limit of the size of files ends the process (`SIGXFSZ`): then
        // nothing more is said.
        let wrote = std::fs::write(std::env::temp_dir().join("bugaboo-got-out.txt"), b"x").is_ok();
        say(format!("wrote={wrote}"));
    }

    /// On Linux the child of a preview writes no file, opens no socket and starts no program,
    /// whatever a crafted file makes it run (ADR-0032 §4): the limits of the system hold also where
    /// the kernel has no Landlock, and the child must have reached every try for the test to count.
    /// Root may start programs and lift its own limits, so starting a program is judged only for
    /// any other account; the limits of open descriptors and of the size of files hold for root too.
    #[cfg(target_os = "linux")]
    #[test]
    fn att_04_the_child_of_a_preview_writes_no_file_and_starts_no_program() {
        let output = std::process::Command::new(std::env::current_exe().unwrap())
            .args([
                "--ignored",
                "--exact",
                "preview::child_that_tries_to_get_out",
                "--nocapture",
            ])
            .output()
            .unwrap();
        let said = String::from_utf8_lossy(&output.stdout);
        for reached in [
            "shut-in=Some(",
            "started=",
            "tcp=",
            "udp=",
            "unix=",
            "writing",
        ] {
            assert!(
                said.contains(reached),
                "the child did not reach {reached}: {said}"
            );
        }
        for refused in ["tcp=false", "udp=false", "unix=false"] {
            assert!(said.contains(refused), "{refused}: {said}");
        }
        assert!(!said.contains("wrote=true"), "{said}");
        if nix::unistd::geteuid().as_raw() != 0 {
            assert!(said.contains("started=false"), "{said}");
        }
    }

    /// A child that says it was shut in only partly, as an older kernel makes it, and ends.
    #[test]
    #[ignore = "plays a child of the tests that start it, never run by itself"]
    fn child_that_is_shut_in_partly() {
        use std::io::Write as _;
        let report = bb_files::preview::shut_in_report(bb_core::files::ShutIn::Partly);
        std::io::stderr().write_all(report.as_bytes()).unwrap();
    }

    /// The server reads how far the kernel shut a child in from what it says on its standard error,
    /// keeps the last answer for the status page, and does so for the real program as well: on
    /// Linux it always tells, on other systems it has nothing to tell.
    #[tokio::test]
    async fn att_04_the_server_learns_how_far_the_child_of_a_preview_is_shut_in() {
        let folder = tempfile::tempdir().unwrap();
        let input = folder.path().join("small.png");
        std::fs::write(&input, picture(20, 20, Format::Png)).unwrap();
        let output = folder.path().join("small-preview.png");
        let partly = this_test_as_a_child("child_that_is_shut_in_partly");
        let seen = partly.shut_in_seen();
        assert_eq!(seen.last(), None);
        bb_files::preview::create(&partly, PreviewKind::Image, &input, &output)
            .await
            .unwrap();
        assert_eq!(seen.last(), Some(bb_core::files::ShutIn::Partly));
        let real = PreviewCommand::new(PROGRAM, &["preview"]);
        let outcome = bb_files::preview::create(&real, PreviewKind::Image, &input, &output)
            .await
            .unwrap();
        assert_eq!(outcome, PreviewOutcome::Created);
        assert_eq!(
            real.shut_in_seen().last().is_some(),
            cfg!(target_os = "linux")
        );
    }

    /// A child that writes without end, as far as the other side reads.
    #[test]
    #[ignore = "plays a child of the tests that start it, never run by itself"]
    fn child_that_floods() {
        use std::io::Write as _;
        let chunk = vec![b'x'; 64 * 1024];
        let mut out = std::io::stdout().lock();
        // Never more than a gigabyte, so a child that is not stopped ends by itself.
        for _ in 0..16_384 {
            if out.write_all(&chunk).is_err() {
                return;
            }
        }
    }

    #[tokio::test]
    async fn att_04_a_child_that_never_reads_its_input_is_stopped_by_the_time_and_costs_the_preview_only()
     {
        // Six megabytes do not fit in a pipe, so the server is still writing when time runs out.
        let command = this_test_as_a_child("child_that_never_reads").within(Duration::from_secs(1));
        let (folder, files) = files_with(command);
        let big = stored(&files, &vec![7_u8; 6 * 1024 * 1024]).await;
        let started = std::time::Instant::now();
        assert_eq!(
            files.create_preview(big, PreviewKind::Image).await.unwrap(),
            PreviewStatus::Without
        );
        assert!(
            started.elapsed() < Duration::from_secs(10),
            "it was stopped, not waited for"
        );
        assert!(preview_png(&files, big).await.is_none());
        let work = folder.path().join("previews").join("work");
        assert_eq!(std::fs::read_dir(&work).map_or(0, Iterator::count), 0);
    }

    #[tokio::test]
    async fn att_04_a_child_that_floods_its_output_is_cut_off_and_costs_the_preview_only() {
        let (folder, files) = files_with(this_test_as_a_child("child_that_floods"));
        let small = stored(&files, &picture(20, 20, Format::Png)).await;
        let started = std::time::Instant::now();
        assert_eq!(
            files
                .create_preview(small, PreviewKind::Image)
                .await
                .unwrap(),
            PreviewStatus::Without
        );
        // The server read a few megabytes and closed the pipe: it did not take a gigabyte.
        assert!(
            started.elapsed() < Duration::from_secs(10),
            "it was cut off, not waited for"
        );
        assert!(preview_png(&files, small).await.is_none());
        let work = folder.path().join("previews").join("work");
        assert_eq!(std::fs::read_dir(&work).map_or(0, Iterator::count), 0);
    }

    #[tokio::test]
    async fn att_04_a_broken_pdf_gets_no_preview() {
        let (_folder, files) = files();
        let cut = stored(&files, &PDF[..PDF.len() / 2]).await;
        assert_eq!(
            files.create_preview(cut, PreviewKind::Pdf).await.unwrap(),
            PreviewStatus::Without
        );
        let whole = stored(&files, PDF).await;
        assert_eq!(
            files.create_preview(whole, PreviewKind::Pdf).await.unwrap(),
            PreviewStatus::Ready
        );
    }

    #[tokio::test]
    async fn att_04_a_child_that_ends_badly_or_takes_too_long_costs_the_preview_only() {
        let hash_and_files = |command| async move {
            let (folder, files) = files_with(command);
            let hash = stored(&files, &picture(50, 50, Format::Png)).await;
            (folder, files, hash)
        };
        // A child that does not understand what it is asked ends with an error: no preview.
        let (_folder, files, hash) =
            hash_and_files(PreviewCommand::new(PROGRAM, &["no-such-subcommand"])).await;
        assert_eq!(
            files
                .create_preview(hash, PreviewKind::Image)
                .await
                .unwrap(),
            PreviewStatus::Without
        );
        // A child that takes longer than it may is stopped.
        let (_folder, files, hash) =
            hash_and_files(PreviewCommand::new(PROGRAM, &["preview"]).within(Duration::ZERO)).await;
        assert_eq!(
            files
                .create_preview(hash, PreviewKind::Image)
                .await
                .unwrap(),
            PreviewStatus::Without
        );
        // A program that is not there cannot be started: that is a failure of the job, which the
        // queue tries again, and not a verdict on the file.
        let (_folder, files, hash) =
            hash_and_files(PreviewCommand::new("no-such-program-here", &[])).await;
        assert!(
            files
                .create_preview(hash, PreviewKind::Image)
                .await
                .is_err()
        );
    }

    #[test]
    fn att_04_the_program_takes_the_file_on_stdin_and_answers_with_the_preview_or_a_code() {
        use std::io::Write as _;
        let run = |kind: &str, bytes: &[u8]| {
            let mut child = std::process::Command::new(PROGRAM)
                .args(["preview", kind])
                .env_clear()
                .stdin(std::process::Stdio::piped())
                .stdout(std::process::Stdio::piped())
                .stderr(std::process::Stdio::null())
                .spawn()
                .unwrap();
            // A child that refuses early may not read it all.
            child.stdin.take().unwrap().write_all(bytes).ok();
            let done = child.wait_with_output().unwrap();
            (done.status.code(), done.stdout)
        };
        let (code, sent) = run("image", &pretending_png(100_000, 100_000));
        assert_eq!(code, Some(PreviewRefusal::TooManyPixels.exit_code()));
        assert!(sent.is_empty());
        let (code, sent) = run("pdf", b"not a pdf");
        assert_eq!(code, Some(PreviewRefusal::Unreadable.exit_code()));
        assert!(sent.is_empty());
        let (code, sent) = run("image", &picture(20, 20, Format::Png));
        assert_eq!(code, Some(0));
        assert!(sent.starts_with(&[0x89, b'P', b'N', b'G']));
    }
}
