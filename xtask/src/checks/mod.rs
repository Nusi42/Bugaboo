//! Every implemented check. Each one lives in its own file and is registered in [`registry`].
//!
//! To add a check (ADR-0020): write the rule in the docs, add the catalogue entry, implement
//! [`Check`] here, and add a failing and a passing example (`bad_example` / `good_example` tests
//! or `tools/checks/fixtures/<ID>/`).

mod arch_deps;
mod arch_errors;
mod arch_map_sync;
mod arch_no_unsafe;
mod arch_settings;
mod arch_wasm;
mod build_location;
mod data_marks;
mod dep_licenses;
mod dep_register;
mod dev_ports;
mod doc_adr_enforced;
mod doc_generated;
mod fmt_css;
mod fmt_md;
mod fmt_rust;
mod fmt_toml;
mod fmt_ts;
pub(crate) mod git_commit_msg;
mod git_files;
mod lint_ci;
mod lint_css;
mod lint_html;
mod lint_links;
mod lint_rust;
mod lint_spell;
mod lint_ts;
mod lint_types;
mod meta_catalog;
mod meta_coverage;
mod meta_fixtures;
mod meta_new_module;
mod meta_rule_trace;
mod meta_speed;
pub(crate) mod meta_suppress;
mod meta_wiring;
mod node_tool;
mod plan_trace;
mod sec_secrets;
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

impl Context<'_> {
    /// The existing files among the triggering ones that this check cares about, or `None` when
    /// the whole repository is checked.
    pub(crate) fn files_for(&self, id: &str) -> anyhow::Result<Option<Vec<String>>> {
        let (Some(files), Some(entry)) = (self.files, self.catalog.get(id)) else {
            return Ok(None);
        };
        let mut wanted = entry.matching(files)?;
        wanted.retain(|file| self.repo.path(file).is_file());
        Ok(Some(wanted))
    }
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
        Box::new(fmt_rust::FmtRust),
        Box::new(fmt_toml::FmtToml),
        Box::new(fmt_md::FmtMd),
        Box::new(lint_rust::LintRust),
        Box::new(lint_spell::LintSpell),
        Box::new(lint_links::LintLinks),
        Box::new(lint_ci::LintCi),
        Box::new(arch_deps::ArchDeps),
        Box::new(arch_no_unsafe::ArchNoUnsafe),
        Box::new(arch_errors::ArchErrors),
        Box::new(arch_settings::ArchSettings),
        Box::new(arch_wasm::ArchWasm),
        Box::new(data_marks::DataMarks),
        Box::new(sec_secrets::SecSecrets),
        Box::new(dep_register::DepRegister),
        Box::new(dep_licenses::DepLicenses),
        Box::new(git_files::GitFiles),
        Box::new(git_commit_msg::GitCommitMsg),
        Box::new(fmt_ts::FmtTs),
        Box::new(fmt_css::FmtCss),
        Box::new(lint_ts::LintTs),
        Box::new(lint_types::LintTypes),
        Box::new(lint_css::LintCss),
        Box::new(lint_html::LintHtml),
        Box::new(dev_ports::DevPorts),
        Box::new(sty_contrast::StyContrast),
        Box::new(txt_catalog::TxtCatalog),
        Box::new(doc_generated::DocGenerated),
        Box::new(doc_adr_enforced::DocAdrEnforced),
        Box::new(plan_trace::PlanTrace),
        Box::new(arch_map_sync::ArchMapSync),
        Box::new(meta_fixtures::MetaFixtures),
        Box::new(meta_coverage::MetaCoverage),
        Box::new(meta_new_module::MetaNewModule),
        Box::new(meta_rule_trace::MetaRuleTrace),
        Box::new(meta_wiring::MetaWiring),
        Box::new(meta_suppress::MetaSuppress),
        Box::new(meta_speed::MetaSpeed),
    ]
}
