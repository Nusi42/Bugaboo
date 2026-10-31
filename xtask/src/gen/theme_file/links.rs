//! A color that names another color (`color "hover-line" "$border-control"`, M5-F121).
//!
//! The style sheet keeps the link (`var(--border-control)`), so the browser follows it. The contrast
//! check needs the color the link leads to, in each theme and mode; it is found here, and loading the
//! theme refuses a link to a name that is no color and a loop.

use super::{ThemeFile, TokenKind};

/// Which value of a theme a link is followed in.
#[derive(Clone, Copy)]
pub(crate) enum Mode {
    /// The usual colors.
    Usual,
    /// The colors when the device asks for more contrast: a token's own `more=` value where it has
    /// one, else its usual value. A link without a `more=` of its own leads to the `more=` value of
    /// the color it names, as the browser draws it.
    More,
}

/// The color `name` has in `theme`, with every link followed; the problem in words when a link
/// names no color or the colors name each other in a loop.
pub(crate) fn follow<'a>(
    file: &'a ThemeFile,
    name: &str,
    theme: &str,
    mode: Mode,
) -> Result<&'a str, String> {
    let mut path = vec![name.to_owned()];
    let mut current = name.to_owned();
    loop {
        let token = file
            .token(&current)
            .filter(|token| token.kind == TokenKind::Color)
            .ok_or_else(|| format!("`${current}` is no color token"))?;
        let value = match mode {
            Mode::Usual => token.in_theme(theme),
            Mode::More => token.in_theme_with_more_contrast(theme),
        }
        .ok_or_else(|| format!("`{current}` has no value in the theme {theme}"))?;
        let Some(next) = value.strip_prefix('$') else {
            return Ok(value);
        };
        let looped = path.iter().any(|known| known == next);
        path.push(next.to_owned());
        if looped {
            return Err(format!(
                "the colors name each other in a loop: {}",
                path.join(" -> ")
            ));
        }
        next.clone_into(&mut current);
    }
}

/// Every link of every color leads to a color, in every theme and mode.
pub(super) fn check(file: &ThemeFile) -> anyhow::Result<()> {
    for color in file.colors() {
        for value in &color.themes {
            for mode in [Mode::Usual, Mode::More] {
                follow(file, &color.name, &value.theme, mode).map_err(|problem| {
                    anyhow::anyhow!(
                        "{}: line {}: `{}` in the theme {}: {problem}",
                        value.path,
                        value.line,
                        color.name,
                        value.theme
                    )
                })?;
            }
        }
    }
    Ok(())
}
