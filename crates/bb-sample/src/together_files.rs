//! The files of the sample (ATT-01, ATT-04): about five hundred small pictures and PDFs, generated
//! so that no two are the same, added to issues and to comments by the people who may add them, with
//! the previews the job runner makes. A picture is a PNG or a JPEG.

use bb_app::files::{
    FinishUpload, SendPartQuery, StartUploadQuery, finish_upload, send_part, start_upload,
};
use bb_core::id::{CommentId, IssueId, UploadId};
use bb_files::fixtures::{Format, varied_pdf, varied_picture};

use crate::error::{SampleError, Step};
use crate::issues::Maker;
use crate::issues_setup::{id_in, json};
use crate::together::Work;

/// The sides of a picture, in pixels: small, but big enough to have something to show.
const PICTURE_SIDE_MIN: u32 = 160;
const PICTURE_SIDE_SPAN: usize = 240;
/// The most rounds of the job runner a sample waits for: each takes five jobs, and the runner
/// stops when none is due.
const ROUNDS_MAX: usize = 100_000;

/// Whether the file with this number is one of the `percent` of every hundred: the shares come out
/// exactly, however few files there are.
fn share_of(number: usize, percent: u32) -> bool {
    let percent = usize::try_from(percent).unwrap_or(0);
    (number + 1) * percent / 100 > number * percent / 100
}

/// What a file is, so its name and its bytes are made the same way.
#[derive(Clone, Copy)]
enum Kind {
    Png,
    Jpeg,
    Pdf,
}

impl Maker<'_, '_> {
    /// Adds the files of the plan: the share on comments goes to a comment its author may add to,
    /// the rest to an issue a Team member may change.
    pub(crate) async fn add_files(&mut self, work: &mut Work) -> Result<(), SampleError> {
        let total = usize::try_from(work.counts.files).unwrap_or(0);
        let made_comments: Vec<usize> = (0..self.comments.len())
            .filter(|at| matches!(self.comments.get(*at), Some(Some(_))))
            .collect();
        let made_issues: Vec<usize> = (0..self.issues.len())
            .filter(|at| matches!(self.issues.get(*at), Some(Some(_))))
            .collect();
        if made_issues.is_empty() {
            return Err(SampleError::NotMade("issues for files".to_owned()));
        }
        for number in 0..total {
            let kind = if share_of(number, work.plan.files.pictures_percent) {
                if number % 2 == 0 {
                    Kind::Png
                } else {
                    Kind::Jpeg
                }
            } else {
                Kind::Pdf
            };
            let on_comment =
                share_of(number, work.plan.files.on_comments_percent) && !made_comments.is_empty();
            let (uploader, issue, comment) = if on_comment {
                let at = made_comments[work.random.below(made_comments.len())];
                let comment = self.comments[at]
                    .as_ref()
                    .ok_or_else(|| SampleError::NotMade(format!("comment {at}")))?;
                let issue = self.issue(self.plan.comments[at].issue)?.id;
                (comment.author, issue, Some(comment.id))
            } else {
                let at = made_issues[work.random.below(made_issues.len())];
                let uploader = self
                    .hands
                    .anyone(u32::try_from(work.random.below(1 << 16)).unwrap_or(0));
                (uploader, self.issue(at)?.id, None)
            };
            let (name, bytes) = file_of(work, kind, number);
            self.add_file(uploader, issue, comment, name, bytes).await?;
        }
        Ok(())
    }

    /// One file: start the upload, send it in one part, finish it.
    async fn add_file(
        &mut self,
        person: usize,
        issue: IssueId,
        comment: Option<CommentId>,
        name: String,
        bytes: Vec<u8>,
    ) -> Result<(), SampleError> {
        let token = self.hands.token(person).await?;
        let ctx = self.world.services.ctx(self.world.request());
        let step = Step::Files;
        self.world.tick();
        let started = json(
            start_upload(
                &ctx,
                StartUploadQuery {
                    session_token: token.clone(),
                    issue: issue.to_string(),
                    comment,
                    name,
                    size: bytes.len() as u64,
                },
            )
            .await,
            step,
        )?;
        let upload: UploadId = id_in(&started, "id")?;
        json(
            send_part(
                &ctx,
                SendPartQuery {
                    session_token: token.clone(),
                    upload,
                    offset: 0,
                    bytes,
                },
            )
            .await,
            step,
        )?;
        json(
            finish_upload(
                &ctx,
                FinishUpload {
                    session_token: token,
                    upload,
                },
            )
            .await,
            step,
        )?;
        Ok(())
    }

    /// Runs the jobs that were queued (the previews of the files, the notifications and their
    /// e-mail) until none is due, the way a server does round after round.
    pub(crate) async fn run_the_jobs(&mut self) -> Result<(), SampleError> {
        let ctx = self.world.services.ctx(self.world.request());
        for _ in 0..ROUNDS_MAX {
            self.world.tick();
            let taken = ctx
                .run_due_jobs()
                .await
                .map_err(|error| SampleError::step(Step::Jobs, &error))?;
            if taken == 0 {
                return Ok(());
            }
        }
        Err(SampleError::NotMade("the jobs were not done".to_owned()))
    }
}

/// The name and the bytes of the file with this number: picture or document, never twice the same.
fn file_of(work: &mut Work, kind: Kind, number: usize) -> (String, Vec<u8>) {
    let n = u32::try_from(number).unwrap_or(0);
    let words = &work.words.files;
    match kind {
        Kind::Png | Kind::Jpeg => {
            let stem = work
                .random
                .pick(&words.pictures)
                .cloned()
                .unwrap_or_default();
            let width =
                PICTURE_SIDE_MIN + u32::try_from(work.random.below(PICTURE_SIDE_SPAN)).unwrap_or(0);
            let height =
                PICTURE_SIDE_MIN + u32::try_from(work.random.below(PICTURE_SIDE_SPAN)).unwrap_or(0);
            let (extension, format) = match kind {
                Kind::Jpeg => ("jpg", Format::Jpeg),
                _ => ("png", Format::Png),
            };
            (
                format!("{stem}-{number:04}.{extension}"),
                varied_picture(n, width, height, format),
            )
        }
        Kind::Pdf => {
            let stem = work
                .random
                .pick(&words.documents)
                .cloned()
                .unwrap_or_default();
            (format!("{stem}-{number:04}.pdf"), varied_pdf(n))
        }
    }
}
