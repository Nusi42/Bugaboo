//! The bodies, paths and queries of the routes of settings.

use super::{
    ApiError, Authorized, BTreeMap, ContentBuilder, DefaultValue, Deserialize, Format,
    IntoResponses, Level, Masked, PROBLEM_CONTENT_TYPE, ProjectLabel, Ref, RefOr, Response,
    ResponseBuilder, ResponseDescription, Serialize, SettingList, SettingSource, SettingValue,
    SettingView, ToSchema, answer_authorized, problem_answers,
};

use crate::api::projects::ProjectRef;

problem_answers! {
    /// A lower level cannot change what the organization locked.
    LockedAnswer => ("409", "The organization locked this setting (setting.locked)."),
}

/// What a setting holds: yes or no, a whole number, or text.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(untagged)]
pub(crate) enum ValueBody {
    /// Yes or no.
    Bool(bool),
    /// A whole number.
    Integer(i64),
    /// Text, or one choice from a list written as text.
    Text(String),
}

impl From<SettingValue> for ValueBody {
    fn from(value: SettingValue) -> Self {
        match value {
            SettingValue::Bool(value) => Self::Bool(value),
            SettingValue::Integer(value) => Self::Integer(value),
            SettingValue::Text(value) => Self::Text(value),
        }
    }
}

impl From<ValueBody> for SettingValue {
    fn from(value: ValueBody) -> Self {
        match value {
            ValueBody::Bool(value) => Self::Bool(value),
            ValueBody::Integer(value) => Self::Integer(value),
            ValueBody::Text(value) => Self::Text(value),
        }
    }
}

/// The type of a setting.
#[derive(Serialize, ToSchema)]
#[serde(rename_all = "kebab-case")]
pub(super) enum KindBody {
    /// Yes or no.
    Bool,
    /// A whole number.
    Integer,
    /// Text.
    Text,
}

/// What kind of text a text setting holds.
#[derive(Serialize, ToSchema)]
#[serde(rename_all = "kebab-case")]
pub(super) enum FormatBody {
    /// Any text.
    Free,
    /// One of a fixed list (`choices`).
    Choice,
    /// `device`, or a time zone like `Europe/Berlin`.
    TimeZone,
    /// The address of a page of the app.
    PageAddress,
    /// The keys of the keyboard shortcuts, as one JSON object.
    ShortcutKeys,
    /// The sizes of the windows of the forms, as one JSON object.
    WindowSizes,
    /// A list of host names separated by commas.
    Hosts,
}

/// Where a setting can be set.
#[derive(Serialize, ToSchema)]
#[serde(rename_all = "kebab-case")]
pub(super) enum LevelBody {
    /// The whole organization.
    Organization,
    /// One project and its sub-projects.
    Project,
    /// One person.
    Person,
}

impl From<Level> for LevelBody {
    fn from(level: Level) -> Self {
        match level {
            Level::Organization => Self::Organization,
            Level::Project => Self::Project,
            Level::Person => Self::Person,
        }
    }
}

/// One value of a setting that has a fixed list.
#[derive(Serialize, ToSchema)]
pub(super) struct ChoiceBody {
    /// The text that is stored and sent.
    pub(super) value: String,
    /// The key of the message that names the choice.
    pub(super) label_key: String,
}

/// The smallest and the largest value of a whole-number setting.
#[derive(Serialize, ToSchema)]
pub(super) struct BoundsBody {
    /// The smallest value.
    pub(super) least: i64,
    /// The largest value.
    pub(super) most: i64,
}

/// The size of a list a text setting holds (items separated by commas).
#[derive(Serialize, ToSchema)]
pub(super) struct ListLimitsBody {
    /// The most items.
    pub(super) most_items: u32,
    /// The most characters of one item.
    pub(super) longest_item: u32,
}

/// Where the value that counts comes from.
#[derive(Serialize, ToSchema)]
#[serde(rename_all = "kebab-case")]
pub(super) enum SourceKindBody {
    /// Nobody set it: the standard value.
    Default,
    /// The organization set it.
    Organization,
    /// A project set it.
    Project,
    /// The person set it themselves.
    Person,
}

/// Where the value that counts comes from.
#[derive(Serialize, ToSchema)]
pub(super) struct SourceBody {
    /// Which level.
    pub(super) kind: SourceKindBody,
    /// For a value from a project: which one. Missing when the person cannot see the project.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(super) project: Option<ProjectRef>,
}

impl From<&SettingSource> for SourceBody {
    fn from(source: &SettingSource) -> Self {
        let (kind, project) = match source {
            SettingSource::Default => (SourceKindBody::Default, None),
            SettingSource::Organization => (SourceKindBody::Organization, None),
            SettingSource::Person => (SourceKindBody::Person, None),
            SettingSource::Project(ProjectLabel::Visible {
                id,
                short_name,
                name,
            }) => (
                SourceKindBody::Project,
                Some(ProjectRef {
                    id: id.to_string(),
                    short_name: short_name.clone(),
                    name: name.clone(),
                }),
            ),
            SettingSource::Project(ProjectLabel::Hidden) => (SourceKindBody::Project, None),
        };
        Self { kind, project }
    }
}

/// One setting: what the registry knows about it, and the value that counts.
#[derive(Serialize, ToSchema)]
pub(crate) struct SettingBody {
    /// The key: the topic, a dot and the name, like `topic.name`.
    pub(super) key: String,
    /// The key of the message that names the setting.
    pub(super) name_key: String,
    /// The key of the message that explains it in one line.
    pub(super) help_key: String,
    /// What it holds.
    pub(super) kind: KindBody,
    /// For text: what kind of text.
    pub(super) format: FormatBody,
    /// For a setting with a fixed list: the values it can have.
    pub(super) choices: Vec<ChoiceBody>,
    /// For a whole number: the smallest and the largest value.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(super) bounds: Option<BoundsBody>,
    /// For a list in a text: its size.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(super) list: Option<ListLimitsBody>,
    /// Where it can be set.
    pub(super) levels: Vec<LevelBody>,
    /// Whether administrators can lock it.
    pub(super) lockable: bool,
    /// The standard value.
    pub(super) default: ValueBody,
    /// The value that counts here.
    pub(super) value: ValueBody,
    /// Where it comes from.
    pub(super) source: SourceBody,
    /// Whether this level has a value of its own, which *Reset to default* takes away.
    pub(super) own: bool,
    /// On the organization's level: whether the value is locked. Below it: whether the
    /// organization's lock is in effect, so the value cannot be changed.
    pub(super) locked: bool,
}

impl Masked for SettingBody {}

impl From<&SettingView> for SettingBody {
    fn from(view: &SettingView) -> Self {
        let definition = view.definition;
        let format = definition.format();
        Self {
            key: definition.key().to_owned(),
            name_key: definition.name().to_string(),
            help_key: definition.help().to_string(),
            kind: match definition.default() {
                DefaultValue::Bool(_) => KindBody::Bool,
                DefaultValue::Integer(_) => KindBody::Integer,
                DefaultValue::Text(_) => KindBody::Text,
            },
            format: match format {
                Format::Free => FormatBody::Free,
                Format::Choices(_) => FormatBody::Choice,
                Format::TimeZone => FormatBody::TimeZone,
                Format::PageAddress => FormatBody::PageAddress,
                Format::ShortcutKeys => FormatBody::ShortcutKeys,
                Format::WindowSizes => FormatBody::WindowSizes,
                Format::Hosts => FormatBody::Hosts,
            },
            choices: format
                .choices()
                .iter()
                .map(|choice| ChoiceBody {
                    value: choice.value().to_owned(),
                    label_key: choice.label().to_string(),
                })
                .collect(),
            bounds: definition
                .bounds()
                .map(|(least, most)| BoundsBody { least, most }),
            list: definition
                .list_limits()
                .map(|(most_items, longest_item)| ListLimitsBody {
                    most_items: u32::try_from(most_items).unwrap_or(u32::MAX),
                    longest_item: u32::try_from(longest_item).unwrap_or(u32::MAX),
                }),
            levels: definition
                .levels()
                .iter()
                .map(|level| LevelBody::from(*level))
                .collect(),
            lockable: definition.lockable(),
            default: definition.default().to_value().into(),
            value: view.value.clone().into(),
            source: SourceBody::from(&view.source),
            own: view.own,
            locked: view.locked,
        }
    }
}

/// The settings of a topic, in the order of the registry.
#[derive(Serialize, ToSchema)]
pub(crate) struct SettingsBody {
    /// The settings.
    pub(super) settings: Vec<SettingBody>,
}

impl Masked for SettingsBody {}

pub(super) fn list_body(list: Authorized<SettingList>) -> Result<Response, ApiError> {
    answer_authorized(&list.map(|list| SettingsBody {
        settings: list.settings.iter().map(SettingBody::from).collect(),
    }))
}

pub(super) fn setting_body(view: Authorized<SettingView>) -> Result<Response, ApiError> {
    answer_authorized(&view.map(|view| SettingBody::from(&view)))
}

/// A new value for a setting.
#[derive(Deserialize, ToSchema)]
pub(crate) struct NewValueBody {
    /// The value. It must be of the type of the setting and one it accepts.
    pub(super) value: ValueBody,
}

/// What a route of a list reads from its address.
pub(super) mod query {
    use serde::Deserialize;
    use utoipa::IntoParams;

    /// Which settings to list.
    #[derive(Deserialize, IntoParams)]
    #[into_params(parameter_in = Query)]
    pub(in super::super) struct TopicParams {
        /// Only the settings whose key starts with one of these texts, separated by commas:
        /// `security.` for a topic, or longer (`auth.access-key-`) for part of one, so
        /// `security.,auth.access-key-` is the settings of one page. Left out, every setting of
        /// the level.
        pub(in super::super) topic: Option<String>,
    }

    /// Which of a person's own settings to list, and where they work.
    #[derive(Deserialize, IntoParams)]
    #[into_params(parameter_in = Query)]
    pub(in super::super) struct MineParams {
        /// Only the settings whose key starts with one of these texts, separated by commas, like
        /// `region.` for a topic.
        pub(in super::super) topic: Option<String>,
        /// The ID or the short name of the project the person works in: the values of the project (and of the
        /// projects above it) then count for them.
        pub(in super::super) project: Option<String>,
    }
}

/// The path of a setting.
#[derive(Deserialize)]
pub(crate) struct KeyPath {
    pub(super) key: String,
}

/// The path of a setting of a project.
#[derive(Deserialize)]
pub(crate) struct ProjectKeyPath {
    pub(super) project: String,
    pub(super) key: String,
}

/// The path of the settings of a project.
#[derive(Deserialize)]
pub(crate) struct ProjectPath {
    pub(super) project: String,
}
