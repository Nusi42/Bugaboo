//! The structured answer to a stale version (`edit.version-changed`, ADR-0019 amendment M3) as the
//! API writes it: the usual problem answer plus a `conflict` member with who changed the record,
//! when, which fields, and the values now.
//!
//! It is made in one place for every route: a use case answers that the version is stale, `AuthApi`
//! turns that into a [`bb_app::editing::Conflict`], and `From<AppError>` for [`crate::ApiError`]
//! puts it here. The API description documents it for every route that takes a `version`
//! (`crate::openapi`), so the generated client types it for all of them.

use std::collections::BTreeMap;

use bb_app::editing::Conflict;
use bb_app::themes::NamedPair;
use bb_core::Problem;
use bb_core::theme::{ValueProblem, ValueProblemKind};
use bb_policy::PersonRef;
use serde::Serialize;
use utoipa::ToSchema;

/// What changed a record since the person looked at it.
#[derive(Clone, Debug, Serialize, ToSchema)]
pub(crate) struct ConflictBody {
    /// The version the record has now: what *Use mine* sends again.
    version: Option<i64>,
    /// Who changed it last, as the reader may see them.
    changed_by: Option<PersonRef>,
    /// When, as a moment in time.
    changed_at: Option<String>,
    /// The fields that changed, without the ones the reader may not see.
    fields: Vec<String>,
    /// The values those fields have now.
    #[schema(value_type = Object)]
    current: BTreeMap<String, serde_json::Value>,
}

impl From<&Conflict> for ConflictBody {
    fn from(conflict: &Conflict) -> Self {
        Self {
            version: conflict.version,
            changed_by: conflict.changed_by.clone(),
            changed_at: conflict.changed_at.map(|moment| moment.to_string()),
            fields: conflict.fields.clone(),
            current: conflict.current.clone(),
        }
    }
}

/// The answer to a change that found the record at another version, or to any other refused
/// change: the usual problem, and for `edit.version-changed` the `conflict` member.
#[derive(Clone, Debug, Serialize, ToSchema)]
pub(crate) struct ConflictProblem {
    #[serde(flatten)]
    problem: Problem,
    /// What changed the record; only for `edit.version-changed`.
    #[serde(skip_serializing_if = "Option::is_none")]
    conflict: Option<ConflictBody>,
    /// Who took the editing over, as the person who saved may see them; only for
    /// `edit.taken-over`, so the browser needs no further request to name them (M3-F176).
    #[serde(skip_serializing_if = "Option::is_none")]
    taken_over_by: Option<PersonRef>,
    /// The mistakes of the text of a search, each with the key of its message and the bytes it
    /// stands on; only for `search.invalid`.
    #[serde(skip_serializing_if = "Option::is_none")]
    mistakes: Option<Vec<MistakeBody>>,
    /// The date of the person's own work day that was never ended (`2026-10-05`); only for
    /// `working-hours.day-left-open`, so the screen can ask when it ended.
    #[serde(skip_serializing_if = "Option::is_none")]
    open_day: Option<String>,
    /// One loop of dependencies, as the issue IDs on it in order; an issue the person may not see
    /// is `null`, so the loop never names it. Only for `relation.dependency-loop`.
    #[serde(skip_serializing_if = "Option::is_none")]
    dependency_loop: Option<Vec<Option<String>>>,
    /// The forward names of the kinds of relation that have a date effect; only for
    /// `relation-kind.date-effect-most`.
    #[serde(skip_serializing_if = "Option::is_none")]
    date_effect_kinds: Option<Vec<String>>,
    /// The values of an own theme that may not be set, each by its token; only for
    /// `theme.value-refused`.
    #[serde(skip_serializing_if = "Option::is_none")]
    refused_values: Option<Vec<RefusedValueBody>>,
    /// The pairs of colors that are *Hard to read*; only for `theme.hard-to-read`.
    #[serde(skip_serializing_if = "Option::is_none")]
    hard_to_read: Option<Vec<HardPairBody>>,
}

/// A value of an own theme that is refused, by the token (or the part of the file) it is stored
/// under.
#[derive(Clone, Debug, Serialize, ToSchema)]
pub(crate) struct RefusedValueBody {
    /// The name of the value: of the token it sets, or of the part of the file that is not one an
    /// own theme has.
    name: String,
    /// Why it is refused.
    reason: ValueProblemKind,
}

impl From<&ValueProblem> for RefusedValueBody {
    fn from(problem: &ValueProblem) -> Self {
        Self {
            name: problem.token.clone(),
            reason: problem.kind,
        }
    }
}

/// A pair of colors of a theme that is *Hard to read*, by its tokens: the refusal of a theme for
/// everyone, and the warning of a personal one.
#[derive(Clone, Debug, Serialize, ToSchema)]
pub(crate) struct HardPairBody {
    /// The first color: the text, the line or the state.
    first: String,
    /// The second color: the background, the surface or the other state.
    second: String,
    /// The color to change to mend it.
    place: String,
    /// Whether the pair is read with the colors for more contrast.
    more_contrast: bool,
}

impl From<&NamedPair> for HardPairBody {
    fn from(pair: &NamedPair) -> Self {
        Self {
            first: pair.first.clone(),
            second: pair.second.clone(),
            place: pair.place.clone(),
            more_contrast: pair.more_contrast,
        }
    }
}

/// One mistake in the text of a search (ADR-0005): what the browser needs to write the message in
/// the language of the person and to mark the word.
#[derive(Clone, Debug, Serialize, ToSchema)]
pub(crate) struct MistakeBody {
    /// The key of the message in the catalog (`search.mistake.*`).
    key: String,
    /// The words for the placeholders of the message, by name.
    args: BTreeMap<String, String>,
    /// The first byte of the text it is about.
    start: usize,
    /// The byte after the last one.
    end: usize,
    /// What may have been meant, closest first.
    suggestions: Vec<String>,
}

impl From<&bb_app::search::Mistake> for MistakeBody {
    fn from(mistake: &bb_app::search::Mistake) -> Self {
        Self {
            key: mistake.key.clone(),
            args: mistake.args.clone(),
            start: mistake.span.start,
            end: mistake.span.end,
            suggestions: mistake.suggestions.clone(),
        }
    }
}

impl ConflictProblem {
    /// The problem with what the answer adds to it.
    pub(crate) fn new(problem: Problem, extras: &crate::errors::Extras) -> Self {
        Self {
            problem,
            conflict: extras.conflict.as_deref().cloned(),
            taken_over_by: extras.taken_over_by.as_deref().cloned(),
            mistakes: extras.mistakes.as_deref().cloned(),
            open_day: extras.open_day.clone(),
            dependency_loop: extras.dependency_loop.as_deref().cloned(),
            date_effect_kinds: extras.date_effect_kinds.as_deref().cloned(),
            refused_values: extras.refused_values.as_deref().cloned(),
            hard_to_read: extras.hard_to_read.as_deref().cloned(),
        }
    }
}
