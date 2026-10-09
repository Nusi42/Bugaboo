//! SEC-13: the sanitizer corpus, the attacks, the limits and the tokens (ISS-01).

#![allow(
    clippy::disallowed_methods,
    reason = "the corpus test reads its files; a test is not the pure code of the crate (ARCH-PURE, architecture §4)"
)]

use std::cell::RefCell;
use std::fs;
use std::path::{Path, PathBuf};

use html5ever::tendril::StrTendril;
use html5ever::tokenizer::{
    BufferQueue, Tag, Token, TokenSink, TokenSinkResult, Tokenizer, TokenizerOpts,
};

use bb_markdown::{MAX_NESTING_DEPTH, MAX_TEXT_BYTES, MAX_TOKENS, Options, TokenKind, render};
use proptest::prelude::*;

// Used by the library only (`unused_crate_dependencies`).
use pulldown_cmark as _;

/// Set it to write the expected HTML of a new corpus case; read the file before it is committed.
const BLESS: &str = "BB_MARKDOWN_BLESS";
/// What the HTML may have, written again here on purpose: the test must fail if the writer grows.
/// Each tag with the attributes it may have.
const ALLOWED: [(&str, &[&str]); 27] = [
    ("p", &[]),
    ("br", &[]),
    ("hr", &[]),
    ("h1", &[]),
    ("h2", &[]),
    ("h3", &[]),
    ("h4", &[]),
    ("h5", &[]),
    ("h6", &[]),
    ("blockquote", &[]),
    ("pre", &[]),
    ("code", &[]),
    ("em", &[]),
    ("strong", &[]),
    ("del", &[]),
    ("a", &["href", "title", "target", "rel"]),
    ("img", &["src", "alt", "title"]),
    ("ul", &[]),
    ("ol", &["start"]),
    ("li", &[]),
    ("table", &[]),
    ("thead", &[]),
    ("tbody", &[]),
    ("tr", &[]),
    ("th", &[]),
    ("td", &[]),
    ("span", &["data-token", "data-kind"]),
];
/// The schemes an address with a scheme may have.
const SCHEMES: [&str; 3] = ["http", "https", "mailto"];

/// Collects the tags a real HTML tokenizer (html5ever, the parser browsers' rules come from) reads.
#[derive(Default)]
struct Tags(RefCell<Vec<Tag>>);

impl TokenSink for Tags {
    type Handle = ();

    fn process_token(&self, token: Token, _line: u64) -> TokenSinkResult<()> {
        if let Token::TagToken(tag) = token {
            self.0.borrow_mut().push(tag);
        }
        TokenSinkResult::Continue
    }
}

/// Whether an address of `href` or `src` is one a browser may follow, read the way a browser reads
/// it: tabs and line breaks inside are dropped, an address has a scheme only when the letters
/// before the first colon are a valid scheme name, and `\` counts as `/`.
fn address_is_fine(value: &str) -> bool {
    let cleaned: String = value
        .trim_matches(|c: char| c.is_ascii_control() || c == ' ')
        .chars()
        .filter(|c| !matches!(c, '\t' | '\n' | '\r'))
        .map(|c| if c == '\\' { '/' } else { c })
        .collect::<String>()
        .to_ascii_lowercase();
    let name_end = cleaned.find(|c: char| !(c.is_ascii_alphanumeric() || "+-.".contains(c)));
    match name_end {
        Some(at)
            if at > 0
                && cleaned.as_bytes()[0].is_ascii_alphabetic()
                && cleaned.as_bytes()[at] == b':' =>
        {
            SCHEMES.contains(&&cleaned[..at])
        }
        _ => !cleaned.starts_with("//"),
    }
}

/// What is wrong with `html` when a real HTML tokenizer reads it: a tag, an attribute or an address
/// that the allow list does not have. Empty means fine.
fn problems_in(html: &str) -> Vec<String> {
    let input = BufferQueue::default();
    input.push_back(StrTendril::from_slice(html));
    let tokenizer = Tokenizer::new(Tags::default(), TokenizerOpts::default());
    let _ = tokenizer.feed(&input);
    tokenizer.end();
    let mut problems = Vec::new();
    for tag in tokenizer.sink.0.borrow().iter() {
        let name: &str = &tag.name;
        let Some((_, attributes)) = ALLOWED.iter().find(|(allowed, _)| *allowed == name) else {
            problems.push(format!("tag <{name}>"));
            continue;
        };
        for attribute in &tag.attrs {
            let attribute_name: &str = &attribute.name.local;
            if !attributes.contains(&attribute_name) {
                problems.push(format!("attribute {attribute_name} in <{name}>"));
            } else if ["href", "src"].contains(&attribute_name)
                && !address_is_fine(&attribute.value)
            {
                problems.push(format!("address {} in <{name}>", &*attribute.value));
            }
        }
    }
    problems
}

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
fn more_tokens_than_the_limit_stay_plain_text() {
    let text = "@a ".repeat(MAX_TOKENS + 500);
    let rendered = render(&text, &Options::default());
    assert_eq!(rendered.tokens.len(), MAX_TOKENS);
    assert_eq!(rendered.html.matches("data-token=").count(), MAX_TOKENS);
    // The cost of the tokens is bounded by their number, not by how short the text is.
    assert!(rendered.html.len() < text.len() + MAX_TOKENS * 100);
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

/// Pieces of Markdown and attacks between any characters at all.
fn mixed() -> impl Strategy<Value = String> {
    (fragments(), "\\PC{0,200}", fragments()).prop_map(|(a, b, c)| format!("{a}{b}{c}"))
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
    fn sec_13_whatever_is_written_the_html_has_only_allowed_tags_attributes_and_addresses(
        text in mixed(),
        arbitrary in "\\PC{0,400}",
    ) {
        let options = Options { image_hosts: vec!["images.example.com".to_owned()] };
        for input in [text, arbitrary] {
            let html = render(&input, &options).html;
            prop_assert_eq!(problems_in(&html), Vec::<String>::new(), "{}", input);
        }
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
