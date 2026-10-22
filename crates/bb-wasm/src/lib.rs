//! WebAssembly bindings for the search language, Markdown and validation (ADR-0005, ADR-0011).
//!
//! This crate only passes calls through to the pure crates, so the browser runs the very code the
//! server runs. It holds no rule of its own. The browser loads the built module (`cargo xtask
//! wasm`) only when a preview or an editor opens (coding-standards §6).

#![forbid(unsafe_code)]

pub mod names;

use bb_markdown::Options;
use wasm_bindgen::prelude::wasm_bindgen;

/// Renders Markdown to safe HTML, exactly as the server does (`bb_markdown::render`).
/// `image_hosts` are the hosts of the organization setting `content.image-hosts`.
#[must_use]
#[wasm_bindgen]
pub fn render(text: &str, image_hosts: Vec<String>) -> String {
    bb_markdown::render(
        text,
        &Options {
            image_hosts,
            ..Options::default()
        },
    )
    .html
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_browser_gets_the_html_of_the_server() {
        let text = "**hi** <script>alert(1)</script>";
        assert_eq!(
            render(text, Vec::new()),
            bb_markdown::render(text, &Options::default()).html
        );
    }
}
