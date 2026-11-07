//! SEC-RICHTEXT, AI-05: the answer of the assistant goes through the one writer with the option for
//! AI answers: no picture is ever loaded, no issue ID or name becomes a chip, and every link shows
//! its full address.

use bb_markdown::{Options, render};

// Used by other test files or by the library only (`unused_crate_dependencies`).
use html5ever as _;
use proptest as _;
use pulldown_cmark as _;

fn ai() -> Options {
    Options {
        ai_answer: true,
        // The option wins over the setting: even an allowed host shows no picture.
        image_hosts: vec!["images.example.com".to_owned()],
        ..Options::default()
    }
}

#[test]
fn ai_05_an_answer_never_loads_a_picture() {
    for text in [
        "![x](https://images.example.com/a.png)",
        "![x](https://evil.example/p.png)",
        "![x](/files/a.png)",
        "![x][r]\n\n[r]: https://evil.example/p.png",
        "<img src=\"https://evil.example/q.png\">",
    ] {
        let html = render(text, &ai()).html;
        assert!(!html.contains("<img"), "{text}: {html}");
    }
}

#[test]
fn ai_05_a_picture_shows_its_full_address_as_a_link() {
    let html = render("![x](https://evil.example/p.png)", &ai()).html;
    assert!(
        html.contains(">x (https://evil.example/p.png)</a>"),
        "{html}"
    );
}

#[test]
fn ai_05_a_link_shows_its_address_beside_its_words() {
    let html = render("[the docs](https://example.com/docs?a=1&b=2)", &ai()).html;
    assert!(
        html.contains("the docs (https://example.com/docs?a=1&amp;b=2)</a>"),
        "{html}"
    );
    // Without the option the link is as it was.
    let plain = render("[the docs](https://example.com/docs)", &Options::default()).html;
    assert!(plain.contains(">the docs</a>"), "{plain}");
}

#[test]
fn ai_05_a_link_that_names_another_address_cannot_hide_it() {
    let html = render("[https://example.com](https://evil.example/login)", &ai()).html;
    assert!(html.contains("(https://evil.example/login)"), "{html}");
}

#[test]
fn ai_05_an_address_in_angle_brackets_is_written_once() {
    let html = render("<https://example.com/x>", &ai()).html;
    assert_eq!(html.matches("https://example.com/x").count(), 2, "{html}");
    assert!(!html.contains("(https://example.com/x)"), "{html}");
}

#[test]
fn ai_05_an_issue_id_or_a_name_is_text_and_not_a_chip() {
    let rendered = render("Ask @ana about WEB-142.", &ai());
    assert!(!rendered.html.contains("<span"), "{}", rendered.html);
    assert!(rendered.tokens.is_empty());
    assert!(rendered.html.contains("@ana"));
}

#[test]
fn ai_05_an_address_that_runs_code_is_no_link() {
    let html = render("[x](javascript:alert(1))", &ai()).html;
    assert!(!html.contains("<a "), "{html}");
}
