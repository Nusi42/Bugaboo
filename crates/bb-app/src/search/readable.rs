//! The readable fields of a person (QRY-05, ADR-0005 amendment M5, decision 5): the fields the
//! checker of `bb-searchlang` gets, built once per request from the field definitions, the levels
//! of the person's roles and *Who sees people*. The browser asks for the same fields through a
//! route, so both sides check a search against the same fields.
//!
//! A search looks at many projects at once and a role may set another level in each, so a field is
//! readable only when it is readable in **every** project the search looks at (and, for a field of
//! an issue type, for every type: the lowest level counts, as for a view that does not say its
//! type). A field that is hidden in one project is then as unknown as one that does not exist, and
//! no filter, sort or group can find out what lies in a project that hides it. An own field counts
//! only in the projects it is used in.

use std::sync::Arc;

use bb_core::access::{FieldLevel, PeopleLevel};
use bb_core::fields::{Field, FieldKey};
use bb_core::id::ProjectId;
use bb_core::issue::{Priority, StatusGroup};
use bb_core::own_fields::{FieldFor, FieldType};
use bb_policy::{Authorized, Evaluation, FieldMask, Masked, Place as Where};
use bb_searchlang::Field as SearchField;
use bb_searchlang::{FieldKind, Operator, ReadableFields, own_field_word};
use serde::Serialize;

use crate::access::Actor;
use crate::auth::{AppError, AuthStore, Ctx, internal};
use crate::fields::Definition;

/// The words of the fields every issue has and that no role can hide: they are not in the field
/// catalogue (what lets a person see them is seeing the issue), and the two values of a yes-no
/// field.
mod words {
    pub(super) const PROJECT: &str = "project";
    pub(super) const TYPE: &str = "type";
    pub(super) const STATUS_GROUP: &str = "statusGroup";
    pub(super) const PARENT: &str = "parent";
    pub(super) const CREATED: &str = "created";
    pub(super) const CHANGED: &str = "changed";
    pub(super) const CLOSED: &str = "closed";
    pub(super) const YES: &str = "yes";
    pub(super) const NO: &str = "no";
}

/// Where a search looks.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SearchScope {
    /// Every project the person sees.
    Everywhere,
    /// One project and the projects inside it that the person sees (the list of a project).
    Within(ProjectId),
}

/// Ask for the fields the person may search by.
pub struct ReadableFieldsQuery {
    /// The secret of the session of the request.
    pub session_token: String,
    /// Where the search looks.
    pub scope: SearchScope,
}

/// The readable fields as an answer: what the checker takes and the browser gets.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(transparent)]
pub struct SearchFields(pub ReadableFields);

impl Masked for SearchFields {}

/// The fields the person may search by, where a search looks.
///
/// # Errors
/// An internal error when the store fails.
pub async fn readable_fields<S: AuthStore>(
    ctx: &Ctx<'_, S>,
    query: ReadableFieldsQuery,
) -> Result<Authorized<SearchFields>, AppError> {
    let actor = ctx.actor(&query.session_token).await?;
    let fields = ctx.readable_fields_of(&actor, query.scope).await?;
    let held = ctx.evaluation_of(&actor, Where::Organization).await?;
    Ok(held.authorize(SearchFields(fields)))
}

impl<S: AuthStore> Ctx<'_, S> {
    /// The fields the actor may search by where a search looks, built once for a request: the one
    /// input of the checker, for a search that is typed, saved or sent.
    ///
    /// # Errors
    /// An internal error when the store fails.
    pub(crate) async fn readable_fields_of(
        &self,
        actor: &Actor,
        scope: SearchScope,
    ) -> Result<ReadableFields, AppError> {
        let seen = self.shown_with_levels_to(actor).await?;
        let within = match scope {
            SearchScope::Everywhere => None,
            SearchScope::Within(project) => Some(project),
        };
        let places: Vec<Place> = seen
            .seen_places(within)
            .into_iter()
            .map(|(chain, held)| Place::of(chain, &held))
            .collect();
        let sources = Sources {
            statuses: self
                .workflow_directory()
                .await?
                .status_keys()
                .iter()
                .map(ToString::to_string)
                .collect(),
            issue_types: self
                .store()
                .issue_types()
                .await
                .map_err(|error| internal::<S>(&error))?
                .into_iter()
                .map(|issue_type| issue_type.name)
                .collect(),
            own: self
                .store()
                .field_definitions()
                .await
                .map_err(|error| internal::<S>(&error))?,
        };
        Ok(fields_in(&places, &sources))
    }
}

/// What a person may do in one project a search looks at.
struct Place {
    /// The projects from the top down to it.
    chain: Vec<ProjectId>,
    /// What the person may do with each field, at the lowest level over the issue types.
    mask: FieldMask,
    /// Whether the person sees the names of the people in *Assigned to* and *Reported by*.
    sees_names: [bool; 2],
}

impl Place {
    fn of(chain: Vec<ProjectId>, held: &Arc<Evaluation>) -> Self {
        let names = |field: Field| {
            field
                .person_kind()
                .is_some_and(|kind| held.kind_level(kind) == PeopleLevel::Names)
        };
        Self {
            chain,
            mask: held.field_mask_for(None),
            sees_names: [names(Field::AssignedTo), names(Field::ReportedBy)],
        }
    }

    /// Whether the person may use the field here: they see it, and for a field that holds people
    /// they see the names (ADR-0022: a kind whose names are hidden cannot be searched).
    fn may_use(&self, field: FieldKey) -> bool {
        self.mask.level(field) >= FieldLevel::CanSee
            && match field {
                FieldKey::BuiltIn(Field::AssignedTo) => self.sees_names[0],
                FieldKey::BuiltIn(Field::ReportedBy) => self.sees_names[1],
                FieldKey::BuiltIn(_) | FieldKey::Own(_) => true,
            }
    }
}

/// What the fields of a search are made of besides the person's rights.
struct Sources {
    /// The keys of the statuses.
    statuses: Vec<String>,
    /// The names of the issue types.
    issue_types: Vec<String>,
    /// The own fields, with their choices.
    own: Vec<Definition>,
}

/// The kind of value of a field of the catalogue that a search can ask about; `None` for a field
/// whose values the issues do not hold yet (a later milestone adds its line here, and the compiler
/// its join).
const fn catalogue_kind(field: Field) -> Option<FieldKind> {
    match field {
        Field::Title | Field::Description => Some(FieldKind::Text),
        Field::Status | Field::Priority | Field::Labels | Field::Tags => Some(FieldKind::Choice),
        Field::AssignedTo | Field::ReportedBy => Some(FieldKind::Person),
        Field::DueDate
        | Field::PlannedForRelease
        | Field::FoundInRelease
        | Field::EstimatedTime
        | Field::TimeLeft
        | Field::TimeSpent
        | Field::Activity
        | Field::CostCenter => None,
    }
}

/// The values of a catalogue field with a fixed list, in their order.
fn catalogue_choices(field: Field, sources: &Sources) -> Vec<String> {
    match field {
        Field::Status => sources.statuses.clone(),
        Field::Priority => Priority::ALL
            .iter()
            .map(|each| each.code().to_owned())
            .collect(),
        _ => Vec::new(),
    }
}

/// The field of a search for an own field: its kind, what may be asked of it, and its choices.
fn own_search_field(word: &str, definition: &Definition) -> SearchField {
    let kind = match definition.field.kind {
        FieldType::Text | FieldType::LongText | FieldType::WebAddress => FieldKind::Text,
        FieldType::WholeNumber | FieldType::Decimal | FieldType::Duration => FieldKind::Number,
        FieldType::Date | FieldType::DateTime => FieldKind::Date,
        FieldType::PickOne | FieldType::PickSeveral | FieldType::YesNo | FieldType::Group => {
            FieldKind::Choice
        }
        FieldType::Person | FieldType::People => FieldKind::Person,
        FieldType::Issue => FieldKind::Issue,
    };
    let mut field = SearchField::new(word, kind);
    match definition.field.kind {
        FieldType::PickOne | FieldType::PickSeveral => {
            field.choices = definition
                .choices
                .iter()
                .map(|choice| choice.name.clone())
                .collect();
        }
        FieldType::YesNo => {
            field.choices = vec![words::YES.to_owned(), words::NO.to_owned()];
            field.operators = vec![
                Operator::Is,
                Operator::IsNot,
                Operator::IsEmpty,
                Operator::IsNotEmpty,
            ];
        }
        FieldType::Group => {
            field.operators = vec![
                Operator::Is,
                Operator::IsNot,
                Operator::In,
                Operator::IsEmpty,
                Operator::IsNotEmpty,
            ];
        }
        _ => {}
    }
    field
}

/// A field with a fixed list of values.
fn listed(word: &str, kind: FieldKind, choices: Vec<String>) -> SearchField {
    let mut field = SearchField::new(word, kind);
    field.choices = choices;
    field
}

/// The fields a search may use in these places: those of the catalogue and the own fields that
/// are readable in every place they apply to, and the fields every issue has. A word that is
/// already taken (an own field named like another field) stays with the first field that has it:
/// the built-in ones first.
fn fields_in(places: &[Place], sources: &Sources) -> ReadableFields {
    let mut fields: Vec<SearchField> = Vec::new();
    if places.is_empty() {
        return ReadableFields { fields };
    }
    let mut push = |field: SearchField| {
        if fields.iter().all(|taken| taken.word != field.word) {
            fields.push(field);
        }
    };
    for field in Field::ALL {
        let Some(kind) = catalogue_kind(*field) else {
            continue;
        };
        if places
            .iter()
            .all(|place| place.may_use(FieldKey::from(*field)))
        {
            push(listed(
                &field.search_word(),
                kind,
                catalogue_choices(*field, sources),
            ));
        }
    }
    push(listed(words::PROJECT, FieldKind::Project, Vec::new()));
    push(listed(
        words::TYPE,
        FieldKind::Choice,
        sources.issue_types.clone(),
    ));
    push(listed(
        words::STATUS_GROUP,
        FieldKind::Choice,
        StatusGroup::ALL
            .iter()
            .map(|group| group.code().to_owned())
            .collect(),
    ));
    push(listed(words::PARENT, FieldKind::Issue, Vec::new()));
    for word in [words::CREATED, words::CHANGED, words::CLOSED] {
        push(listed(word, FieldKind::Date, Vec::new()));
    }
    for definition in &sources.own {
        let reached: Vec<&Place> = places
            .iter()
            .filter(|place| definition.scope.reaches_project(&place.chain))
            .collect();
        let readable = definition.field.applies_to == FieldFor::Issues
            && !reached.is_empty()
            && reached
                .iter()
                .all(|place| place.may_use(FieldKey::Own(definition.field.id)));
        if readable {
            push(own_search_field(
                &own_field_word(&definition.field.name),
                definition,
            ));
        }
    }
    ReadableFields { fields }
}
