//! Every implemented check. Each one lives in its own file and is registered in [`registry`].
//!
//! To add a check (ADR-0020): write the rule in the docs, add the catalogue entry, implement
//! [`Check`] here, and add a failing and a passing example (`bad_example` / `good_example` tests
//! or `tools/checks/fixtures/<ID>/`).

mod build_location;
mod dev_ports;
mod fmt_css;
mod fmt_ts;
mod lint_css;
mod lint_html;
mod lint_ts;
mod lint_types;
mod meta_catalog;
mod node_tool;
mod sty_contrast;
mod txt_catalog;

use crate::catalog::Catalog;
use crate::repo::Repo;

/// One problem a check found.
pub(crate) struct Finding {
    /// Where the problem is (a file, `file:line`, or a catalogue ID), if it has a place.
    pub(crate) location: Option<String>,
    /// What is wrong and how to fix it, in plain words.
    pub(crate) message: String,
}

impl Finding {
    pub(crate) fn at(location: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            location: Some(location.into()),
            message: message.into(),
        }
    }

    pub(crate) fn general(message: impl Into<String>) -> Self {
        Self {
            location: None,
            message: message.into(),
        }
    }
}

/// What a check can see while it runs.
pub(crate) struct Context<'a> {
    pub(crate) repo: &'a Repo,
    pub(crate) catalog: &'a Catalog,
    /// The files that triggered this run, or `None` when the whole repository is checked.
    pub(crate) files: Option<&'a [String]>,
}

/// A check from the catalogue. An empty list of findings means it passed.
pub(crate) trait Check {
    fn id(&self) -> &'static str;
    fn run(&self, context: &Context<'_>) -> anyhow::Result<Vec<Finding>>;
}

/// Every implemented check, in catalogue order.
pub(crate) fn registry() -> Vec<Box<dyn Check>> {
    vec![
        Box::new(meta_catalog::MetaCatalog),
        Box::new(build_location::BuildLocation),
        Box::new(fmt_ts::FmtTs),
        Box::new(fmt_css::FmtCss),
        Box::new(lint_ts::LintTs),
        Box::new(lint_types::LintTypes),
        Box::new(lint_css::LintCss),
        Box::new(lint_html::LintHtml),
        Box::new(dev_ports::DevPorts),
        Box::new(sty_contrast::StyContrast),
        Box::new(txt_catalog::TxtCatalog),
    ]
}
