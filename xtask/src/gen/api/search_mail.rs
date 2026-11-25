//! The bounds of e-mailed searches (QRY-07) as constants for the browser client.

use anyhow::{Context as _, bail};

use super::TS_HEADER;
use super::web_constants::number_const;
use crate::source::Source;

/// Where the bounds are defined.
pub(super) const SEARCH_MAIL_SOURCE: &str = "crates/bb-core/src/search_mail.rs";

/// The last hour of a day a person may choose for an e-mailed search (`LAST_HOUR`), for the window
/// *E-mail me this search*.
pub(super) fn search_mail(sources: &[Source]) -> anyhow::Result<String> {
    let Some((_, text)) = sources.iter().find(|(path, _)| path == SEARCH_MAIL_SOURCE) else {
        bail!("{SEARCH_MAIL_SOURCE} is missing");
    };
    let file =
        syn::parse_file(text).with_context(|| format!("{SEARCH_MAIL_SOURCE} is not Rust"))?;
    Ok(format!(
        "{TS_HEADER}/** The last hour of a day an e-mailed search may go at (`bb_core::search_mail::LAST_HOUR`); the first is 0. */
export const mailLastHour = {};
",
        number_const(&file, SEARCH_MAIL_SOURCE, "LAST_HOUR")?,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn qry_07_the_last_hour_comes_from_the_constant_of_the_core() {
        let source = (
            SEARCH_MAIL_SOURCE.to_owned(),
            "pub const LAST_HOUR: i8 = 23;".to_owned(),
        );
        let written = search_mail(&[source]).unwrap();
        assert!(
            written.contains("export const mailLastHour = 23;"),
            "{written}"
        );
        assert!(search_mail(&[]).is_err());
    }
}
