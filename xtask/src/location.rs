//! BUILD-LOCATION: nothing is built inside OneDrive; work happens in `the work root` (ADR-0024).

use std::env;
use std::path::{Component, Path, PathBuf};

use anyhow::bail;

/// Environment variables Windows sets to the user's OneDrive folders.
const ONEDRIVE_VARIABLES: [&str; 3] = ["OneDrive", "OneDriveConsumer", "OneDriveCommercial"];

/// Stops xtask before it builds, installs or tests anything inside a OneDrive folder.
pub(crate) fn guard(root: &Path) -> anyhow::Result<()> {
    if let Some(problem) = check(root, &onedrive_roots()) {
        bail!("{problem}");
    }
    Ok(())
}

/// The problem with building at `root`, if there is one.
pub(crate) fn check(root: &Path, onedrive_roots: &[PathBuf]) -> Option<String> {
    let inside = onedrive_roots
        .iter()
        .any(|onedrive| root.starts_with(onedrive))
        || root.components().any(|part| match part {
            Component::Normal(name) => name.to_string_lossy().starts_with("OneDrive"),
            _ => false,
        });
    inside.then(|| {
        format!(
            "This repository is inside OneDrive ({}). Nothing may be built or installed here; \
             work in a clone under C:\\Users\\<you>\\the work root instead (ADR-0024). \
             If a `target` folder was created here, delete it.",
            root.display()
        )
    })
}

pub(crate) fn onedrive_roots() -> Vec<PathBuf> {
    ONEDRIVE_VARIABLES
        .iter()
        .filter_map(env::var_os)
        .filter(|value| !value.is_empty())
        .map(PathBuf::from)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bad_example() {
        let roots = [PathBuf::from(r"C:\Users\ana\OneDrive")];
        assert!(check(Path::new(r"C:\Users\ana\OneDrive\the projects folder\bugaboo"), &roots).is_some());
        assert!(check(Path::new(r"D:\OneDrive - ACME\code\bugaboo"), &[]).is_some());
    }

    #[test]
    fn good_example() {
        let roots = [PathBuf::from(r"C:\Users\ana\OneDrive")];
        assert!(
            check(
                Path::new(r"C:\Users\ana\the work root\bugaboo.win1"),
                &roots
            )
            .is_none()
        );
        assert!(check(Path::new("/home/ana/src/bugaboo"), &[]).is_none());
    }
}
