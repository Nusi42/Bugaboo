//! Markdown to safe HTML: the event pipeline between the parser and the allowlist.

use pulldown_cmark::{
    CodeBlockKind, Event, LinkType, Options as ParserOptions, Parser, Tag, TagEnd,
};

use crate::html::{self, Item};
use crate::tokens::{self, Token};
use crate::urls;
use crate::{MAX_NESTING_DEPTH, MAX_TEXT_BYTES, Options, Rendered};

/// What the parser understands besides plain `CommonMark`: tables and ~~strikethrough~~.
const PARSER_OPTIONS: ParserOptions =
    ParserOptions::ENABLE_TABLES.union(ParserOptions::ENABLE_STRIKETHROUGH);

/// Turns `text` into safe HTML and finds its issue IDs and mentions. See the crate documentation
/// for the steps; the same function runs on the server and, built for WebAssembly, in the browser.
#[must_use]
pub fn render(text: &str, options: &Options) -> Rendered {
    let truncated = text.len() > MAX_TEXT_BYTES;
    let text = if truncated {
        &text[..text.floor_char_boundary(MAX_TEXT_BYTES)]
    } else {
        text
    };
    let mut state = State::default();
    let mut items = Vec::new();
    for (event, range) in Parser::new_ext(text, PARSER_OPTIONS).into_offset_iter() {
        state.accept(
            event,
            &text[range.clone()],
            range.start,
            options,
            &mut items,
        );
    }
    Rendered {
        html: html::write(items),
        tokens: state.tokens,
        truncated,
    }
}

/// An image that is not allowed and shows as a link to its address.
struct ImageAsLink {
    url: String,
    has_text: bool,
}

/// What the pipeline remembers while the events go by.
#[derive(Default)]
struct State {
    /// For each open tag: was its start kept (the nesting was not too deep)?
    kept: Vec<bool>,
    /// Inside a code block: no tokens there.
    in_code_block: bool,
    /// For each open link: the image it was made from, if it was one.
    links: Vec<Option<ImageAsLink>>,
    /// For each open image: was it turned into a link?
    images: Vec<bool>,
    tokens: Vec<Token>,
}

impl State {
    fn accept(
        &mut self,
        event: Event<'_>,
        source: &str,
        offset: usize,
        options: &Options,
        out: &mut Vec<Item>,
    ) {
        match event {
            Event::Start(tag) => self.start(tag, options, out),
            Event::End(tag) => self.end(tag, out),
            // Raw HTML is no HTML: it is text, and shows as text (SEC-13).
            Event::Html(html) | Event::InlineHtml(html) => {
                self.mark_text();
                out.push(Item::Event(Event::Text(html.into_string().into())));
            }
            Event::Text(text) => {
                self.mark_text();
                let plain_place =
                    self.in_code_block || !self.links.is_empty() || !self.images.is_empty();
                if plain_place || *text != *source {
                    out.push(Item::Event(Event::Text(text.into_string().into())));
                } else {
                    self.with_tokens(&text, offset, out);
                }
            }
            Event::Code(code) => {
                self.mark_text();
                out.push(Item::Event(Event::Code(code.into_string().into())));
            }
            other => out.push(Item::Event(other.into_static())),
        }
    }

    /// Keeps the start of a tag unless it is nested too deep; the end follows the start.
    fn start(&mut self, tag: Tag<'_>, options: &Options, out: &mut Vec<Item>) {
        let mut dropped = false;
        let tag = match tag {
            // An image whose address is not allowed becomes a link to that address.
            Tag::Image {
                dest_url,
                title,
                id,
                ..
            } if !urls::image_allowed(&dest_url, &options.image_hosts) => {
                self.images.push(true);
                dropped = !urls::link_allowed(&dest_url);
                self.links.push(Some(ImageAsLink {
                    url: dest_url.to_string(),
                    has_text: false,
                }));
                Tag::Link {
                    link_type: LinkType::Inline,
                    dest_url,
                    title,
                    id,
                }
            }
            Tag::Image { .. } => {
                self.images.push(false);
                tag
            }
            Tag::Link { ref dest_url, .. } => {
                self.links.push(None);
                dropped = !urls::link_allowed(dest_url);
                tag
            }
            // A block of raw HTML is shown as a block of code: text and layout as written.
            Tag::HtmlBlock => {
                self.in_code_block = true;
                Tag::CodeBlock(CodeBlockKind::Indented)
            }
            Tag::CodeBlock(_) => {
                self.in_code_block = true;
                tag
            }
            other => other,
        };
        let keep = self.kept.len() < MAX_NESTING_DEPTH && !dropped;
        self.kept.push(keep);
        if keep {
            out.push(Item::Event(Event::Start(tag.into_static())));
        }
    }

    fn end(&mut self, tag: TagEnd, out: &mut Vec<Item>) {
        let keep = self.kept.pop().unwrap_or(false);
        let tag = match tag {
            TagEnd::Image if self.images.pop() == Some(true) => TagEnd::Link,
            TagEnd::HtmlBlock => TagEnd::CodeBlock,
            other => other,
        };
        match tag {
            TagEnd::CodeBlock => self.in_code_block = false,
            TagEnd::Link => {
                // A link made of an image without a description shows its address.
                if let Some(Some(image)) = self.links.pop()
                    && !image.has_text
                    && keep
                {
                    out.push(Item::Event(Event::Text(image.url.into())));
                }
            }
            _ => {}
        }
        if keep {
            out.push(Item::Event(Event::End(tag)));
        }
    }

    /// Notes that text came: an image shown as a link needs its address as text otherwise.
    fn mark_text(&mut self) {
        if let Some(Some(image)) = self.links.last_mut() {
            image.has_text = true;
        }
    }

    /// Splits a piece of text around its tokens: the token is a `span` the page can find.
    fn with_tokens(&mut self, text: &str, offset: usize, out: &mut Vec<Item>) {
        let mut done = 0;
        for token in tokens::find(text, offset) {
            let start = token.start - offset;
            let end = token.end - offset;
            if start > done {
                out.push(Item::Event(Event::Text(
                    text[done..start].to_owned().into(),
                )));
            }
            out.push(Item::Token {
                index: self.tokens.len(),
                kind: token.kind,
                text: text[start..end].to_owned(),
            });
            self.tokens.push(token);
            done = end;
        }
        if done < text.len() {
            out.push(Item::Event(Event::Text(text[done..].to_owned().into())));
        }
    }
}
