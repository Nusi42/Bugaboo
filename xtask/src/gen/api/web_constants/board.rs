//! The highest card limit of a board, for the browser client.

use anyhow::{Context as _, bail};

use super::{TS_HEADER, number_const};
use crate::source::Source;

const SAVED_SEARCH_SOURCE: &str = "crates/bb-core/src/saved_search.rs";

/// The highest card limit of a column of a board, as a constant for the browser client: the box of a
/// card limit and its words say this number and do not write it a second time.
pub(crate) fn board(sources: &[Source]) -> anyhow::Result<String> {
    let Some((_, text)) = sources.iter().find(|(path, _)| path == SAVED_SEARCH_SOURCE) else {
        bail!("{SAVED_SEARCH_SOURCE} is missing");
    };
    let file =
        syn::parse_file(text).with_context(|| format!("{SAVED_SEARCH_SOURCE} is not Rust"))?;
    let most = number_const(&file, SAVED_SEARCH_SOURCE, "CARD_LIMIT_MAX")?;
    Ok(format!(
        "{TS_HEADER}/** The highest card limit of a column of a board (`CARD_LIMIT_MAX` of the server). */
export const cardLimitMax = {most};
"
    ))
}
