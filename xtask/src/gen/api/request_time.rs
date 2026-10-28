//! The time the server gives an API request, as a constant for the browser client.

use anyhow::{Context as _, bail};

use super::TS_HEADER;
use super::web_constants::number_of;
use crate::source::Source;

pub(super) const API_SOURCE: &str = "crates/bb-api/src/api.rs";
const TIME_LIMIT: &str = "TIME_LIMIT";

/// The whole seconds of `pub const NAME: Duration = Duration::from_secs(30);`.
fn seconds_const(file: &syn::File, path: &str, name: &str) -> anyhow::Result<u128> {
    file.items
        .iter()
        .find_map(|item| match item {
            syn::Item::Const(item) if item.ident == name => match &*item.expr {
                syn::Expr::Call(call)
                    if matches!(&*call.func, syn::Expr::Path(function)
                        if function.path.segments.last().is_some_and(|segment| segment.ident == "from_secs"))
                        && call.args.len() == 1 =>
                {
                    call.args.first().and_then(number_of)
                }
                _ => None,
            },
            _ => None,
        })
        .with_context(|| format!("{path} has no `const {name}` that is `Duration::from_secs(..)`"))
}

/// The time the server gives an API request, for the limit of the browser client: it waits a
/// little longer, so the answer of the server (408, `request.timed-out`) is the one that arrives.
pub(super) fn request_time(sources: &[Source]) -> anyhow::Result<String> {
    let Some((_, text)) = sources.iter().find(|(path, _)| path == API_SOURCE) else {
        bail!("{API_SOURCE} is missing");
    };
    let file = syn::parse_file(text).with_context(|| format!("{API_SOURCE} is not Rust"))?;
    let seconds = seconds_const(&file, API_SOURCE, TIME_LIMIT)?;
    Ok(format!(
        "{TS_HEADER}/** How long the server lets an API request take, in milliseconds (`bb_api::TIME_LIMIT`); after it the server answers `request.timed-out`. */
export const serverTimeLimitMs = {};
",
        seconds * 1000
    ))
}
