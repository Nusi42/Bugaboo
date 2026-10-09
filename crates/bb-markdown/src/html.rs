//! The HTML writer (SEC-13): the only place that makes tags, and the allowlist in one.
//!
//! The HTML is *made* from the Markdown events, never read back and cleaned. Only the tags and
//! attributes written here can ever be in the output; every text and every attribute value is
//! escaped; the addresses were checked before (`urls`). There is no raw HTML to sanitize because
//! raw HTML in the text was turned into text before it got here. This also keeps the
//! WebAssembly module small: an HTML parser and cleaner would be most of its size (PERF-04).

use pulldown_cmark::{Event, HeadingLevel, Tag, TagEnd};

use crate::tokens::TokenKind;

/// What the writer is given: Markdown events, and the tokens found in the text.
pub(crate) enum Item {
    /// An event of the parser, with nothing borrowed.
    Event(Event<'static>),
    /// An issue ID or a mention: the page finds it by its index.
    Token {
        index: usize,
        kind: TokenKind,
        text: String,
    },
}

/// The attribute of a token's `span` that says which of [`crate::Rendered::tokens`] it is.
const TOKEN_INDEX: &str = "data-token";
/// The attribute of a token's `span` that says what kind it is.
const TOKEN_KIND: &str = "data-kind";
/// What every link gets, so the page that opened it learns nothing and cannot be reached by it.
const LINK_REL: &str = "noopener noreferrer";
/// Where links open: outside the app.
const LINK_TARGET: &str = "_blank";

/// An image while its description (the `alt` text) is being collected.
struct OpenImage {
    src: String,
    title: String,
    alt: String,
}

/// Writes the items as HTML.
pub(crate) fn write(items: impl IntoIterator<Item = Item>) -> String {
    let mut writer = Writer::default();
    for item in items {
        match item {
            Item::Event(event) => writer.event(event),
            Item::Token { index, kind, text } => {
                writer.raw(&format!(
                    "<span {TOKEN_INDEX}=\"{index}\" {TOKEN_KIND}=\"{}\">",
                    kind.word()
                ));
                writer.text(&text);
                writer.raw("</span>");
            }
        }
    }
    writer.out
}

#[derive(Default)]
struct Writer {
    out: String,
    /// Inside the head of a table: its cells are `th`.
    in_table_head: bool,
    image: Option<OpenImage>,
}

impl Writer {
    fn raw(&mut self, markup: &str) {
        if self.image.is_none() {
            self.out.push_str(markup);
        }
    }

    fn text(&mut self, text: &str) {
        if let Some(image) = &mut self.image {
            image.alt.push_str(text);
        } else {
            escape(text, &mut self.out);
        }
    }

    fn event(&mut self, event: Event<'static>) {
        match event {
            Event::Start(tag) => self.start(tag),
            Event::End(tag) => self.end(tag),
            Event::Text(text) => self.text(&text),
            Event::Code(code) => {
                self.raw("<code>");
                self.text(&code);
                self.raw("</code>");
            }
            Event::SoftBreak => self.text("\n"),
            Event::HardBreak => self.raw("<br>\n"),
            Event::Rule => self.raw("<hr>\n"),
            // Raw HTML was turned into text before; the rest is not enabled.
            _ => {}
        }
    }

    fn start(&mut self, tag: Tag<'static>) {
        match tag {
            Tag::Paragraph => self.raw("<p>"),
            Tag::Heading { level, .. } => self.raw(&format!("<h{}>", heading_number(level))),
            Tag::BlockQuote(_) => self.raw("<blockquote>\n"),
            // The language of a fence is not used: no class, no highlighting.
            Tag::CodeBlock(_) => self.raw("<pre><code>"),
            Tag::List(None) => self.raw("<ul>\n"),
            Tag::List(Some(1)) => self.raw("<ol>\n"),
            Tag::List(Some(first)) => self.raw(&format!("<ol start=\"{first}\">\n")),
            Tag::Item => self.raw("<li>"),
            Tag::Table(_) => self.raw("<table>"),
            Tag::TableHead => {
                self.in_table_head = true;
                self.raw("<thead><tr>");
            }
            Tag::TableRow => self.raw("<tr>"),
            Tag::TableCell => self.raw(if self.in_table_head { "<th>" } else { "<td>" }),
            Tag::Emphasis => self.raw("<em>"),
            Tag::Strong => self.raw("<strong>"),
            Tag::Strikethrough => self.raw("<del>"),
            Tag::Link {
                dest_url, title, ..
            } => {
                let mut markup = String::from("<a href=\"");
                escape(&dest_url, &mut markup);
                markup.push('"');
                attribute(&mut markup, "title", &title);
                attribute(&mut markup, "target", LINK_TARGET);
                attribute(&mut markup, "rel", LINK_REL);
                markup.push('>');
                self.raw(&markup);
            }
            Tag::Image {
                dest_url, title, ..
            } => {
                self.image = Some(OpenImage {
                    src: dest_url.into_string(),
                    title: title.into_string(),
                    alt: String::new(),
                });
            }
            // Not enabled in the parser, or without markup of their own.
            _ => {}
        }
    }

    fn end(&mut self, tag: TagEnd) {
        match tag {
            TagEnd::Paragraph => self.raw("</p>\n"),
            TagEnd::Heading(level) => self.raw(&format!("</h{}>\n", heading_number(level))),
            TagEnd::BlockQuote(_) => self.raw("</blockquote>\n"),
            TagEnd::CodeBlock => self.raw("</code></pre>\n"),
            TagEnd::List(true) => self.raw("</ol>\n"),
            TagEnd::List(false) => self.raw("</ul>\n"),
            TagEnd::Item => self.raw("</li>\n"),
            TagEnd::Table => self.raw("</tbody></table>\n"),
            TagEnd::TableHead => {
                self.in_table_head = false;
                self.raw("</tr></thead><tbody>\n");
            }
            TagEnd::TableRow => self.raw("</tr>\n"),
            TagEnd::TableCell => self.raw(if self.in_table_head { "</th>" } else { "</td>" }),
            TagEnd::Emphasis => self.raw("</em>"),
            TagEnd::Strong => self.raw("</strong>"),
            TagEnd::Strikethrough => self.raw("</del>"),
            TagEnd::Link => self.raw("</a>"),
            TagEnd::Image => {
                if let Some(image) = self.image.take() {
                    let mut markup = String::from("<img src=\"");
                    escape(&image.src, &mut markup);
                    markup.push('"');
                    attribute(&mut markup, "alt", &image.alt);
                    attribute(&mut markup, "title", &image.title);
                    markup.push('>');
                    self.raw(&markup);
                }
            }
            _ => {}
        }
    }
}

const fn heading_number(level: HeadingLevel) -> usize {
    level as usize
}

/// ` name="value"`, left out when the value is empty.
fn attribute(markup: &mut String, name: &str, value: &str) {
    if !value.is_empty() {
        markup.push(' ');
        markup.push_str(name);
        markup.push_str("=\"");
        escape(value, markup);
        markup.push('"');
    }
}

/// Escapes the characters that could end a text or an attribute value.
fn escape(text: &str, out: &mut String) {
    for c in text.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&#39;"),
            other => out.push(other),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn text_and_values_are_escaped() {
        let mut out = String::new();
        escape(r#"<a href="x">'&'</a>"#, &mut out);
        assert_eq!(out, "&lt;a href=&quot;x&quot;&gt;&#39;&amp;&#39;&lt;/a&gt;");
    }

    #[test]
    fn an_empty_value_leaves_the_attribute_out() {
        let mut markup = String::new();
        attribute(&mut markup, "title", "");
        assert_eq!(markup, "");
        attribute(&mut markup, "title", "a\"b");
        assert_eq!(markup, " title=\"a&quot;b\"");
    }
}
