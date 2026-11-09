//! The guide to AI for everybody (COMP-11, M7-F90): `userDocs/ai/literacy.md`, built into the
//! program and shown as a page of the app. The text has one home, the file; this module renders it
//! once and gives it to every signed-in person, so an organization on its own servers never has
//! to send anybody to the internet to read it.

use bb_policy::{Authorized, Masked, Place};

use crate::auth::{AppError, AuthStore, Ctx};

/// The guide as it is written in the user docs.
const GUIDE: &str = include_str!("../../../../userDocs/ai/literacy.md");

/// Ask for the guide.
pub struct AiGuideQuery {
    /// The secret of the session of the request.
    pub session_token: String,
}

/// The guide, as safe HTML.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AiGuide {
    /// The HTML of the guide, from `bb-markdown`.
    pub html: String,
}

impl Masked for AiGuide {}

/// Whether a link target is an address of the web or of mail, the only ones that can be followed
/// from the app: the other pages of the user docs are not in the app.
fn is_followable(target: &str) -> bool {
    ["http://", "https://", "mailto:"]
        .iter()
        .any(|scheme| target.starts_with(scheme))
}

/// The text with every link to another page of the user docs written as its words only. A link
/// that goes to the web stays.
fn without_pages_of_the_docs(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(open) = rest.find('[') {
        let after = &rest[open + 1..];
        let link = after.find("](").and_then(|close| {
            let words = &after[..close];
            let tail = &after[close + 2..];
            let end = tail.find(')')?;
            (!words.contains('[') && !words.contains('\n')).then(|| (words, &tail[..end], end))
        });
        match link {
            Some((words, target, end)) if !is_followable(target) => {
                out.push_str(&rest[..open]);
                out.push_str(words);
                rest = &after[words.len() + 2 + end + 1..];
            }
            _ => {
                out.push_str(&rest[..=open]);
                rest = after;
            }
        }
    }
    out.push_str(rest);
    out
}

/// The AI guide for a signed-in person. Nothing in it is private, so any person who is signed in
/// may read it.
///
/// # Errors
/// `auth.not-signed-in` without a session.
pub async fn ai_guide<S: AuthStore>(
    ctx: &Ctx<'_, S>,
    input: AiGuideQuery,
) -> Result<Authorized<AiGuide>, AppError> {
    let actor = ctx.actor(&input.session_token).await?;
    let held = ctx.evaluation_of(&actor, Place::Organization).await?;
    let html = bb_markdown::render(
        &without_pages_of_the_docs(GUIDE),
        &bb_markdown::Options::default(),
    )
    .html;
    Ok(held.authorize(AiGuide { html }))
}

#[cfg(test)]
mod tests {
    use super::without_pages_of_the_docs;

    #[test]
    fn a_link_to_another_page_of_the_docs_keeps_its_words_only() {
        assert_eq!(
            without_pages_of_the_docs("See [the numbers](numbers.md) and [more](../a/b.md)."),
            "See the numbers and more."
        );
    }

    #[test]
    fn a_link_to_the_web_stays() {
        let text = "Read [it](https://example.com/x) now, [a] and [b](";
        assert_eq!(without_pages_of_the_docs(text), text);
    }
}
