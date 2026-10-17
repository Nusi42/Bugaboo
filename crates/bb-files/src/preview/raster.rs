//! The preview of a picture: PNG, JPEG, GIF (its first picture) and WebP, read by the `image`
//! library. The size is read from the header first and refused when it is over the pixel limit,
//! so a small file that says it is huge is never decoded.

use std::io::Cursor;

use image::{DynamicImage, ImageReader, Limits};

use super::{MAX_DECODER_BYTES, MAX_IMAGE_PIXELS, PREVIEW_EDGE_PIXELS, PreviewRefusal, encode};

/// A reader of the picture in `bytes`, whose format is found from the content and which is held
/// to the memory limit.
fn reader(bytes: &[u8]) -> Result<ImageReader<Cursor<&[u8]>>, PreviewRefusal> {
    let mut reader = ImageReader::new(Cursor::new(bytes))
        .with_guessed_format()
        .map_err(|_| PreviewRefusal::Unreadable)?;
    let mut limits = Limits::default();
    limits.max_alloc = Some(MAX_DECODER_BYTES);
    reader.limits(limits);
    Ok(reader)
}

/// The preview of the picture in `bytes` as PNG.
pub(super) fn render(bytes: &[u8]) -> Result<Vec<u8>, PreviewRefusal> {
    let (width, height) = reader(bytes)?
        .into_dimensions()
        .map_err(|_| PreviewRefusal::Unreadable)?;
    if u64::from(width) * u64::from(height) > MAX_IMAGE_PIXELS {
        return Err(PreviewRefusal::TooManyPixels);
    }
    let picture = reader(bytes)?
        .decode()
        .map_err(|_| PreviewRefusal::Unreadable)?;
    encode(&fitted(picture))
}

/// The picture scaled down to fit the preview, never up.
fn fitted(picture: DynamicImage) -> DynamicImage {
    if picture.width().max(picture.height()) > PREVIEW_EDGE_PIXELS {
        picture.thumbnail(PREVIEW_EDGE_PIXELS, PREVIEW_EDGE_PIXELS)
    } else {
        picture
    }
}
