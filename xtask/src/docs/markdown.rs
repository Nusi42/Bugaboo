//! Markdown to HTML with the page's conventions: numbered headings, mermaid diagrams, wrapped
//! tables, and links between Markdown files turned into in-page anchors.

use pulldown_cmark::{CodeBlockKind, Event, LinkType, Options, Parser, Tag, TagEnd, html};

use super::text::{escape_attribute, escape_text};

/// Where links to other Markdown files point on the page being built.
pub(crate) struct Links {
    /// Prefix for in-page anchors: empty on the architecture page, the architecture page's file
    /// name on other pages.
    pub(crate) page: String,
    /// The file names of the architecture chapters (`04-building-blocks.md`).
    pub(crate) chapters: Vec<String>,
}

impl Links {
    fn chapter_number(&self, href: &str) -> Option<u32> {
        if href.contains("adr/") {
            return None;
        }
        let name = href.rsplit('/').next()?;
        if !self.chapters.iter().any(|chapter| chapter == name) {
            return None;
        }
        name.get(..2)?.parse().ok()
    }

    fn adr_number(href: &str) -> Option<&str> {
        let name = href.rsplit('/').next()?;
        let number = name.get(..4)?;
        let is_adr_name = number.bytes().all(|b| b.is_ascii_digit())
            && name.get(4..5) == Some("-")
            && super::text::has_extension(name, "md");
        let in_adr_folder = href.contains("adr/") || href == name;
        (is_adr_name && in_adr_folder).then_some(number)
    }

    /// The opening tag for a link, and the tag that closes it.
    fn open(&self, href: &str) -> (String, &'static str) {
        if let Some(number) = self.chapter_number(href) {
            return (format!("<a href=\"{}#s{number}\">", self.page), "</a>");
        }
        if let Some(number) = Self::adr_number(href) {
            return (format!("<a href=\"{}#adr-{number}\">", self.page), "</a>");
        }
        if href.starts_with("http://") || href.starts_with("https://") {
            return (
                format!(
                    "<a href=\"{}\" target=\"_blank\" rel=\"noopener\">",
                    escape_attribute(href)
                ),
                "</a>",
            );
        }
        (
            format!(
                "<span title=\"{} in the repository\">",
                escape_attribute(href)
            ),
            "</span>",
        )
    }
}

/// The opening and closing HTML for a heading of the given source level (1 to 6).
pub(crate) type Heading = (String, String);

/// Renders Markdown. `heading` decides how each heading is written, given its source level.
pub(crate) fn render(md: &str, links: &Links, mut heading: impl FnMut(u8) -> Heading) -> String {
    let options =
        Options::ENABLE_TABLES | Options::ENABLE_STRIKETHROUGH | Options::ENABLE_TASKLISTS;
    let mut events: Vec<Event<'_>> = Vec::new();
    let mut parser = Parser::new_ext(md, options);
    let mut closers: Vec<Option<&'static str>> = Vec::new();
    let mut closing_heading: Vec<String> = Vec::new();
    while let Some(event) = parser.next() {
        match event {
            Event::Start(Tag::Heading { level, .. }) => {
                let (open, close) = heading(level as u8);
                events.push(Event::Html(open.into()));
                closing_heading.push(close);
            }
            Event::End(TagEnd::Heading(_)) => {
                let close = closing_heading.pop().unwrap_or_default();
                events.push(Event::Html(format!("{close}\n").into()));
            }
            Event::Start(Tag::CodeBlock(kind)) => {
                let mut code = String::new();
                for inner in parser.by_ref() {
                    match inner {
                        Event::Text(text) => code.push_str(&text),
                        Event::End(TagEnd::CodeBlock) => break,
                        _ => {}
                    }
                }
                events.push(Event::Html(code_block(&kind, &code).into()));
            }
            Event::Start(Tag::Link {
                link_type,
                dest_url,
                title,
                id,
            }) => {
                if matches!(link_type, LinkType::Email | LinkType::Autolink) {
                    closers.push(None);
                    events.push(Event::Start(Tag::Link {
                        link_type,
                        dest_url,
                        title,
                        id,
                    }));
                } else {
                    let (open, close) = links.open(&dest_url);
                    closers.push(Some(close));
                    events.push(Event::Html(open.into()));
                }
            }
            Event::End(TagEnd::Link) => match closers.pop().flatten() {
                Some(close) => events.push(Event::Html(close.into())),
                None => events.push(Event::End(TagEnd::Link)),
            },
            other => events.push(other),
        }
    }
    let mut out = String::new();
    html::push_html(&mut out, wrap_tables(events).into_iter());
    out
}

/// The alignments of a table column that Markdown knows, and the classes of the page's sheet.
const ALIGNMENTS: [(&str, &str); 3] = [
    ("left", "align-left"),
    ("center", "align-center"),
    ("right", "align-right"),
];

/// Renders each table on its own, inside the page's table wrapper.
fn wrap_tables(events: Vec<Event<'_>>) -> Vec<Event<'_>> {
    let mut wrapped = Vec::with_capacity(events.len());
    let mut table: Option<Vec<Event<'_>>> = None;
    for event in events {
        let ends_table = matches!(event, Event::End(TagEnd::Table));
        match (&mut table, event) {
            (Some(inner), event) => inner.push(event),
            (None, event @ Event::Start(Tag::Table(_))) => table = Some(vec![event]),
            (None, event) => wrapped.push(event),
        }
        if ends_table && let Some(inner) = table.take() {
            let mut html = String::new();
            html::push_html(&mut html, inner.into_iter());
            let html = html.replacen("<table>", "<table class=\"dtable\">", 1);
            // The alignment of a column is a class of the sheet, not a style of the cell.
            let html = ALIGNMENTS.iter().fold(html, |html, (side, class)| {
                html.replace(
                    &format!(" style=\"text-align: {side}\""),
                    &format!(" class=\"{class}\""),
                )
            });
            wrapped.push(Event::Html(
                format!("<div class=\"tbl-wrap\">{html}</div>\n").into(),
            ));
        }
    }
    wrapped
}

fn code_block(kind: &CodeBlockKind<'_>, code: &str) -> String {
    let language = match kind {
        CodeBlockKind::Fenced(info) => info.split_whitespace().next().unwrap_or(""),
        CodeBlockKind::Indented => "",
    };
    let code = escape_text(code.strip_suffix('\n').unwrap_or(code));
    if language == "mermaid" {
        format!("<pre class=\"mermaid\">{code}</pre>\n")
    } else {
        format!("<pre><code>{code}</code></pre>\n")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn links() -> Links {
        Links {
            page: String::new(),
            chapters: vec!["04-building-blocks.md".to_owned()],
        }
    }

    fn plain(level: u8) -> Heading {
        (format!("<h{level}>"), format!("</h{level}>"))
    }

    #[test]
    fn mermaid_blocks_keep_their_source_for_the_browser() {
        let html = render("```mermaid\nA --> B\n```\n", &links(), plain);
        assert_eq!(html, "<pre class=\"mermaid\">A --&gt; B</pre>\n");
    }

    #[test]
    fn other_code_is_escaped_and_has_no_language_class() {
        let html = render("```rust\nlet a = 1 < 2;\n```\n", &links(), plain);
        assert_eq!(html, "<pre><code>let a = 1 &lt; 2;</code></pre>\n");
    }

    #[test]
    fn tables_are_wrapped_and_get_the_page_class() {
        let html = render("| a |\n|---|\n| b |\n", &links(), plain);
        assert!(html.starts_with("<div class=\"tbl-wrap\"><table class=\"dtable\">"));
        assert!(html.ends_with("</table>\n</div>\n"));
    }

    #[test]
    fn links_between_markdown_files_become_anchors() {
        let md = "[a](../architecture/04-building-blocks.md) [b](../adr/0020-x.md) \
                  [c](glossary.md) [d](https://example.org/?a=1&b=2) [e](99-other.md)";
        let html = render(md, &links(), plain);
        assert!(html.contains("<a href=\"#s4\">a</a>"));
        assert!(html.contains("<a href=\"#adr-0020\">b</a>"));
        assert!(html.contains("<span title=\"glossary.md in the repository\">c</span>"));
        assert!(html.contains(
            "<a href=\"https://example.org/?a=1&amp;b=2\" target=\"_blank\" rel=\"noopener\">d</a>"
        ));
        assert!(html.contains("<span title=\"99-other.md in the repository\">e</span>"));
    }

    #[test]
    fn other_pages_link_into_the_architecture_page() {
        let links = Links {
            page: "03-architecture.html".to_owned(),
            chapters: vec!["11-testing.md".to_owned()],
        };
        let html = render("[t](architecture/11-testing.md)", &links, plain);
        assert!(html.contains("<a href=\"03-architecture.html#s11\">t</a>"));
    }

    #[test]
    fn headings_use_the_given_markup() {
        let html = render("# Title\n\n## Part\n", &links(), |level| {
            let shifted = level + 1;
            (format!("<h{shifted} id=\"x\">"), format!("</h{shifted}>"))
        });
        assert_eq!(html, "<h2 id=\"x\">Title</h2>\n<h3 id=\"x\">Part</h3>\n");
    }
}
