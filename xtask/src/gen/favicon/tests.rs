//! The tests of the icon.

use std::io::Read as _;

use super::*;
use crate::repo::Repo;

fn file() -> ThemeFile {
    ThemeFile::load(&Repo::discover().unwrap()).unwrap()
}

/// The value of a token in the light theme that starts, as the theme file writes it.
fn written(file: &ThemeFile, name: &str) -> String {
    file.tokens
        .iter()
        .find(|token| token.name == name)
        .and_then(|token| token.in_theme(&file.default_light))
        .unwrap()
        .to_owned()
}

#[test]
fn thm_the_colors_of_the_icon_are_the_accent_tokens_of_the_theme() {
    let file = file();
    let colors = colors(&file).unwrap();
    assert_eq!(colors.accent.hex(), written(&file, ACCENT));
    assert_eq!(colors.on_accent.hex(), written(&file, ON_ACCENT));
    let svg = svg(colors, "test");
    assert!(svg.contains(&format!("fill=\"{}\"", written(&file, ACCENT))));
    assert!(svg.contains(&format!("fill=\"{}\"", written(&file, ON_ACCENT))));
}

/// The RGBA rows inside a PNG file this module wrote, and its side.
fn decoded(file: &[u8]) -> (u32, Vec<u8>) {
    assert_eq!(file[..8], PNG_SIGNATURE);
    let side = u32::from_be_bytes(file[16..20].try_into().unwrap());
    let length = usize::try_from(u32::from_be_bytes(file[33..37].try_into().unwrap())).unwrap();
    assert_eq!(&file[37..41], b"IDAT");
    let mut rows = Vec::new();
    flate2::read::ZlibDecoder::new(&file[41..41 + length])
        .read_to_end(&mut rows)
        .unwrap();
    let row_bytes = usize::try_from(side).unwrap() * BYTES_PER_PIXEL + 1;
    let rgba = rows
        .chunks(row_bytes)
        .flat_map(|row| row[1..].to_vec())
        .collect();
    (side, rgba)
}

fn at(rgba: &[u8], side: u32, x: u32, y: u32) -> [u8; 4] {
    let start = usize::try_from((y * side + x) * 4).unwrap();
    rgba[start..start + 4].try_into().unwrap()
}

#[test]
fn thm_the_pictures_show_the_ghost_on_the_accent_square_with_clear_corners() {
    let colors = IconColors {
        accent: Rgb::parse("#5b3fd9").unwrap(),
        on_accent: Rgb::parse("#ffffff").unwrap(),
    };
    for side in [SMALL_PX, TOUCH_PX] {
        let (read_side, rgba) = decoded(&png(side, &draw(colors, side)).unwrap());
        assert_eq!(read_side, side);
        // A point of the drawing, in tenths of its units, as a pixel of this picture.
        let px = |tenths: u32| tenths * side / 320;
        // The corner outside the rounding is clear, the edge of the square is the accent, the
        // middle of the ghost's body is white and the middle of an eye is the accent again.
        assert_eq!(at(&rgba, side, 0, 0), [0, 0, 0, 0]);
        assert_eq!(at(&rgba, side, px(10), px(160)), [0x5b, 0x3f, 0xd9, 255]);
        assert_eq!(at(&rgba, side, px(160), px(200)), [255, 255, 255, 255]);
        assert_eq!(at(&rgba, side, px(133), px(146)), [0x5b, 0x3f, 0xd9, 255]);
    }
}

#[test]
fn thm_a_color_that_is_not_written_as_rrggbb_is_refused() {
    assert!(Rgb::parse("#5b3fd").is_err());
    assert!(Rgb::parse("5b3fd9").is_err());
    assert!(Rgb::parse("#5b3fzz").is_err());
}
