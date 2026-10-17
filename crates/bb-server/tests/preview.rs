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
use bb_api as _;
use bb_api_words as _;
use bb_app as _;
use bb_auth as _;
use bb_dev_words as _;
use bb_jobs as _;
use bb_load_check as _;
use bb_store as _;
use bb_testing as _;
use clap as _;
use getrandom as _;
use h2 as _;
use jiff as _;
use rustls as _;
use serde_json as _;
use tokio_rustls as _;
use toml as _;
use tower as _;
use tracing as _;
use tracing_subscriber as _;
use webauthn_authenticator_rs as _;
#[cfg(windows)]
use windows_service as _;
use zeroize as _;

#[cfg(test)] // Clippy lets tests unwrap only inside test modules (clippy.toml).
mod preview {
    use std::time::Duration;

    use bb_app::context::SystemIds;
    use bb_app::files::FileStore as _;
    use bb_core::files::{LinkTarget, PreviewKind, PreviewStatus};
    use bb_core::id::UploadId;
    use bb_core::security_log::Digest;
    use bb_files::fixtures::{Format, PDF, pdf_bomb, picture, pretending_png};
    use bb_files::preview::{MAX_CHILD_MEMORY_BYTES, TIME_LIMIT};
    use bb_files::{DiskFiles, PreviewCommand, PreviewRefusal};

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
        let bomb = stored(
            &files,
            &pdf_bomb(MAX_CHILD_MEMORY_BYTES + MAX_CHILD_MEMORY_BYTES / 2),
        )
        .await;
        let started = std::time::Instant::now();
        assert_eq!(
            files.create_preview(bomb, PreviewKind::Pdf).await.unwrap(),
            PreviewStatus::Without
        );
        assert!(
            started.elapsed() < TIME_LIMIT,
            "it ended by its limit, not by the clock"
        );
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
