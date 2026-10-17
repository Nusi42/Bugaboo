//! Files on issues and comments (ATT-01…ATT-04, ATT-07, ADR-0032): what is stored, what an upload is,
//! and the limits and the words they share.
//!
//! A **file** is stored once, by the SHA-256 of its content. An **attachment** points to a file and
//! to the issue or comment it was added to. An **upload** is a file on its way: it goes in parts, so
//! an interrupted one goes on where it stopped. The numbers that name a rule are defined here, once:
//! the adapter that stores the bytes, the use cases and the routes all read them from here.

use jiff::{SignedDuration, Timestamp};

use crate::id::{AttachmentId, CommentId, IssueId, UploadId, UserId};
use crate::personal::Entity;
use crate::security_log::Digest;
use crate::settings::FILES_MAX_MEGABYTES;
use crate::text::is_invisible_format;

/// The size of one megabyte in the limits of files: the limit of a project is a whole number of
/// them.
pub const MEGABYTE: u64 = 1024 * 1024;

/// The biggest file the settings can ever allow: the upper bound of `files.max-megabytes` in
/// bytes. No limit of a project is above it, so nothing that reads a file needs to read more.
pub const BIGGEST_FILE_BYTES: u64 = match FILES_MAX_MEGABYTES.definition().bounds() {
    Some((_, most)) => most.unsigned_abs() * MEGABYTE,
    None => 0,
};

/// The most bytes one part of an upload holds; a client sends parts of this size, and the route of
/// a part refuses a body above it (M1-F30). Every other route keeps the small limit of the API.
pub const PART_BYTES: usize = 4 * 1024 * 1024;

/// The longest name of a file, in characters.
pub const NAME_MAX_CHARS: usize = 255;

/// How many uploads one person may have open (started and not finished) at a time: a stop for
/// somebody who starts uploads and never ends them, which would fill the disk.
pub const OPEN_UPLOADS_MAX: usize = 20;

/// How many bytes the open uploads of one person may declare together (the sizes they started
/// with): four of the biggest file the settings allow. Together with [`OPEN_UPLOADS_MAX`] it stops
/// a few uploads that are never finished from filling the disk before the daily clean-up.
pub const OPEN_UPLOADS_BYTES_MAX: u64 = 4 * BIGGEST_FILE_BYTES;

/// How much room the disk of the server keeps free: a part of an upload is refused
/// (`file.no_room`) when less than this would be left after it, so a full disk never takes the
/// database and the logs down with it.
pub const DISK_KEPT_FREE_BYTES: u64 = 1024 * MEGABYTE;

/// How long an unfinished upload is kept after its last part (ADR-0032): then it is deleted.
pub const UNFINISHED_KEPT: SignedDuration = SignedDuration::from_hours(24);

/// How many files one issue holds at most, so the list of its files stays short enough to send at
/// once and a full issue answers with a clear refusal (`limit.reached`).
pub const ATTACHMENTS_PER_ISSUE_MAX: usize = 200;

/// How long a deleted attachment can still be taken out of the trash (ADR-0032 §5); after that it
/// is gone for good, and its file with it when nothing else points to it.
pub const TRASH_KEPT: SignedDuration = SignedDuration::from_hours(30 * 24);

/// How long a signed link works (ADR-0032, ATT-07).
pub const LINK_LIFETIME: SignedDuration = SignedDuration::from_mins(5);

/// The type a file has when its content says nothing more: the bytes are some data.
pub const UNKNOWN_MEDIA_TYPE: &str = "application/octet-stream";

/// The type of every preview: a small PNG, the one safe image type the preview process writes
/// (ADR-0032 §4).
pub const PREVIEW_MEDIA_TYPE: &str = "image/png";

/// The type of a file, `type/subtype` in lower case (`image/png`): the real one, read from the
/// content, never the one the browser or the name claims (ATT-03).
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct MediaType(String);

impl MediaType {
    /// The type of a file whose content says nothing more.
    #[must_use]
    pub fn unknown() -> Self {
        Self(UNKNOWN_MEDIA_TYPE.to_owned())
    }

    /// The type written as `text`, if it is `type/subtype` of letters, digits and `.+-`.
    #[must_use]
    pub fn new(text: &str) -> Option<Self> {
        let (kind, subtype) = text.split_once('/')?;
        let word = |part: &str| {
            !part.is_empty()
                && part.len() <= 127
                && part.bytes().all(|byte| {
                    byte.is_ascii_lowercase() || byte.is_ascii_digit() || b".+-".contains(&byte)
                })
        };
        (word(kind) && word(subtype)).then(|| Self(text.to_owned()))
    }

    /// The type as text.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

// Letters and digits that name a kind of data; nothing in them says anything about a person.
impl crate::personal::NotPersonal for MediaType {}

words! {
    /// What a signed link opens: the file itself or its preview.
    pub enum LinkTarget {
        /// The file, to be downloaded.
        File => "file",
        /// The small picture made of an image or of the first page of a PDF.
        Preview => "preview",
    }
}

words! {
    /// Where the preview of a file is.
    pub enum PreviewStatus {
        /// A job will make it.
        Pending => "pending",
        /// It is there.
        Ready => "ready",
        /// The file is of a type that has none, or the job could not make one (a file that is too
        /// big, takes too long or crashes the child gets none, ADR-0032).
        Without => "without",
    }
}

words_in_api!(
    PreviewStatus,
    "Where the preview of a file is: on its way, there, or none (a type without one, or a file the preview could not be made of)."
);

words! {
    /// The kinds of file that have a preview.
    pub enum PreviewKind {
        /// A picture the `image` library reads.
        Image => "image",
        /// A PDF: the preview is its first page.
        Pdf => "pdf",
    }
}

impl PreviewKind {
    /// The kind of preview a file of this type gets, if it gets one (ADR-0032): pictures that
    /// browsers show everywhere, and PDF. Nothing else, and never SVG or HTML.
    #[must_use]
    pub fn of_media_type(media_type: &MediaType) -> Option<Self> {
        match media_type.as_str() {
            "image/png" | "image/jpeg" | "image/gif" | "image/webp" => Some(Self::Image),
            "application/pdf" => Some(Self::Pdf),
            _ => None,
        }
    }
}

/// The name of a file as it is kept: the last part of what the browser sent (a path is cut off,
/// both `/` and `\` count), without control or format characters, without the dots and spaces at
/// its end (Windows drops them, so `run.bat.` is `run.bat` there), at most [`NAME_MAX_CHARS`]
/// long. `None` when nothing is left.
#[must_use]
pub fn clean_file_name(sent: &str) -> Option<String> {
    let last = sent.rsplit(['/', '\\']).next().unwrap_or(sent);
    if last
        .chars()
        .any(|c| c.is_control() || is_invisible_format(c))
    {
        return None;
    }
    let name = last.trim().trim_end_matches(['.', ' ', '\t']);
    if name.is_empty() || name == "." || name == ".." || name.chars().count() > NAME_MAX_CHARS {
        return None;
    }
    Some(name.to_owned())
}

/// The part after the last dot of a name in lower case (`Report.PDF` is `pdf`), if there is one.
#[must_use]
pub fn name_extension(name: &str) -> Option<String> {
    let (stem, extension) = name.rsplit_once('.')?;
    (!stem.is_empty() && !extension.is_empty()).then(|| extension.to_lowercase())
}

/// A file as it is stored: once, by the hash of its content (ATT-02).
#[derive(Clone, Debug, PartialEq, Eq, Entity)]
pub struct StoredFile {
    /// The SHA-256 of the content: the name of the file in the store.
    #[not_personal]
    pub hash: Digest,
    /// The size in bytes.
    #[not_personal]
    pub size: i64,
    /// The real type, read from the content ([`UNKNOWN_MEDIA_TYPE`] when it says nothing).
    #[not_personal]
    pub media_type: MediaType,
    /// Where the preview is.
    #[not_personal]
    pub preview: PreviewStatus,
    /// When the file was first stored.
    #[not_personal]
    pub created_at: Timestamp,
}

/// A file added to an issue, or to a comment under it (ATT-01).
#[derive(Clone, Debug, PartialEq, Eq, Entity)]
pub struct Attachment {
    /// The ID of the attachment.
    #[not_personal]
    pub id: AttachmentId,
    /// The issue it belongs to.
    #[not_personal]
    pub issue: IssueId,
    /// The comment it was added to; none for a file of the issue itself.
    #[not_personal]
    pub comment: Option<CommentId>,
    /// The file: the hash of its content.
    #[not_personal]
    pub file: Digest,
    /// The name the person gave it. Free text that could name a person.
    #[personal(category = "content", basis = "contract")]
    pub name: String,
    /// Who added it.
    #[personal(category = "activity", basis = "contract")]
    pub added_by: UserId,
    /// When it was added.
    #[not_personal]
    pub added_at: Timestamp,
    /// When it went into the trash, while it is there.
    #[not_personal]
    pub deleted_at: Option<Timestamp>,
    /// Where the preview of this attachment is. It is the attachment's own state, not the one of
    /// the file: a file that is stored already still gets its own job, so nobody learns from an
    /// answer that the same content was stored before (ADR-0032 §4).
    #[not_personal]
    pub preview: PreviewStatus,
    /// Counts up with every change, for the check that nobody changed it meanwhile (ADR-0019).
    #[not_personal]
    pub version: i64,
}

/// A file on its way: started with its name and size, sent in parts, then finished (ADR-0032).
#[derive(Clone, Debug, PartialEq, Eq, Entity)]
pub struct Upload {
    /// The ID of the upload.
    #[not_personal]
    pub id: UploadId,
    /// The issue the file is for.
    #[not_personal]
    pub issue: IssueId,
    /// The comment the file is for; none for a file of the issue itself.
    #[not_personal]
    pub comment: Option<CommentId>,
    /// Who sends it.
    #[personal(category = "activity", basis = "contract")]
    pub person: UserId,
    /// The name of the file. Free text that could name a person.
    #[personal(category = "content", basis = "contract")]
    pub name: String,
    /// The size the person said it has; the parts must add up to it.
    #[not_personal]
    pub size: i64,
    /// When the upload was started.
    #[not_personal]
    pub created_at: Timestamp,
    /// When the last part came (or the upload was started): an unfinished upload is deleted
    /// [`UNFINISHED_KEPT`] after it.
    #[not_personal]
    pub updated_at: Timestamp,
}

/// What a signed link says (ATT-07): which attachment, what of it, for whom, until when. The
/// server signs these bytes (`bb_auth::LinkKey`) and checks them again with a fresh look at the
/// issue every time the link is used.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DownloadLink {
    /// The attachment.
    pub attachment: AttachmentId,
    /// The file or its preview.
    pub target: LinkTarget,
    /// The one person it is for.
    pub person: UserId,
    /// The moment it stops working.
    pub expires_at: Timestamp,
}

/// The bytes of a link: the attachment, the target, the person and the end, in this order.
const LINK_BYTES: usize = 16 + 1 + 16 + 8;

impl DownloadLink {
    /// The link of `attachment` for `person`, good for [`LINK_LIFETIME`] from `now`.
    ///
    /// # Errors
    /// `None` when the end of the link is beyond the moments that can be written.
    #[must_use]
    pub fn new(
        attachment: AttachmentId,
        target: LinkTarget,
        person: UserId,
        now: Timestamp,
    ) -> Option<Self> {
        Some(Self {
            attachment,
            target,
            person,
            expires_at: now.checked_add(LINK_LIFETIME).ok()?,
        })
    }

    /// The bytes that are signed.
    #[must_use]
    pub fn to_bytes(&self) -> [u8; LINK_BYTES] {
        let mut bytes = [0_u8; LINK_BYTES];
        bytes[..16].copy_from_slice(&self.attachment.to_bytes());
        bytes[16] = match self.target {
            LinkTarget::File => 0,
            LinkTarget::Preview => 1,
        };
        bytes[17..33].copy_from_slice(&self.person.to_bytes());
        bytes[33..].copy_from_slice(&self.expires_at.as_microsecond().to_be_bytes());
        bytes
    }

    /// The link written in `bytes`, if they are one.
    #[must_use]
    pub fn from_bytes(bytes: &[u8]) -> Option<Self> {
        let bytes: &[u8; LINK_BYTES] = bytes.try_into().ok()?;
        let target = match bytes[16] {
            0 => LinkTarget::File,
            1 => LinkTarget::Preview,
            _ => return None,
        };
        Some(Self {
            attachment: AttachmentId::from_bytes(bytes[..16].try_into().ok()?).ok()?,
            target,
            person: UserId::from_bytes(bytes[17..33].try_into().ok()?).ok()?,
            expires_at: Timestamp::from_microsecond(i64::from_be_bytes(
                bytes[33..].try_into().ok()?,
            ))
            .ok()?,
        })
    }

    /// Whether the link still works at `now`.
    #[must_use]
    pub fn works_at(&self, now: Timestamp) -> bool {
        now < self.expires_at
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::id::SequentialIds;

    #[test]
    fn att_01_a_name_loses_its_path_and_control_characters_are_refused() {
        assert_eq!(
            clean_file_name("C:\\temp\\shot.png").as_deref(),
            Some("shot.png")
        );
        assert_eq!(
            clean_file_name("../../etc/passwd").as_deref(),
            Some("passwd")
        );
        assert_eq!(
            clean_file_name("  plan v2.pdf ").as_deref(),
            Some("plan v2.pdf")
        );
        assert_eq!(clean_file_name("a\0b.txt"), None);
        assert_eq!(clean_file_name("a\nb.txt"), None);
        assert_eq!(clean_file_name(".."), None);
        assert_eq!(clean_file_name("dir/"), None);
        assert_eq!(clean_file_name(""), None);
        assert_eq!(clean_file_name(&"x".repeat(NAME_MAX_CHARS + 1)), None);
        assert!(clean_file_name(&"x".repeat(NAME_MAX_CHARS)).is_some());
    }

    #[test]
    fn att_03_a_name_cannot_hide_its_extension_behind_dots_spaces_or_format_characters() {
        // Windows drops trailing dots and spaces, so these are `run.bat` there.
        assert_eq!(clean_file_name("run.bat.").as_deref(), Some("run.bat"));
        assert_eq!(clean_file_name("run.bat ").as_deref(), Some("run.bat"));
        assert_eq!(clean_file_name("run.bat. . ").as_deref(), Some("run.bat"));
        assert_eq!(clean_file_name("..."), None);
        // A right-to-left override turns `gpj.exe` into `exe.jpg` on the screen.
        assert_eq!(clean_file_name("photo\u{202E}gpj.exe"), None);
        assert_eq!(clean_file_name("zero\u{200B}width.txt"), None);
        assert_eq!(clean_file_name("\u{FEFF}bom.txt"), None);
        // The format characters that later versions of Unicode added are refused the same way.
        assert_eq!(clean_file_name("run\u{0890}.txt"), None);
        assert_eq!(clean_file_name("glyph\u{13430}.txt"), None);
        // The joiners that emoji and several scripts need stay allowed.
        let family = "family \u{1F468}\u{200D}\u{1F469}.png";
        assert_eq!(clean_file_name(family).as_deref(), Some(family));
        // Letters of other scripts are fine.
        let german = "Bericht \u{00FC}ber \u{65E5}\u{672C}.pdf";
        assert_eq!(clean_file_name(german).as_deref(), Some(german));
    }

    #[test]
    fn att_03_the_extension_of_a_name_is_its_last_part_in_lower_case() {
        assert_eq!(name_extension("Setup.EXE").as_deref(), Some("exe"));
        assert_eq!(name_extension("a.tar.gz").as_deref(), Some("gz"));
        assert_eq!(name_extension("README"), None);
        assert_eq!(name_extension(".hidden"), None);
        assert_eq!(name_extension("trailing."), None);
    }

    #[test]
    fn att_04_only_pictures_browsers_show_and_pdf_get_a_preview() {
        let kind = |text: &str| PreviewKind::of_media_type(&MediaType::new(text).unwrap());
        for shown in ["image/png", "image/jpeg", "image/gif", "image/webp"] {
            assert_eq!(kind(shown), Some(PreviewKind::Image));
        }
        assert_eq!(kind("application/pdf"), Some(PreviewKind::Pdf));
        for none in [
            "image/svg+xml",
            "text/html",
            UNKNOWN_MEDIA_TYPE,
            "image/x-icon",
        ] {
            assert_eq!(kind(none), None, "{none}");
        }
        assert_eq!(MediaType::new("image/PNG"), None);
        assert_eq!(MediaType::new("png"), None);
        assert_eq!(MediaType::new("a/b c"), None);
    }

    #[test]
    fn att_07_a_link_is_written_and_read_back_and_works_for_five_minutes() {
        let mut ids = SequentialIds::default();
        let now: Timestamp = "2027-03-01T10:00:00Z".parse().unwrap();
        let link = DownloadLink::new(
            AttachmentId::generate(&mut ids).unwrap(),
            LinkTarget::Preview,
            UserId::generate(&mut ids),
            now,
        )
        .unwrap();
        assert_eq!(DownloadLink::from_bytes(&link.to_bytes()), Some(link));
        assert!(link.works_at(now));
        assert!(link.works_at("2027-03-01T10:04:59Z".parse().unwrap()));
        assert!(!link.works_at("2027-03-01T10:05:00Z".parse().unwrap()));
        let mut changed = link.to_bytes();
        changed[16] = 7;
        assert_eq!(DownloadLink::from_bytes(&changed), None);
        assert_eq!(DownloadLink::from_bytes(&changed[1..]), None);
    }
}
