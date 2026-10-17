//! The preview of a PDF: its first page, drawn on white by `hayro` (pure Rust, ADR-0032 §4). A PDF
//! that needs a password, is broken, or has no page gets no preview.

use hayro::hayro_interpret::InterpreterSettings;
use hayro::hayro_syntax::Pdf;
use hayro::vello_cpu::color::palette::css::WHITE;
use hayro::{PixmapSettings, RenderCache, RenderSettings};
use image::{DynamicImage, RgbaImage};

use super::{PREVIEW_EDGE_PIXELS, PreviewRefusal, encode};

/// The most a small page is enlarged: a page is shown at the size of the preview, but a stamp-size
/// page is not blown up into a blur.
const SCALE_UP_MAX: f32 = 2.0;

/// The preview of the first page of the PDF in `bytes` as PNG.
pub(super) fn render(bytes: &[u8]) -> Result<Vec<u8>, PreviewRefusal> {
    let pdf = Pdf::new(bytes.to_vec()).map_err(|_| PreviewRefusal::Unreadable)?;
    let page = pdf.pages().first().ok_or(PreviewRefusal::Unreadable)?;
    let (width, height) = page.render_dimensions();
    let longest = width.max(height);
    if !(width.is_finite() && height.is_finite() && width > 0.0 && height > 0.0) {
        return Err(PreviewRefusal::Unreadable);
    }
    // The page is drawn at the size of the preview: its pixels are known before it is drawn,
    // whatever size the page claims to have.
    let edge = f32::from(u16::try_from(PREVIEW_EDGE_PIXELS).unwrap_or(u16::MAX));
    let scale = (edge / longest).min(SCALE_UP_MAX);
    let pixmap = render_page(page, scale);
    let (width, height) = (u32::from(pixmap.width()), u32::from(pixmap.height()));
    if width == 0 || height == 0 {
        return Err(PreviewRefusal::Unreadable);
    }
    // The page is drawn on an opaque background, so the pixels are not premultiplied by anything.
    let picture = RgbaImage::from_raw(width, height, pixmap.data_as_u8_slice().to_vec())
        .ok_or(PreviewRefusal::Unreadable)?;
    encode(&DynamicImage::ImageRgba8(picture))
}

fn render_page(page: &hayro::hayro_syntax::page::Page<'_>, scale: f32) -> hayro::vello_cpu::Pixmap {
    hayro::render(
        page,
        &RenderCache::new(),
        &InterpreterSettings::default(),
        &RenderSettings::default(),
        &PixmapSettings {
            x_scale: scale,
            y_scale: scale,
            bg_color: WHITE,
        },
    )
}
