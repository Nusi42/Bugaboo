//! The previews and their limits (ATT-04, ADR-0032 §4): a picture and the first page of a PDF get a
//! small PNG, and a picture with too many pixels, a broken PDF and a file of another kind get none.

use std::io::Cursor;

use image::{DynamicImage, ImageFormat, Rgba, RgbaImage};

use super::*;

/// A PNG picture of one colour.
fn picture(width: u32, height: u32, format: ImageFormat) -> Vec<u8> {
    let image = DynamicImage::ImageRgba8(RgbaImage::from_pixel(
        width,
        height,
        Rgba([200, 30, 30, 255]),
    ));
    let mut bytes = Cursor::new(Vec::new());
    image.write_to(&mut bytes, format).unwrap();
    bytes.into_inner()
}

/// The size of a PNG that was written.
fn size_of(png: &[u8]) -> (u32, u32) {
    let image = image::load_from_memory_with_format(png, ImageFormat::Png).unwrap();
    (image.width(), image.height())
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

/// A PNG that says it is `width` by `height` and holds no picture at all: what a decompression
/// bomb looks like from the outside. Nothing but the header is true.
fn pretending_png(width: u32, height: u32) -> Vec<u8> {
    let mut header = Vec::new();
    header.extend_from_slice(&width.to_be_bytes());
    header.extend_from_slice(&height.to_be_bytes());
    header.extend_from_slice(&[8, 0, 0, 0, 0]);
    let mut png = vec![0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A];
    chunk(&mut png, b"IHDR", &header);
    chunk(&mut png, b"IDAT", &[0x78, 0x9C, 0x03, 0x00]);
    chunk(&mut png, b"IEND", &[]);
    png
}

/// One chunk of a PNG: its length, its name, its data and the CRC of name and data.
fn chunk(png: &mut Vec<u8>, name: &[u8; 4], data: &[u8]) {
    png.extend_from_slice(&u32::try_from(data.len()).unwrap().to_be_bytes());
    let named = [&name[..], data].concat();
    png.extend_from_slice(&named);
    png.extend_from_slice(&crc32(&named).to_be_bytes());
}

/// A PDF of one page, 200 by 100 points, with a blue rectangle at 20, 20 that is 100 by 50. It
/// has no table of its own: the library finds its objects by reading the file.
const PDF: &[u8] = b"%PDF-1.4\n\
1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n\
2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n\
3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 200 100] /Contents 4 0 R >>\nendobj\n\
4 0 obj\n<< /Length 30 >>\nstream\n0 0 1 rg 20 20 100 50 re f\nendstream\nendobj\n\
trailer\n<< /Root 1 0 R /Size 5 >>\n%%EOF\n";

#[test]
fn att_04_a_picture_gets_a_preview_that_fits_and_is_never_enlarged() {
    for format in [ImageFormat::Png, ImageFormat::Jpeg, ImageFormat::Gif] {
        let big = render(PreviewKind::Image, &picture(2000, 1000, format)).unwrap();
        assert_eq!(
            size_of(&big),
            (PREVIEW_EDGE_PIXELS, PREVIEW_EDGE_PIXELS / 2),
            "{format:?}"
        );
        let small = render(PreviewKind::Image, &picture(100, 50, format)).unwrap();
        assert_eq!(size_of(&small), (100, 50), "{format:?}");
    }
}

#[test]
fn att_04_the_preview_is_the_content_not_the_name() {
    // A JPEG read as a picture, however it was named: the format is found from the bytes.
    let jpeg = picture(300, 200, ImageFormat::Jpeg);
    assert!(render(PreviewKind::Image, &jpeg).is_ok());
    // Something that is no picture gets none.
    assert_eq!(
        render(
            PreviewKind::Image,
            b"<svg xmlns=\"http://www.w3.org/2000/svg\"/>"
        ),
        Err(PreviewRefusal::Unreadable)
    );
    assert_eq!(
        render(PreviewKind::Image, b""),
        Err(PreviewRefusal::Unreadable)
    );
}

#[test]
fn att_04_a_huge_image_is_refused_by_the_pixel_limit() {
    // The limit is on the size the header says, so the picture is never decoded.
    let over = u32::try_from(MAX_IMAGE_PIXELS / 1000 + 1).unwrap();
    assert_eq!(
        render(PreviewKind::Image, &pretending_png(1000, over)),
        Err(PreviewRefusal::TooManyPixels)
    );
    // A decompression bomb: a few bytes that say 100000 by 100000 pixels.
    let bomb = pretending_png(100_000, 100_000);
    assert!(bomb.len() < 100);
    assert_eq!(
        render(PreviewKind::Image, &bomb),
        Err(PreviewRefusal::TooManyPixels)
    );
    // At the limit it is only a broken picture, not a refused one.
    let at_the_limit = u32::try_from(MAX_IMAGE_PIXELS / 1000).unwrap();
    assert_eq!(
        render(PreviewKind::Image, &pretending_png(1000, at_the_limit)),
        Err(PreviewRefusal::Unreadable)
    );
}

#[test]
fn att_04_the_first_page_of_a_pdf_gets_a_preview() {
    let png = render(PreviewKind::Pdf, PDF).unwrap();
    // The page is drawn at twice its size (a small page is not blown up more than that).
    assert_eq!(size_of(&png), (400, 200));
    let page = image::load_from_memory_with_format(&png, ImageFormat::Png)
        .unwrap()
        .to_rgba8();
    assert_eq!(
        page.get_pixel(100, 100).0,
        [0, 0, 255, 255],
        "inside the blue rectangle"
    );
    assert_eq!(
        page.get_pixel(10, 10).0,
        [255, 255, 255, 255],
        "white paper"
    );
}

#[test]
fn att_04_a_broken_pdf_gets_no_preview() {
    for broken in [
        &b""[..],
        b"%PDF-1.4\nthis is not a document",
        b"%PDF-1.4\n1 0 obj\n<< /Type /Catalog /Pages 9 0 R >>\nendobj\ntrailer\n<< /Root 1 0 R >>\n",
        &PDF[..PDF.len() / 2],
        &picture(10, 10, ImageFormat::Png),
    ] {
        assert_eq!(render(PreviewKind::Pdf, broken), Err(PreviewRefusal::Unreadable));
    }
}

#[test]
fn att_04_a_file_over_the_input_limit_is_not_read() {
    let big = vec![0_u8; usize::try_from(MAX_INPUT_BYTES).unwrap() + 1];
    assert_eq!(
        render(PreviewKind::Image, &big),
        Err(PreviewRefusal::TooLarge)
    );
}

#[test]
fn att_04_the_input_limit_is_the_biggest_file_the_settings_allow() {
    assert_eq!(MAX_INPUT_BYTES, 100 * MEGABYTE);
}

#[test]
fn att_04_the_child_writes_a_png_or_answers_with_the_code_of_its_refusal() {
    let folder = tempfile::tempdir().unwrap();
    let (input, output) = (folder.path().join("in"), folder.path().join("out.png"));
    std::fs::write(&input, picture(50, 50, ImageFormat::Png)).unwrap();
    assert_eq!(run(PreviewKind::Image, &input, &output), 0);
    assert_eq!(size_of(&std::fs::read(&output).unwrap()), (50, 50));

    std::fs::remove_file(&output).unwrap();
    std::fs::write(&input, pretending_png(100_000, 100_000)).unwrap();
    assert_eq!(
        run(PreviewKind::Image, &input, &output),
        PreviewRefusal::TooManyPixels.exit_code()
    );
    assert!(!output.exists(), "a refused file leaves nothing behind");
    assert_eq!(
        run(PreviewKind::Pdf, &folder.path().join("missing"), &output),
        1
    );
}
