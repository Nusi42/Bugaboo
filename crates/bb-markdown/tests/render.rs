//! SEC-13: the sanitizer corpus, the attacks, the limits and the tokens (ISS-01).

use std::fs;
use std::path::{Path, PathBuf};

use bb_markdown::{MAX_NESTING_DEPTH, MAX_TEXT_BYTES, Options, TokenKind, render};
use proptest::prelude::*;

// Used by the library only (`unused_crate_dependencies`).
use pulldown_cmark as _;

/// Set it to write the expected HTML of a new corpus case; read the file before it is committed.
const BLESS: &str = "BB_MARKDOWN_BLESS";
/// The tags the sanitizer allows, written again here on purpose: the test must fail if the
/// allowlist grows.
const ALLOWED_TAGS: [&str; 28] = [
    "p",
    "br",
    "hr",
    "h1",
    "h2",
    "h3",
    "h4",
    "h5",
    "h6",
    "blockquote",
    "pre",
    "code",
    "em",
    "strong",
    "del",
    "a",
    "img",
    "ul",
    "ol",
    "li",
    "table",
    "thead",
    "tbody",
    "tr",
    "th",
    "td",
    "span",
    "sup",
];
/// The attributes the sanitizer allows, again on purpose.
const ALLOWED_ATTRIBUTES: [&str; 9] = [
    "href",
    "title",
    "src",
    "alt",
    "start",
    "rel",
    "target",
    "data-token",
    "data-kind",
];

fn corpus_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/corpus")
}

/// The options of a corpus case: the hosts in `name.hosts`, one per line, if the file is there.
fn options_of(case: &Path) -> Options {
    let hosts = fs::read_to_string(case.with_extension("hosts")).unwrap_or_default();
    Options {
        image_hosts: hosts.lines().map(str::to_owned).collect(),
    }
}

/// Every tag of `html` and the attributes inside it, with what is not allowed named.
fn problems_in(html: &str) -> Vec<String> {
    let mut problems = Vec::new();
    for piece in html.split('<').skip(1) {
        let tag_end = piece.find('>').unwrap_or(piece.len());
        let tag = &piece[..tag_end];
        let name = tag
            .trim_start_matches('/')
            .split(|c: char| !c.is_ascii_alphanumeric())
            .next()
            .unwrap_or_default();
        if !ALLOWED_TAGS.contains(&name) {
            problems.push(format!("tag <{name}>"));
        }
        for attribute in tag.split_whitespace().skip(1) {
            let attribute_name = attribute.split('=').next().unwrap_or_default();
            if !ALLOWED_ATTRIBUTES.contains(&attribute_name) && !attribute.ends_with('"') {
                problems.push(format!("attribute {attribute_name} in <{name}>"));
            }
        }
        for attribute in tag.split_whitespace().skip(1) {
            let value = attribute.split_once("=\"").map_or("", |(_, value)| value);
            for danger in ["javascript:", "data:", "vbscript:"] {
                if value.to_ascii_lowercase().starts_with(danger) {
                    problems.push(format!("{danger} in <{name}>"));
                }
            }
        }
    }
    problems
}

#[test]
fn sec_13_the_sanitizer_corpus_renders_every_attack_as_text() {
    let mut cases: Vec<PathBuf> = fs::read_dir(corpus_dir())
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .filter(|path| path.extension().is_some_and(|extension| extension == "md"))
        .collect();
    cases.sort();
    assert!(cases.len() >= 10, "the corpus is there");
    for case in cases {
        let text = fs::read_to_string(&case).unwrap();
        let html = render(&text, &options_of(&case)).html;
        assert_eq!(
            problems_in(&html),
            Vec::<String>::new(),
            "{}",
            case.display()
        );
        let expected_path = case.with_extension("html");
        if std::env::var_os(BLESS).is_some() {
            fs::write(&expected_path, &html).unwrap();
        }
        let expected = fs::read_to_string(&expected_path).unwrap();
        assert_eq!(html, expected, "{}", case.display());
    }
}

#[test]
fn sec_13_a_script_in_a_comment_shows_as_text() {
    let html = render("<script>alert(1)</script>", &Options::default()).html;
    assert!(
        html.contains("&lt;script&gt;alert(1)&lt;/script&gt;"),
        "{html}"
    );
    assert!(!html.contains("<script"));
}

#[test]
fn sec_13_links_open_outside_and_say_nothing_about_us() {
    let html = render("[a](https://example.com)", &Options::default()).html;
    assert!(html.contains(r#"rel="noopener noreferrer""#), "{html}");
    assert!(html.contains(r#"target="_blank""#), "{html}");
}

#[test]
fn sec_13_only_our_own_origin_and_allowed_hosts_show_images() {
    let allowed = Options {
        image_hosts: vec!["images.example.com".to_owned()],
    };
    let with_host = render("![a](https://images.example.com/a.png)", &allowed).html;
    assert!(with_host.contains("<img"), "{with_host}");
    let without = render(
        "![a](https://images.example.com/a.png)",
        &Options::default(),
    )
    .html;
    assert!(!without.contains("<img"), "{without}");
    assert!(
        without.contains("<a "),
        "a denied image stays reachable as a link: {without}"
    );
    assert!(
        render("![a](/files/a.png)", &Options::default())
            .html
            .contains("<img")
    );
}

#[test]
fn a_text_over_the_limit_is_cut_and_says_so() {
    let long = "é".repeat(MAX_TEXT_BYTES);
    let rendered = render(&long, &Options::default());
    assert!(rendered.truncated);
    assert!(rendered.html.len() <= MAX_TEXT_BYTES * 2 + 16);
    assert!(!render("short", &Options::default()).truncated);
}

#[test]
fn nesting_deeper_than_the_limit_is_flattened() {
    let deep = format!("{}deep", "> ".repeat(MAX_NESTING_DEPTH * 4));
    let html = render(&deep, &Options::default()).html;
    assert_eq!(html.matches("<blockquote>").count(), MAX_NESTING_DEPTH);
    assert!(html.contains("deep"));
    assert_eq!(
        html.matches("<blockquote>").count(),
        html.matches("</blockquote>").count()
    );
}

fn fragments() -> impl Strategy<Value = String> {
    let pieces = prop::sample::select(vec![
        "WEB-142",
        "WEB-K7Q2X",
        "@ana",
        "@Bo.x",
        " ",
        "\n",
        "\n\n",
        "*",
        "**",
        "`",
        "```\n",
        "[",
        "](",
        ")",
        "x",
        "é",
        "<b>",
        "<script>alert(1)</script>",
        "<img src=x onerror=1>",
        "> ",
        "- ",
        "1. ",
        "\\",
        "![i](",
        "https://a.example/",
        "javascript:",
        "|",
        "-",
        "&amp;",
        "&#106;",
    ]);
    prop::collection::vec(pieces, 0..40).prop_map(|pieces| pieces.concat())
}

proptest! {
    #[test]
    fn iss_01_issue_ids_and_mentions_come_back_as_tokens_with_their_places(text in fragments()) {
        let rendered = render(&text, &Options::default());
        let mut end_of_last = 0;
        for token in &rendered.tokens {
            prop_assert!(token.start >= end_of_last, "tokens overlap or are out of order");
            prop_assert!(token.start < token.end);
            prop_assert!(token.end <= text.len());
            prop_assert!(text.is_char_boundary(token.start) && text.is_char_boundary(token.end));
            let piece = &text[token.start..token.end];
            match token.kind {
                TokenKind::Mention => prop_assert!(piece.starts_with('@') && piece.len() > 1),
                TokenKind::IssueId => prop_assert!(piece.contains('-')),
            }
            end_of_last = token.end;
        }
        prop_assert_eq!(rendered.html.matches("data-token=").count(), rendered.tokens.len());
    }

    #[test]
    fn sec_13_whatever_is_written_the_html_has_only_allowed_tags_and_attributes(text in fragments()) {
        let html = render(&text, &Options::default()).html;
        prop_assert_eq!(problems_in(&html), Vec::<String>::new());
    }

    #[test]
    fn any_text_renders_the_same_twice(text in "\\PC{0,300}") {
        let options = Options::default();
        prop_assert_eq!(render(&text, &options), render(&text, &options));
    }
}

#[test]
fn issue_ids_and_mentions_have_their_places_in_a_sentence() {
    let text = "Ask @ana about WEB-K7Q2X today.";
    let rendered = render(text, &Options::default());
    let pieces: Vec<_> = rendered
        .tokens
        .iter()
        .map(|t| (t.kind, &text[t.start..t.end]))
        .collect();
    assert_eq!(
        pieces,
        [
            (TokenKind::Mention, "@ana"),
            (TokenKind::IssueId, "WEB-K7Q2X")
        ]
    );
    assert!(
        rendered
            .html
            .contains(r#"<span data-token="1" data-kind="issue-id">WEB-K7Q2X</span>"#)
    );
}
