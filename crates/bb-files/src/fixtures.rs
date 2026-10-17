//! Files for the tests of previews, here and in the tests of the server: a picture, a PDF, and what
//! a decompression bomb looks like from the outside. Not part of the program: only the tests switch
//! the feature on.

use std::io::Cursor;

use image::{DynamicImage, ImageFormat, Rgba, RgbaImage};

/// A picture of one colour, `width` by `height`, in this format.
///
/// The bytes are empty if the picture cannot be written, which no format of the previews does.
#[must_use]
pub fn picture(width: u32, height: u32, format: ImageFormat) -> Vec<u8> {
    let image = DynamicImage::ImageRgba8(RgbaImage::from_pixel(
        width,
        height,
        Rgba([200, 30, 30, 255]),
    ));
    let mut bytes = Cursor::new(Vec::new());
    image
        .write_to(&mut bytes, format)
        .map(|()| bytes.into_inner())
        .unwrap_or_default()
}

/// The CRC-32 that PNG puts after every chunk.
fn crc32(bytes: &[u8]) -> u32 {
    let mut crc = u32::MAX;
    for byte in bytes {
        crc ^= u32::from(*byte);
        for _ in 0..8 {
            crc = if crc & 1 == 1 {
                (crc >> 1) ^ 0xEDB8_8320
            } else {
                crc >> 1
            };
        }
    }
    !crc
}

/// One chunk of a PNG: its length, its name, its data and the CRC of name and data.
fn chunk(png: &mut Vec<u8>, name: [u8; 4], data: &[u8]) {
    png.extend_from_slice(&u32::try_from(data.len()).unwrap_or(u32::MAX).to_be_bytes());
    let named = [&name[..], data].concat();
    png.extend_from_slice(&named);
    png.extend_from_slice(&crc32(&named).to_be_bytes());
}

/// A PNG that says it is `width` by `height` and holds no picture at all: what a decompression
/// bomb looks like from the outside. Nothing but the header is true.
#[must_use]
pub fn pretending_png(width: u32, height: u32) -> Vec<u8> {
    let mut header = Vec::new();
    header.extend_from_slice(&width.to_be_bytes());
    header.extend_from_slice(&height.to_be_bytes());
    header.extend_from_slice(&[8, 0, 0, 0, 0]);
    let mut png = vec![0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A];
    chunk(&mut png, *b"IHDR", &header);
    chunk(&mut png, *b"IDAT", &[0x78, 0x9C, 0x03, 0x00]);
    chunk(&mut png, *b"IEND", &[]);
    png
}

/// A PDF of one page, 200 by 100 points, with a blue rectangle at 20, 20 that is 100 by 50. It
/// has no table of its own: the library finds its objects by reading the file.
pub const PDF: &[u8] = b"%PDF-1.4\n\
1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n\
2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n\
3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 200 100] /Contents 4 0 R >>\nendobj\n\
4 0 obj\n<< /Length 30 >>\nstream\n0 0 1 rg 20 20 100 50 re f\nendstream\nendobj\n\
trailer\n<< /Root 1 0 R /Size 5 >>\n%%EOF\n";

/// The format of a picture, so a test names it without the `image` library.
pub use image::ImageFormat as Format;
