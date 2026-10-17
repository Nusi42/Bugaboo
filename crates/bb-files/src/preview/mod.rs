//! Previews (ATT-04, ADR-0032 §4): a small picture of an image or of the first page of a PDF.
//!
//! A preview is made by `bugaboo preview`, a child process of the server ([`spawn`]), because the
//! libraries that read pictures and PDFs are the most likely place for a crafted file to find a
//! bug. The child reads one file and writes one small PNG, the one safe image type the page shows
//! ([`bb_core::files::PREVIEW_MEDIA_TYPE`]). It is held to limits that are named here and nowhere
//! else:
//!
//! - **pixels** ([`MAX_IMAGE_PIXELS`]): a picture that says it has more is refused before a single
//!   pixel is decoded, so a small file that unpacks to gigabytes (a decompression bomb) costs
//!   nothing;
//! - **memory** ([`MAX_DECODER_BYTES`]): what the decoder may allocate; the page of a PDF is drawn
//!   into a picture of at most [`PREVIEW_EDGE_PIXELS`] on its longest side;
//! - **time** ([`TIME_LIMIT`]): the server stops the child when it takes longer;
//! - **size** ([`MAX_INPUT_BYTES`]): a bigger input is not read.
//!
//! A file that is refused, broken, too slow or that crashes the child gets no preview, and the
//! server goes on.

mod pdf;
mod raster;
mod spawn;

use std::io::Write as _;
use std::path::Path;
use std::time::Duration;

use bb_core::files::{BIGGEST_FILE_BYTES, MEGABYTE, PreviewKind};

pub(crate) use spawn::create;
pub use spawn::{PreviewCommand, PreviewOutcome};

/// The longest side of a preview, in pixels. Big enough to see what a screenshot shows in the row
/// of the *Files* block and in the larger view, small enough to send at once.
pub const PREVIEW_EDGE_PIXELS: u32 = 640;

/// The most pixels a picture may have to get a preview (about 40 megapixels: what a good camera
/// takes). It is checked from the header of the picture, before anything is decoded.
pub const MAX_IMAGE_PIXELS: u64 = 40_000_000;

/// The most memory the decoder of pictures may ask for, in bytes.
pub const MAX_DECODER_BYTES: u64 = 512 * MEGABYTE;

/// How long the child may take before the server stops it.
pub const TIME_LIMIT: Duration = Duration::from_secs(20);

/// The biggest file the child reads: the biggest file the settings allow.
pub const MAX_INPUT_BYTES: u64 = BIGGEST_FILE_BYTES;

/// The biggest preview the server accepts from the child, in bytes. A preview that is bigger is
/// not a small safe picture and is thrown away.
pub const MAX_OUTPUT_BYTES: u64 = 4 * MEGABYTE;

/// Why a file got no preview.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PreviewRefusal {
    /// The picture has more pixels than [`MAX_IMAGE_PIXELS`].
    TooManyPixels,
    /// The file is bigger than [`MAX_INPUT_BYTES`].
    TooLarge,
    /// The file is no picture or PDF the libraries can read, or it is broken.
    Unreadable,
}

impl PreviewRefusal {
    /// The exit code of the child for this refusal; `0` is a preview, `1` an error of the file
    /// system, and a crash has whatever code the system gives it.
    #[must_use]
    pub const fn exit_code(self) -> i32 {
        match self {
            Self::TooManyPixels => 3,
            Self::TooLarge => 4,
            Self::Unreadable => 5,
        }
    }
}

/// The preview of a picture or of the first page of a PDF, as PNG bytes.
///
/// # Errors
/// The reason the file gets no preview.
pub fn render(kind: PreviewKind, bytes: &[u8]) -> Result<Vec<u8>, PreviewRefusal> {
    if u64::try_from(bytes.len()).map_err(|_| PreviewRefusal::TooLarge)? > MAX_INPUT_BYTES {
        return Err(PreviewRefusal::TooLarge);
    }
    // A library that panics on a crafted file ends in the same place as one that refuses it.
    let made = std::panic::catch_unwind(|| match kind {
        PreviewKind::Image => raster::render(bytes),
        PreviewKind::Pdf => pdf::render(bytes),
    });
    made.unwrap_or(Err(PreviewRefusal::Unreadable))
}

/// What `bugaboo preview` does: reads `input`, writes the preview to `output`, and answers the exit
/// code of the process (see [`PreviewRefusal::exit_code`]).
#[must_use]
pub fn run(kind: PreviewKind, input: &Path, output: &Path) -> i32 {
    let Ok(file) = std::fs::File::open(input) else {
        return 1;
    };
    let mut bytes = Vec::new();
    // One byte more than the limit is read, so a bigger file is seen as bigger.
    if std::io::Read::read_to_end(
        &mut std::io::Read::take(file, MAX_INPUT_BYTES + 1),
        &mut bytes,
    )
    .is_err()
    {
        return 1;
    }
    match render(kind, &bytes) {
        Ok(png) => {
            let written = std::fs::File::create(output).and_then(|mut file| {
                file.write_all(&png)?;
                file.sync_all()
            });
            if written.is_ok() { 0 } else { 1 }
        }
        Err(refusal) => refusal.exit_code(),
    }
}

/// The preview as PNG, written the one way for pictures and for pages.
fn encode(image: &image::DynamicImage) -> Result<Vec<u8>, PreviewRefusal> {
    let mut png = std::io::Cursor::new(Vec::new());
    image
        .write_to(&mut png, image::ImageFormat::Png)
        .map_err(|_| PreviewRefusal::Unreadable)?;
    Ok(png.into_inner())
}

#[cfg(test)]
mod tests;
