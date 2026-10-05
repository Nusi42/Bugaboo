//! What the files and the environment variable of the sample are called: read from
//! `tools/dev/sample.json`, which `xtask` and the end-to-end tests read too, so there is one
//! place that says it. The program reads the text that was built into it.

use serde::Deserialize;

use crate::error::SampleError;

/// The text of the file that is part of this program.
const BUNDLED: &str = include_str!("../../../tools/dev/sample.json");

/// The names the sample files go by (the folders `xtask` makes it in are `xtask`'s business and
/// are in the file too).
#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Layout {
    /// The data folder inside the output folder.
    pub data_folder: String,
    /// The file with the passwords, inside the output folder: the only one that holds a secret.
    pub sign_in_file: String,
    /// The canonical dump, inside the output folder.
    pub dump_file: String,
    /// The file that marks a folder as made by this tool.
    pub marker_file: String,
    /// The environment variable that holds the address of the empty PostgreSQL database of the
    /// program (an address with a password does not belong on a command line, M1-F62).
    pub database_url_variable: String,
}

impl Layout {
    /// The layout that is part of this program.
    ///
    /// # Errors
    /// [`SampleError::Layout`] when the file is not usable (a test keeps it usable).
    pub fn bundled() -> Result<Self, SampleError> {
        Self::parse(BUNDLED)
    }

    /// A layout from the text of a `sample.json` file.
    ///
    /// # Errors
    /// [`SampleError::Layout`] when the text lacks a name or has an empty one.
    pub fn parse(text: &str) -> Result<Self, SampleError> {
        let layout: Self =
            serde_json::from_str(text).map_err(|error| SampleError::Layout(error.to_string()))?;
        let names = [
            &layout.data_folder,
            &layout.sign_in_file,
            &layout.dump_file,
            &layout.marker_file,
            &layout.database_url_variable,
        ];
        if names.iter().any(|name| name.trim().is_empty()) {
            return Err(SampleError::Layout("a name is empty".to_owned()));
        }
        Ok(layout)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_bundled_layout_is_usable() {
        let layout = Layout::bundled().unwrap();
        assert!(!layout.sign_in_file.is_empty());
        assert!(layout.database_url_variable.starts_with("BB_SAMPLE_"));
    }

    #[test]
    fn a_layout_with_a_missing_or_empty_name_is_refused() {
        assert!(matches!(Layout::parse("{}"), Err(SampleError::Layout(_))));
        let layout = Layout::bundled().unwrap();
        let empty = BUNDLED.replace(&format!("\"{}\"", layout.sign_in_file), "\" \"");
        assert!(matches!(Layout::parse(&empty), Err(SampleError::Layout(_))));
    }
}
