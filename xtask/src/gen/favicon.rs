//! The icon of the app (a friendly white ghost, a bugaboo, on a square in the accent color), made
//! by `cargo xtask gen theme` from the colors of the light theme that starts, so a change of the
//! accent changes the icon too (SSOT). The drawing lives here once, as data: the SVG writes it out
//! and the pictures for browsers without SVG icons are drawn from the same shapes, on a fixed grid,
//! so they are the same on every computer.

use std::fmt::Write as _;
use std::io::Write as _;

use anyhow::Context as _;
use flate2::Crc;
use flate2::write::ZlibEncoder;

use super::Generated;
use super::theme_file::ThemeFile;

/// The icon the page names first. Vite serves `web/app/public` at the root, and the build copies it
/// into the root of `dist`, which the server serves.
pub(crate) const SVG: &str = "web/app/public/favicon.svg";
/// The icon for browsers that take no SVG icon.
pub(crate) const PNG_SMALL: &str = "web/app/public/favicon-32.png";
/// The icon of a page saved to the home screen of a phone (`apple-touch-icon`).
pub(crate) const PNG_TOUCH: &str = "web/app/public/apple-touch-icon.png";

/// The sides of the two pictures, in pixels.
const SMALL_PX: u32 = 32;
const TOUCH_PX: u32 = 180;

/// The tokens the icon is drawn in.
const ACCENT: &str = "accent";
const ON_ACCENT: &str = "on-accent";

/// The paths [`files`] writes.
pub(crate) fn paths() -> [&'static str; 3] {
    [SVG, PNG_SMALL, PNG_TOUCH]
}

/// A color as the theme file writes it, `#rrggbb`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Rgb {
    pub(crate) red: u8,
    pub(crate) green: u8,
    pub(crate) blue: u8,
}

impl Rgb {
    /// Reads `#rrggbb` into whole numbers. `bb_core::contrast::channels` reads the same text into
    /// fractions and takes a bad digit for 0; the pixels here are mixed in whole numbers (no cast
    /// from a fraction back to a byte), and a color the theme reader let through wrongly must stop
    /// the generator rather than draw a black icon.
    pub(crate) fn parse(text: &str) -> anyhow::Result<Self> {
        let digits = text
            .strip_prefix('#')
            .filter(|digits| digits.len() == 6 && digits.is_ascii())
            .with_context(|| format!("`{text}` is no color of the form #rrggbb"))?;
        let part = |at: usize| {
            u8::from_str_radix(&digits[at..at + 2], 16)
                .with_context(|| format!("`{text}` is no color of the form #rrggbb"))
        };
        Ok(Self {
            red: part(0)?,
            green: part(2)?,
            blue: part(4)?,
        })
    }

    pub(crate) fn hex(self) -> String {
        format!("#{:02x}{:02x}{:02x}", self.red, self.green, self.blue)
    }

    fn channels(self) -> [u32; 3] {
        [self.red, self.green, self.blue].map(u32::from)
    }
}

/// The two colors of the icon.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct IconColors {
    pub(crate) accent: Rgb,
    pub(crate) on_accent: Rgb,
}

/// The color of a token in the light theme that starts, with a token that names another followed.
fn color(file: &ThemeFile, name: &str) -> anyhow::Result<Rgb> {
    let value = file
        .color(name, &file.default_light)
        .with_context(|| format!("the theme `{}` has no color `{name}`", file.default_light))?;
    Rgb::parse(value)
}

/// The colors of the icon: the accent, and the color that is drawn on it.
pub(crate) fn colors(file: &ThemeFile) -> anyhow::Result<IconColors> {
    Ok(IconColors {
        accent: color(file, ACCENT)?,
        on_accent: color(file, ON_ACCENT)?,
    })
}

/// The files of the icon.
pub(crate) fn files(file: &ThemeFile, generated_by: &str) -> anyhow::Result<Vec<Generated>> {
    let colors = colors(file)?;
    Ok(vec![
        Generated::new(SVG, svg(colors, generated_by)),
        Generated::binary(PNG_SMALL, png(SMALL_PX, &draw(colors, SMALL_PX))?),
        Generated::binary(PNG_TOUCH, png(TOUCH_PX, &draw(colors, TOUCH_PX))?),
    ])
}

/// The side of the drawing, in its own units.
const VIEW: f64 = 32.0;
/// The rounding of the corners of the square.
const CORNER: f64 = 7.0;
/// The head of the ghost: a half circle.
const HEAD_X: f64 = 16.0;
const HEAD_Y: f64 = 14.5;
const HEAD_R: f64 = 7.0;
/// The body under the head and its wavy hem, from the right end of the head round to the left.
const BODY: [(f64, f64); 9] = [
    (23.0, 14.5),
    (23.0, 26.5),
    (20.67, 24.5),
    (18.33, 26.5),
    (16.0, 24.5),
    (13.67, 26.5),
    (11.33, 24.5),
    (9.0, 26.5),
    (9.0, 14.5),
];
/// The two eyes: their centers and radii.
const EYES: [(f64, f64); 2] = [(13.3, 14.6), (18.7, 14.6)];
const EYE_RX: f64 = 1.6;
const EYE_RY: f64 = 2.1;

/// The drawing as SVG, in the two colors.
fn svg(colors: IconColors, generated_by: &str) -> String {
    let accent = colors.accent.hex();
    let on_accent = colors.on_accent.hex();
    let body = BODY[1..].iter().fold(String::new(), |mut path, (x, y)| {
        let _ = write!(path, "L{x} {y}");
        path
    });
    let eyes = EYES.iter().fold(String::new(), |mut eyes, (x, y)| {
        let _ = writeln!(
            eyes,
            "  <ellipse cx=\"{x}\" cy=\"{y}\" rx=\"{EYE_RX}\" ry=\"{EYE_RY}\" fill=\"{accent}\"/>"
        );
        eyes
    });
    let (left, right) = (HEAD_X - HEAD_R, HEAD_X + HEAD_R);
    let head = format!("M{left} {HEAD_Y}A{HEAD_R} {HEAD_R} 0 0 1 {right} {HEAD_Y}");
    format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 {VIEW} {VIEW}\">\n  <!-- {generated_by} -->\n  <rect width=\"{VIEW}\" height=\"{VIEW}\" rx=\"{CORNER}\" fill=\"{accent}\"/>\n  <path fill=\"{on_accent}\" d=\"{head}{body}Z\"/>\n{eyes}</svg>\n"
    )
}

/// Whether a point of the drawing is inside the rounded square.
fn in_square(x: f64, y: f64) -> bool {
    let nearest_x = x.clamp(CORNER, VIEW - CORNER);
    let nearest_y = y.clamp(CORNER, VIEW - CORNER);
    (0.0..=VIEW).contains(&x)
        && (0.0..=VIEW).contains(&y)
        && (x - nearest_x).powi(2) + (y - nearest_y).powi(2) <= CORNER * CORNER
}

/// Whether a point is on the ghost: in its head, or in its body (counted by crossings of its edge).
fn in_ghost(x: f64, y: f64) -> bool {
    let head = y <= HEAD_Y && (x - HEAD_X).powi(2) + (y - HEAD_Y).powi(2) <= HEAD_R * HEAD_R;
    let mut body = false;
    for (index, &(x1, y1)) in BODY.iter().enumerate() {
        let (x2, y2) = BODY[(index + 1) % BODY.len()];
        if (y1 > y) != (y2 > y) && x < x1 + (y - y1) * (x2 - x1) / (y2 - y1) {
            body = !body;
        }
    }
    head || body
}

fn in_eye(x: f64, y: f64) -> bool {
    EYES.iter()
        .any(|(cx, cy)| ((x - cx) / EYE_RX).powi(2) + ((y - cy) / EYE_RY).powi(2) <= 1.0)
}

/// How many points of each pixel are looked at, along each side.
const SAMPLES: u32 = 8;

/// The picture of `side` pixels as RGBA rows: each pixel is the mean of a fixed grid of points.
fn draw(colors: IconColors, side: u32) -> Vec<u8> {
    let scale = VIEW / f64::from(side * SAMPLES);
    let mut pixels = Vec::new();
    for row in 0..side {
        for column in 0..side {
            let (mut square, mut ghost) = (0_u32, 0_u32);
            for sub_y in 0..SAMPLES {
                for sub_x in 0..SAMPLES {
                    let x = (f64::from(column * SAMPLES + sub_x) + 0.5) * scale;
                    let y = (f64::from(row * SAMPLES + sub_y) + 0.5) * scale;
                    if in_square(x, y) {
                        square += 1;
                        if in_ghost(x, y) && !in_eye(x, y) {
                            ghost += 1;
                        }
                    }
                }
            }
            pixels.extend(pixel(colors, square, ghost));
        }
    }
    pixels
}

/// One pixel from the points that fell in the square and, of those, on the ghost.
fn pixel(colors: IconColors, square: u32, ghost: u32) -> [u8; 4] {
    if square == 0 {
        return [0; 4];
    }
    let all = SAMPLES * SAMPLES;
    let accent = colors.accent.channels();
    let on_accent = colors.on_accent.channels();
    let mixed = |at: usize| {
        let sum = on_accent[at] * ghost + accent[at] * (square - ghost);
        u8::try_from((sum + square / 2) / square).unwrap_or(u8::MAX)
    };
    let alpha = u8::try_from((square * u32::from(u8::MAX) + all / 2) / all).unwrap_or(u8::MAX);
    [mixed(0), mixed(1), mixed(2), alpha]
}

/// The bytes that open every PNG file.
const PNG_SIGNATURE: [u8; 8] = [0x89, b'P', b'N', b'G', b'\r', b'\n', 0x1a, b'\n'];
/// Eight bits for each of red, green, blue and alpha.
const BIT_DEPTH: u8 = 8;
const COLOR_RGBA: u8 = 6;
const BYTES_PER_PIXEL: usize = 4;
/// Each row starts with the filter it uses: none.
const NO_FILTER: u8 = 0;

/// A PNG file of a square picture of `side` pixels, from its RGBA rows.
fn png(side: u32, rgba: &[u8]) -> anyhow::Result<Vec<u8>> {
    let row_bytes = usize::try_from(side)? * BYTES_PER_PIXEL;
    let mut rows = Vec::with_capacity(rgba.len() + rgba.len() / row_bytes);
    for row in rgba.chunks(row_bytes) {
        rows.push(NO_FILTER);
        rows.extend_from_slice(row);
    }
    let mut encoder = ZlibEncoder::new(Vec::new(), flate2::Compression::best());
    encoder.write_all(&rows)?;
    let mut header = Vec::new();
    header.extend(side.to_be_bytes());
    header.extend(side.to_be_bytes());
    // Compression, filter method and interlacing: the only ones there are, and none.
    header.extend([BIT_DEPTH, COLOR_RGBA, 0, 0, 0]);
    let mut file = PNG_SIGNATURE.to_vec();
    chunk(&mut file, *b"IHDR", &header)?;
    chunk(&mut file, *b"IDAT", &encoder.finish()?)?;
    chunk(&mut file, *b"IEND", &[])?;
    Ok(file)
}

/// Appends one chunk of a PNG file: its length, its kind, its data and their checksum.
fn chunk(file: &mut Vec<u8>, kind: [u8; 4], data: &[u8]) -> anyhow::Result<()> {
    file.extend(u32::try_from(data.len())?.to_be_bytes());
    let mut crc = Crc::new();
    crc.update(&kind);
    crc.update(data);
    file.extend_from_slice(&kind);
    file.extend_from_slice(data);
    file.extend(crc.sum().to_be_bytes());
    Ok(())
}

#[cfg(test)]
mod tests;
