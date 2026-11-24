//! `cargo xtask release repack --from <folder>` (ADR-0047, as amended): the release folder of this
//! version made from the files of a release published with the earlier layout, which had a
//! signature for each file, the parts list as SPDX too, the VEX statements in a file of their own
//! and the notes as a file.
//!
//! The programs and the notices are taken as they are, the statements of the old VEX file are put
//! into the `CycloneDX` parts list with [`vex::with_statements`], as `cargo xtask release` puts
//! them, and nothing else is taken. `cargo xtask release sign` then writes `SHA256SUMS` and its
//! signature, and `cargo xtask release check` checks the folder.

use std::fs;
use std::path::Path;

use anyhow::{Context as _, bail};
use bb_api_words::about;
use serde_json::Value;

use super::{folder, formats, vex};

/// Makes the folder `to` of the release named `release` from the published files in `from`. The
/// folder `to` is made anew; `from` is only read.
pub(crate) fn repack(from: &Path, to: &Path, release: &str) -> anyhow::Result<()> {
    let parts_list = formats::cyclonedx_name(release);
    let old_vex = about::release_name(release, about::VEX_END);
    let read = |name: &str| -> anyhow::Result<Value> {
        let path = from.join(name);
        let text = fs::read_to_string(&path).with_context(|| {
            format!(
                "could not read {}: `--from` names the folder of the published files of the \
                 release {release}",
                path.display()
            )
        })?;
        serde_json::from_str(&text).with_context(|| format!("{} is not JSON", path.display()))
    };
    let statements = read(&old_vex)?
        .get("vulnerabilities")
        .and_then(Value::as_array)
        .cloned()
        .with_context(|| format!("{old_vex} holds no `vulnerabilities`"))?;
    let document = vex::with_statements(read(&parts_list)?, statements);
    let errors = formats::cyclonedx_errors(&document)?;
    if !errors.is_empty() {
        bail!(
            "{parts_list} with the VEX statements does not fit its schema:\n{}",
            errors.join("\n")
        );
    }
    if let Some(missing) = folder::payload(release)
        .into_iter()
        .find(|name| !from.join(name).is_file())
    {
        bail!("{} holds no {missing}", from.display());
    }
    if to.exists() {
        if fs::canonicalize(from).ok() == fs::canonicalize(to).ok() {
            bail!(
                "{} is the release folder itself: name the folder of the published files",
                from.display()
            );
        }
        fs::remove_dir_all(to).with_context(|| format!("could not clear {}", to.display()))?;
    }
    fs::create_dir_all(to).with_context(|| format!("could not create {}", to.display()))?;
    for name in folder::payload(release) {
        if name != parts_list {
            fs::copy(from.join(&name), to.join(&name))
                .with_context(|| format!("could not copy {name} from {}", from.display()))?;
        }
    }
    super::write_file(to, &parts_list, &super::to_text(&document))
}

#[cfg(test)]
mod tests;
