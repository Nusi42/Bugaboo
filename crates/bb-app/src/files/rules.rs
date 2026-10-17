//! The limits that count for a file in a project (ATT-03, M4-Q4): how big it may be and which
//! types are never accepted. Read from the settings once for a request.

use bb_core::files::{MEGABYTE, name_extension};
use bb_core::id::ProjectId;
use bb_core::settings::{
    FILES_BLOCKED_TYPES, FILES_MAX_MEGABYTES, Layers, SettingValue, list_items,
};

use super::file_store::Sniffed;
use crate::auth::{AppError, AuthStore, Ctx, internal};

/// The limits of one project.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct FileLimits {
    /// The most bytes one file may have.
    pub(crate) max_bytes: u64,
    /// The extensions the organization never accepts, in lower case and without a dot.
    pub(crate) blocked: Vec<String>,
}

/// An extension as it is compared: lower case, without a dot in front.
fn normalized(item: &str) -> String {
    item.trim_start_matches('.').to_lowercase()
}

/// The whole megabytes a stored value of the setting says (the default when there is none or it
/// has the wrong type), cut to the bounds of the setting.
fn megabytes_of(value: Option<&SettingValue>) -> i64 {
    FILES_MAX_MEGABYTES
        .resolve(value)
        .or_else(|_| FILES_MAX_MEGABYTES.default_value())
        .map(|megabytes| FILES_MAX_MEGABYTES.definition().clamp(megabytes))
        .unwrap_or_default()
}

impl FileLimits {
    /// The limits that follow from the layers of a project. A project may set less than the
    /// organization and never more, so the smaller of the two counts.
    pub(crate) fn of(layers: &Layers) -> Self {
        let definition = FILES_MAX_MEGABYTES.definition();
        let nearest = layers.resolve(definition).value;
        let nearest = megabytes_of(Some(&nearest));
        let organization = megabytes_of(
            layers
                .organization
                .get(definition.key())
                .map(|set| &set.value),
        );
        let blocked = FILES_BLOCKED_TYPES
            .resolve(Some(
                &layers.resolve(FILES_BLOCKED_TYPES.definition()).value,
            ))
            .unwrap_or_default();
        Self {
            max_bytes: u64::try_from(nearest.min(organization)).unwrap_or(0) * MEGABYTE,
            blocked: list_items(&blocked).map(normalized).collect(),
        }
    }

    /// The limit in whole megabytes, as the screens say it.
    pub(crate) const fn megabytes(&self) -> u64 {
        self.max_bytes / MEGABYTE
    }

    /// Whether a file is refused for its type: the extension of its name is on the list, or its
    /// content says it is a type whose extension is on the list (a renamed file does not slip
    /// through). `sniffed` is none while the content is not known yet.
    pub(crate) fn blocks(&self, name: &str, sniffed: Option<&Sniffed>) -> bool {
        let by_name = name_extension(name);
        let by_content = sniffed.and_then(|sniffed| sniffed.extension.as_deref());
        by_name
            .iter()
            .map(String::as_str)
            .chain(by_content)
            .any(|extension| self.blocked.iter().any(|blocked| blocked == extension))
    }
}

impl<S: AuthStore> Ctx<'_, S> {
    /// The limits of a project, as the settings have them now.
    pub(crate) async fn file_limits(&self, project: ProjectId) -> Result<FileLimits, AppError> {
        let layers = self
            .store()
            .setting_layers(Some(project), None)
            .await
            .map_err(|error| internal::<S>(&error))?;
        Ok(FileLimits::of(&layers))
    }
}

#[cfg(test)]
mod tests {
    use bb_core::files::MediaType;
    use bb_core::id::SequentialIds;
    use bb_core::settings::OrganizationValue;

    use super::*;

    fn exe() -> Sniffed {
        Sniffed {
            media_type: MediaType::new("application/vnd.microsoft.portable-executable").unwrap(),
            extension: Some("exe".to_owned()),
        }
    }

    fn layers(blocked: &str, organization: i64, project: Option<i64>) -> Layers {
        let mut layers = Layers::default();
        layers.organization.insert(
            FILES_BLOCKED_TYPES.definition().key().to_owned(),
            OrganizationValue {
                value: SettingValue::Text(blocked.to_owned()),
                locked: false,
            },
        );
        layers.organization.insert(
            FILES_MAX_MEGABYTES.definition().key().to_owned(),
            OrganizationValue {
                value: SettingValue::Integer(organization),
                locked: false,
            },
        );
        if let Some(project) = project {
            layers.projects.push((
                ProjectId::generate(&mut SequentialIds::default()).unwrap(),
                [(
                    FILES_MAX_MEGABYTES.definition().key().to_owned(),
                    SettingValue::Integer(project),
                )]
                .into(),
            ));
        }
        layers
    }

    #[test]
    fn att_03_the_default_is_25_megabytes() {
        let limits = FileLimits::of(&Layers::default());
        assert_eq!(limits.megabytes(), 25);
        assert!(limits.blocked.is_empty());
    }

    #[test]
    fn att_03_a_project_may_set_less_than_the_organization_and_never_more() {
        assert_eq!(FileLimits::of(&layers("", 100, Some(10))).megabytes(), 10);
        assert_eq!(FileLimits::of(&layers("", 30, Some(80))).megabytes(), 30);
        assert_eq!(FileLimits::of(&layers("", 100, None)).megabytes(), 100);
    }

    #[test]
    fn att_03_a_type_is_blocked_by_the_name_and_by_the_content() {
        let limits = FileLimits::of(&layers(" .EXE, bat ", 25, None));
        assert!(limits.blocks("setup.exe", None));
        assert!(limits.blocks("RUN.BAT", None));
        assert!(!limits.blocks("notes.txt", None));
        // A renamed file does not slip through.
        assert!(limits.blocks("notes.txt", Some(&exe())));
        assert!(!FileLimits::of(&layers("", 25, None)).blocks("setup.exe", Some(&exe())));
    }
}
