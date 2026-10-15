//! Markdown to safe HTML and issue-ID chips. No I/O; also built for WebAssembly (ADR-0011).
//!
//! Every text a person writes (descriptions, comments, long-text fields) goes through [`render`],
//! on the server and, built for WebAssembly, in the browser's preview. The steps are the same
//! everywhere, so what is previewed is what others see:
//!
//! 1. The text is cut at [`MAX_TEXT_BYTES`] and parsed as Markdown (`pulldown-cmark`).
//! 2. Raw HTML in the text is not HTML: it is turned into text and shows as text. Images from a
//!    host that is not allowed become links. Blocks nested deeper than [`MAX_NESTING_DEPTH`] are
//!    flattened.
//! 3. Issue IDs (`WEB-142`, `WEB-K7Q2X`) and mentions (`@ana`) in the text become *tokens*: the
//!    HTML holds a `span` for each, and [`Rendered::tokens`] says where each one is in the text.
//!    The server resolves tokens with permission checks; the browser shows them as chips.
//! 4. The HTML is *written* from what is left, by one writer that knows only the tags and
//!    attributes of the `html` module, escapes every text and value, and is given only addresses
//!    that `urls` allows. Nothing is parsed back and cleaned, so there is nothing to slip through.

#![forbid(unsafe_code)]

mod html;
mod render;
mod tokens;
mod urls;

pub use render::render;
pub use tokens::{Token, TokenKind};

/// The most bytes of text that are rendered. A longer text is cut (at a character) and
/// [`Rendered::truncated`] says so. The routes that store text refuse a longer text before it
/// gets here; this is the second line of defence for a renderer that must never be slow.
///
/// The routes count characters (`LONG_TEXT_MAX`, 100,000) and a character takes up to four bytes,
/// so this is four times that: a text the routes keep is never shown cut off (a test in `bb-app`
/// pins the two together).
pub const MAX_TEXT_BYTES: usize = 400_000;

/// The deepest nesting of blocks and emphasis that keeps its structure. What is nested deeper is
/// flattened into the text of the level above, so a hostile text cannot make the browser or the
/// server build a tower of elements.
pub const MAX_NESTING_DEPTH: usize = 16;

/// The most characters of a name after `@` that count as a mention.
pub const MAX_MENTION_CHARS: usize = 64;

/// The most tokens one text can hold. More issue IDs and mentions than this stay plain text: each
/// token costs a `span` in the HTML, and a hostile text of nothing but mentions would otherwise
/// make an answer many times the size of the text.
pub const MAX_TOKENS: usize = 2_000;

/// The most mentions of one text that the server tries to name; the rest stay plain text. Each
/// distinct name costs a search of the people, so a text of two thousand different names must not
/// make every page load that shows it expensive for every reader (SEC-06, DoS).
pub const MAX_RESOLVED_MENTIONS: usize = 50;

/// The most issue IDs of one text that the server looks up; the rest stay plain text.
pub const MAX_RESOLVED_ISSUE_IDS: usize = 100;

/// What the renderer needs to know about the organization.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Options {
    /// The hosts from which images may be shown (the setting `content.image-hosts`), such as
    /// `images.example.com`. Images of our own origin (a path that starts with a single `/`) are
    /// always allowed. Empty means no image from outside. The caller makes the list from the setting
    /// with `bb_core::settings::list_items`, the one reading of a list in a text; this crate has no
    /// second parser of it.
    pub image_hosts: Vec<String>,
    /// The places (in [`Rendered::tokens`], counting from 0) of the tokens that stay plain text:
    /// the server renders a text once to find its tokens, decides which of them it may resolve
    /// (an issue the reader may see, a person the writer may name), and renders it again with
    /// the others here, so a hidden issue ID or an unknown name is text and not a chip. The
    /// places do not move: a token that stays plain still has its place in the list. The browser
    /// preview leaves it empty.
    pub plain_tokens: Vec<usize>,
}

/// The result of [`render`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Rendered {
    /// Safe HTML: no script, no event handler, no `javascript:` or `data:` link, no raw HTML of
    /// the person. Fit to be put on a page as it is.
    pub html: String,
    /// The issue IDs and mentions in the text, in order of their place, never overlapping.
    pub tokens: Vec<Token>,
    /// The text was longer than [`MAX_TEXT_BYTES`] and the rest was not rendered.
    pub truncated: bool,
}

// Used by the integration tests in `tests/` only (`unused_crate_dependencies`).
#[cfg(test)]
use html5ever as _;
#[cfg(test)]
use proptest as _;
