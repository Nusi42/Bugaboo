//! What can go wrong while the sample is made. Every error says in plain words what it was doing;
//! the words are in `messages/en.json` like all text.

use std::fmt;
use std::path::PathBuf;

use bb_core::message::{self, Message, MessageKey};
use bb_dev_words::sample::LayoutError;

use crate::people_plan::PEOPLE;

/// What the sample was doing when a use case refused.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Step {
    /// Making the password hasher.
    PasswordHasher,
    /// The first start: the starting roles, issue types and kinds of relation, and the setup code.
    FirstStart,
    /// Making the first administrator.
    FirstAdministrator,
    /// Inviting a person.
    Invitation,
    /// Accepting an invitation.
    AcceptingInvitation,
    /// Signing out.
    SigningOut,
    /// Setting up a code app.
    CodeApp,
    /// Working out the dates.
    Dates,
    /// Choosing the time zone and the formats of a person.
    Region,
    /// Making an access key.
    AccessKey,
    /// Making a group.
    Group,
    /// Putting a person into a group.
    GroupMember,
    /// Making a role.
    Role,
    /// Setting what a role sees.
    Levels,
    /// Making a project.
    Project,
    /// Giving a role or *No access*.
    Assignment,
    /// Making the starting issue types and kinds of relation.
    IssueTypes,
    /// Choosing how a project gives out issue IDs.
    Settings,
    /// Making an own field.
    Fields,
    /// Making a group of labels or a label.
    Labels,
    /// Signing in to do the work.
    SigningIn,
    /// Making an issue.
    Issue,
    /// Changing an issue.
    Change,
    /// Putting a tag or a label on an issue.
    Tags,
    /// Making a relation or putting an issue under a goal.
    Relation,
    /// Writing a comment.
    Comment,
    /// Editing a text under a lock, or keeping a draft.
    Editing,
    /// Reading the history of the issues from the events.
    History,
}

impl Step {
    fn message(self) -> Message {
        Message::from(match self {
            Self::PasswordHasher => message::SAMPLE_STEP_PASSWORD_HASHER,
            Self::FirstStart => message::SAMPLE_STEP_FIRST_START,
            Self::FirstAdministrator => message::SAMPLE_STEP_FIRST_ADMINISTRATOR,
            Self::Invitation => message::SAMPLE_STEP_INVITATION,
            Self::AcceptingInvitation => message::SAMPLE_STEP_ACCEPTING_INVITATION,
            Self::SigningOut => message::SAMPLE_STEP_SIGNING_OUT,
            Self::CodeApp => message::SAMPLE_STEP_CODE_APP,
            Self::Dates => message::SAMPLE_STEP_DATES,
            Self::Region => message::SAMPLE_STEP_REGION,
            Self::AccessKey => message::SAMPLE_STEP_ACCESS_KEY,
            Self::Group => message::SAMPLE_STEP_GROUP,
            Self::GroupMember => message::SAMPLE_STEP_GROUP_MEMBER,
            Self::Role => message::SAMPLE_STEP_ROLE,
            Self::Levels => message::SAMPLE_STEP_LEVELS,
            Self::Project => message::SAMPLE_STEP_PROJECT,
            Self::Assignment => message::SAMPLE_STEP_ASSIGNMENT,
            Self::IssueTypes => message::SAMPLE_STEP_ISSUE_TYPES,
            Self::Settings => message::SAMPLE_STEP_SETTINGS,
            Self::Fields => message::SAMPLE_STEP_FIELDS,
            Self::Labels => message::SAMPLE_STEP_LABELS,
            Self::SigningIn => message::SAMPLE_STEP_SIGNING_IN,
            Self::Issue => message::SAMPLE_STEP_ISSUE,
            Self::Change => message::SAMPLE_STEP_CHANGE,
            Self::Tags => message::SAMPLE_STEP_TAGS,
            Self::Relation => message::SAMPLE_STEP_RELATION,
            Self::Comment => message::SAMPLE_STEP_COMMENT,
            Self::Editing => message::SAMPLE_STEP_EDITING,
            Self::History => message::SAMPLE_STEP_HISTORY,
        })
    }
}

/// Why the sample organization could not be made.
#[derive(Debug)]
pub enum SampleError {
    /// The list of names is not a list of names (what the TOML reader said).
    Names(String),
    /// The list of names is empty or names somebody twice.
    NamesRepeated,
    /// A name has other characters than letters.
    NameNotLetters(String),
    /// The list of names has too few names for the people of the sample.
    TooFewNames,
    /// The system gave no secure random numbers (for the passwords).
    Random,
    /// A folder or a file could not be written or read.
    Folder {
        /// The folder or file.
        path: PathBuf,
        /// What the system said.
        reason: String,
    },
    /// The folder has other things in it and was not made by this tool, so it is left alone.
    NotASampleFolder(PathBuf),
    /// The database or its key files failed.
    Store(String),
    /// A use case refused a step the sample took, like it would refuse a person.
    Step {
        /// What the sample was doing.
        step: Step,
        /// What the use case said.
        reason: String,
    },
    /// The setup code does not work: setup was closed already.
    SetupClosed,
    /// The secret of a code app that was just set up is not in the store.
    SecretNotStored,
    /// A person of the plan has no account after the plan was carried out.
    PersonMissing,
    /// The answer to an invitation has no link in it.
    LinkMissing,
    /// The answer to a new access key has no key in it.
    KeyMissing,
    /// A group, role or project that was just made is not in the database (its name).
    NotMade(String),
    /// Nothing vouches for the end of the security log.
    LogNotAnchored,
    /// The security log of the finished sample does not verify.
    LogBroken {
        /// The place of the first entry that does not verify.
        seq: i64,
        /// What is wrong with it.
        reason: String,
    },
    /// The start moment is not a moment in time.
    Start(i64),
    /// The address of the PostgreSQL database is not in the environment (the variable it is
    /// looked for in).
    DatabaseUrl(String),
    /// `tools/dev/sample.json` is not usable.
    Layout(String),
}

impl From<LayoutError> for SampleError {
    fn from(error: LayoutError) -> Self {
        Self::Layout(error.to_string())
    }
}

impl SampleError {
    /// The error as a message a person can read.
    #[must_use]
    pub fn message(&self) -> Message {
        let plain = |key: MessageKey| Message::from(key);
        match self {
            Self::Names(reason) => message::sample_error_names(reason),
            Self::NamesRepeated => plain(message::SAMPLE_ERROR_NAMES_REPEATED),
            Self::NameNotLetters(name) => message::sample_error_name_not_letters(name),
            Self::TooFewNames => message::sample_error_too_few_names(PEOPLE),
            Self::Random => plain(message::SAMPLE_ERROR_RANDOM),
            Self::Folder { path, reason } => message::sample_error_folder(path.display(), reason),
            Self::NotASampleFolder(path) => {
                message::sample_error_not_a_sample_folder(path.display())
            }
            Self::Store(reason) => message::sample_error_store(reason),
            Self::Step { step, reason } => {
                message::sample_error_step(reason, step.message().render())
            }
            Self::SetupClosed => plain(message::SAMPLE_ERROR_SETUP_CLOSED),
            Self::SecretNotStored => plain(message::SAMPLE_ERROR_SECRET_NOT_STORED),
            Self::PersonMissing => plain(message::SAMPLE_ERROR_PERSON_MISSING),
            Self::LinkMissing => plain(message::SAMPLE_ERROR_LINK_MISSING),
            Self::KeyMissing => plain(message::SAMPLE_ERROR_KEY_MISSING),
            Self::NotMade(name) => message::sample_error_not_made(name),
            Self::LogNotAnchored => plain(message::SAMPLE_ERROR_LOG_NOT_ANCHORED),
            Self::LogBroken { seq, reason } => message::sample_error_log_broken(reason, seq),
            Self::Start(start) => message::sample_error_start(start),
            Self::DatabaseUrl(variable) => message::sample_error_database_url(variable),
            Self::Layout(reason) => message::sample_error_layout(reason),
        }
    }

    /// A folder error from the error of the file system.
    pub(crate) fn folder(path: impl Into<PathBuf>, error: &std::io::Error) -> Self {
        Self::Folder {
            path: path.into(),
            reason: error.to_string(),
        }
    }

    /// A step the use cases refused.
    pub(crate) fn step(step: Step, error: &impl fmt::Display) -> Self {
        Self::Step {
            step,
            reason: error.to_string(),
        }
    }
}

impl fmt::Display for SampleError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message().render())
    }
}

impl std::error::Error for SampleError {}

impl From<bb_store::StoreError> for SampleError {
    fn from(error: bb_store::StoreError) -> Self {
        Self::Store(error.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_error_says_something_in_words_with_nothing_left_to_fill_in() {
        let errors = [
            SampleError::Names("x".to_owned()),
            SampleError::NamesRepeated,
            SampleError::NameNotLetters("Zo\u{eb}".to_owned()),
            SampleError::TooFewNames,
            SampleError::Random,
            SampleError::Folder {
                path: PathBuf::from("a"),
                reason: "b".to_owned(),
            },
            SampleError::NotASampleFolder(PathBuf::from("a")),
            SampleError::Store("x".to_owned()),
            SampleError::step(Step::CodeApp, &"refused"),
            SampleError::SetupClosed,
            SampleError::SecretNotStored,
            SampleError::PersonMissing,
            SampleError::LinkMissing,
            SampleError::KeyMissing,
            SampleError::NotMade("Backend".to_owned()),
            SampleError::LogNotAnchored,
            SampleError::LogBroken {
                seq: 3,
                reason: "Gap".to_owned(),
            },
            SampleError::Start(-1),
            SampleError::DatabaseUrl("A_VARIABLE_NAME".to_owned()),
            SampleError::Layout("a reason".to_owned()),
        ];
        for error in errors {
            let text = error.to_string();
            assert!(text.len() > 20, "{text}");
            assert!(!text.contains('{'), "a placeholder is left in: {text}");
        }
        assert!(SampleError::TooFewNames.to_string().contains("50"));
    }
}
