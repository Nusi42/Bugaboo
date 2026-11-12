//! The maths of readability: brightness, the contrast ratio of WCAG 2.2 and the difference of two
//! colors (CIE 1976 delta E). One implementation for the check of the built-in theme files, for the
//! server when an own theme is saved and for the theme editor (ADR-0026, update M8).

/// WCAG 2.2 contrast ratio between two `#RRGGBB` colors.
#[must_use]
pub fn contrast_ratio(first: &str, second: &str) -> f64 {
    ratio_of(luminance(first), luminance(second))
}

/// The contrast ratio of two luminances.
#[must_use]
pub fn ratio_of(a: f64, b: f64) -> f64 {
    (a.max(b) + 0.05) / (a.min(b) + 0.05)
}

/// The three channels of a `#RRGGBB` color, from 0 to 255.
#[must_use]
pub fn channels(hex: &str) -> [f64; 3] {
    std::array::from_fn(|at| {
        let digits = hex.get(1 + at * 2..3 + at * 2).unwrap_or("00");
        f64::from(u8::from_str_radix(digits, 16).unwrap_or(0))
    })
}

/// How bright a `#RRGGBB` color is, by the formula of WCAG.
#[must_use]
pub fn luminance(hex: &str) -> f64 {
    luminance_of(channels(hex))
}

/// How bright a color of three channels (0 to 255) is, by the formula of WCAG.
#[must_use]
pub fn luminance_of(color: [f64; 3]) -> f64 {
    let [red, green, blue] = color.map(|channel| {
        let value = channel / 255.0;
        if value <= 0.03928 {
            value / 12.92
        } else {
            ((value + 0.055) / 1.055).powf(2.4)
        }
    });
    0.2126 * red + 0.7152 * green + 0.0722 * blue
}

/// A `#RRGGBB` color as it is seen when it is drawn with `opacity` over `behind`: its three channels,
/// from 0 to 255.
#[must_use]
pub fn fade(color: &str, behind: &str, opacity: f64) -> [f64; 3] {
    let (over, under) = (channels(color), channels(behind));
    std::array::from_fn(|at| over[at].mul_add(opacity, under[at] * (1.0 - opacity)))
}

/// A color as `filter: brightness(factor)` draws it: each channel times the factor, at most 255.
#[must_use]
pub fn brighten(color: &str, factor: f64) -> [f64; 3] {
    channels(color).map(|channel| (channel * factor).min(255.0))
}

/// How far apart two `#RRGGBB` colors look: the distance in the CIE L*a*b* space (delta E 1976).
/// The contrast ratio only knows brightness, so a grey and a blue of the same brightness have a
/// ratio of 1 and a difference of 10 and more here.
#[must_use]
pub fn color_difference(first: &str, second: &str) -> f64 {
    let (a, b) = (lab(first), lab(second));
    ((a[0] - b[0]).powi(2) + (a[1] - b[1]).powi(2) + (a[2] - b[2]).powi(2)).sqrt()
}

/// How much hue a `#RRGGBB` color has: its chroma in CIE L*a*b* (C*ab); a neutral grey is near 0.
#[must_use]
pub fn chroma_of(hex: &str) -> f64 {
    let [_, a, b] = lab(hex);
    a.hypot(b)
}

/// A `#RRGGBB` color in CIE L*a*b* (D65).
fn lab(hex: &str) -> [f64; 3] {
    let [red, green, blue] = channels(hex).map(|channel| {
        let value = channel / 255.0;
        if value <= 0.04045 {
            value / 12.92
        } else {
            ((value + 0.055) / 1.055).powf(2.4)
        }
    });
    let x = (0.412_456_4 * red + 0.357_576_1 * green + 0.180_437_5 * blue) / 0.950_47;
    let y = 0.212_672_9 * red + 0.715_152_2 * green + 0.072_175 * blue;
    let z = (0.019_333_9 * red + 0.119_192 * green + 0.950_304_1 * blue) / 1.088_83;
    let curve = |value: f64| {
        if value > 216.0 / 24_389.0 {
            value.cbrt()
        } else {
            (24_389.0 / 27.0 * value + 16.0) / 116.0
        }
    };
    let (fx, fy, fz) = (curve(x), curve(y), curve(z));
    [116.0 * fy - 16.0, 500.0 * (fx - fy), 200.0 * (fy - fz)]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn known_ratios() {
        assert!((contrast_ratio("#000000", "#FFFFFF") - 21.0).abs() < 0.01);
        assert!((contrast_ratio("#777777", "#FFFFFF") - 4.48).abs() < 0.01);
    }

    #[test]
    fn a_color_fades_toward_the_color_behind_it() {
        assert_eq!(fade("#000000", "#ffffff", 0.5), [127.5; 3]);
        assert_eq!(fade("#336699", "#ffffff", 1.0), channels("#336699"));
    }

    #[test]
    fn a_grey_has_no_chroma_and_equal_colors_no_difference() {
        assert!(chroma_of("#808080") < 0.01);
        assert!(color_difference("#123456", "#123456") < f64::EPSILON);
    }
}
