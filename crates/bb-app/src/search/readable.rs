//! The readable fields of a person (QRY-05, ADR-0005 amendment M5, decisions 5 and 9): the fields
//! the checker of `bb-searchlang` gets, built once per request from the field definitions, the
//! levels of the person's roles and *Who sees people*. The browser asks for the same fields through
//! a route, so both sides check a search against the same fields.
//!
//! A search looks at many projects at once and a role may set another level in each, so a field is
//! readable only when it is readable in **every** project the search looks at (and, for a field of
//! an issue type, for every type: the lowest level counts, as for a view that does not say its
//! type). A field that is hidden in one project is then as unknown as one that does not exist, and
//! no filter, sort or group can find out what lies in a project that hides it. An own field counts
//! only in the projects it is used in. A field that holds people is readable only where the person
//! sees their names (ADR-0022).
//!
//! **A word belongs to one field for everyone** (decision 9): the words of the built-in fields and
//! of the fields every issue has are reserved, also those the person may not read or that cannot
//! be searched yet, and between own fields the first in the order they are kept in owns a word.
//! This is decided before the rights of the person, so one canonical text means the same field for
//! every person it is shared with.
//!
//! **Values are written by their name as shown** (decision 9): the choices of a field are names,
//! each once, and only those the person may know: the types they may know (`known_types`) that can
//! reach a place the search looks at, and the statuses of the workflows in force there for those
//! types.

use std::sync::Arc;

use bb_core::ErrorCode;
use bb_core::access::FieldLevel;
use bb_core::fields::{Field, FieldKey};
use bb_core::id::{FieldId, ProjectId};
use bb_core::issue::{IssueType, Priority, StatusGroup};
use bb_core::names::name_key;
use bb_core::own_fields::{FieldFor, FieldType};
use bb_core::people_kinds::PeopleKind;
use bb_core::workflow::WorkflowStatus;
use bb_policy::{Authorized, Evaluation, FieldMask, Masked, Place as Where};
use bb_searchlang::Field as SearchField;
use bb_searchlang::{FieldKind, IssueField, Operator, ReadableFields, own_field_word};
use serde::Serialize;

use super::request::OwnReach;
use super::target::{FieldTarget, Searchable};
use crate::access::Actor;
use crate::auth::{AppError, AuthStore, Ctx, internal};
use crate::fields::Definition;

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
/// `resource.not-found` for a project that the person does not see (as one that does not exist);
/// an internal error when the store fails.
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
    /// `resource.not-found` for a project that the actor does not see; an internal error when the
    /// store fails.
    pub(crate) async fn readable_fields_of(
        &self,
        actor: &Actor,
        scope: SearchScope,
    ) -> Result<ReadableFields, AppError> {
        Ok(readable_of(&self.searchables_of(actor, scope).await?))
    }

    /// The same fields with what each one stands for, which the compiler needs: the one list the
    /// checker and the compiler both start from, so they cannot disagree about a word.
    ///
    /// # Errors
    /// `resource.not-found` for a project that the actor does not see; an internal error when the
    /// store fails.
    pub(crate) async fn searchables_of(
        &self,
        actor: &Actor,
        scope: SearchScope,
    ) -> Result<Vec<Searchable>, AppError> {
        Ok(self.looked_at(actor, scope).await?.fields)
    }

    /// What the person may search by where a search looks, and what the names in a search can
    /// stand for there: the fields, the statuses and issue types the person may know, the own
    /// fields, and the projects. Read once for a request.
    ///
    /// # Errors
    /// `resource.not-found` for a project that the actor does not see; an internal error when the
    /// store fails.
    pub(crate) async fn looked_at(
        &self,
        actor: &Actor,
        scope: SearchScope,
    ) -> Result<Looked, AppError> {
        let seen = self.shown_with_levels_to(actor).await?;
        self.looked_in(seen, scope).await
    }

    /// The same, from the tree and the levels of the person that the caller has read already (the
    /// saved searches read them once for a request and judge many texts with them).
    ///
    /// # Errors
    /// `resource.not-found` for a project that the person does not see; an internal error when the
    /// store fails.
    pub(crate) async fn looked_in(
        &self,
        seen: crate::projects::Leveled,
        scope: SearchScope,
    ) -> Result<Looked, AppError> {
        let within = match scope {
            SearchScope::Everywhere => None,
            SearchScope::Within(project) if seen.sees(project) => Some(project),
            // A project the person does not see is answered as one that does not exist.
            SearchScope::Within(_) => return Err(AppError::Refused(ErrorCode::NotFound)),
        };
        let places: Vec<Place> = seen
            .seen_places(within)
            .into_iter()
            .map(|(chain, held)| Place::of(chain, held))
            .collect();
        let mut types: Vec<IssueType> = Vec::new();
        let mut statuses: Vec<WorkflowStatus> = Vec::new();
        for place in &places {
            for (found, workflow) in self.workflows_reaching(&seen, &place.chain).await? {
                statuses.extend(workflow.statuses.iter().cloned());
                if !types.iter().any(|each| each.id == found.id) {
                    types.push(found);
                }
            }
        }
        let own = self
            .store()
            .field_definitions()
            .await
            .map_err(|error| internal::<S>(&error))?;
        let sources = Sources {
            statuses: once_each(statuses.iter().map(|status| status.name.clone())),
            issue_types: once_each(types.iter().map(|found| found.name.clone())),
            own: own.clone(),
        };
        let fields = fields_in(&places, &sources);
        let own_reach = own
            .iter()
            .filter(|definition| {
                fields.iter().any(|each| {
                    matches!(each.target, FieldTarget::Own { field, .. } if field == definition.field.id)
                })
            })
            .map(|definition| OwnReach {
                field: definition.field.id,
                projects: reached_places(&places, definition)
                    .filter_map(|place| place.chain.last().copied())
                    .collect(),
                issue_types: definition.scope.limited_types().map(<[_]>::to_vec),
            })
            .collect();
        Ok(Looked {
            fields,
            own_reach,
            statuses,
            types,
            own,
            projects: places
                .iter()
                .filter_map(|place| place.chain.last().copied())
                .collect(),
            seen,
        })
    }
}

/// What a search looks at for one person: the fields they may use and what names can stand for.
pub(crate) struct Looked {
    /// The fields the person may use, with what each reads.
    pub(crate) fields: Vec<Searchable>,
    /// The statuses of the workflows in force where the search looks, for the types the person
    /// may know.
    pub(crate) statuses: Vec<WorkflowStatus>,
    /// The issue types the person may know that reach where the search looks.
    pub(crate) types: Vec<IssueType>,
    /// Every own field, with its choices.
    pub(crate) own: Vec<Definition>,
    /// Where the values of each own field the person may read count.
    pub(crate) own_reach: Vec<OwnReach>,
    /// The projects the search looks at.
    pub(crate) projects: Vec<ProjectId>,
    /// The tree as the person sees it, with the levels.
    pub(crate) seen: crate::projects::Leveled,
}

/// The names, each once (names that are the same name are one), in the order they come.
fn once_each(names: impl IntoIterator<Item = String>) -> Vec<String> {
    let mut seen = std::collections::HashSet::new();
    names
        .into_iter()
        .filter(|name| seen.insert(name_key(name)))
        .collect()
}

/// The fields of the language, from the fields with their targets.
pub(crate) fn readable_of(searchables: &[Searchable]) -> ReadableFields {
    ReadableFields {
        fields: searchables
            .iter()
            .map(|searchable| searchable.field.clone())
            .collect(),
    }
}

/// What a person may do in one project a search looks at.
struct Place {
    /// The projects from the top down to it.
    chain: Vec<ProjectId>,
    /// What the person may do with each field, at the lowest level over the issue types.
    mask: FieldMask,
    /// What the person may do here.
    held: Arc<Evaluation>,
}

impl Place {
    fn of(chain: Vec<ProjectId>, held: Arc<Evaluation>) -> Self {
        Self {
            chain,
            mask: held.field_mask_for(None),
            held,
        }
    }

    /// Whether the person may use the field here: they see it, and for a field that holds people
    /// they see the names (ADR-0022: a kind whose names are hidden cannot be searched).
    fn may_use(&self, field: FieldKey, people: Option<PeopleKind>) -> bool {
        self.mask.level(field) >= FieldLevel::CanSee
            && people.is_none_or(|kind| self.held.sees_names(kind))
    }
}

/// What the fields of a search are made of besides the person's rights.
struct Sources {
    /// The names of the statuses the issues can have in the places.
    statuses: Vec<String>,
    /// The names of the issue types the person may know in the places.
    issue_types: Vec<String>,
    /// Every own field, with its choices, in the order they are kept in.
    own: Vec<Definition>,
}

/// The places of the search that an own field reaches: the one filter, for where its values count
/// and for whether the person may read it.
fn reached_places<'a>(
    places: &'a [Place],
    definition: &'a Definition,
) -> impl Iterator<Item = &'a Place> {
    places
        .iter()
        .filter(|place| definition.scope.reaches_project(&place.chain))
}

/// What a field of the catalogue that a search can ask about stands for, with the kind of its
/// values; `None` for a field whose values the issues do not hold yet (a later milestone adds its
/// line here, and the compiler its join). Its word is reserved all the same.
const fn catalogue_target(field: Field) -> Option<(FieldTarget, FieldKind)> {
    match field {
        Field::Title => Some((FieldTarget::Title, FieldKind::Text)),
        Field::Description => Some((FieldTarget::Description, FieldKind::Text)),
        Field::Status => Some((FieldTarget::Status, FieldKind::Choice)),
        Field::Priority => Some((FieldTarget::Priority, FieldKind::Choice)),
        Field::Labels => Some((FieldTarget::Labels, FieldKind::Choice)),
        Field::Tags => Some((FieldTarget::Tags, FieldKind::Choice)),
        Field::AssignedTo => Some((FieldTarget::AssignedTo, FieldKind::Person)),
        Field::ReportedBy => Some((FieldTarget::ReportedBy, FieldKind::Person)),
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

/// The field of the language for a field of the catalogue, with the names of its values.
fn catalogue_field(field: Field, kind: FieldKind, sources: &Sources) -> SearchField {
    let word = field.search_word();
    let found = match field {
        // A status, tags and labels: any number of values with no order among them.
        Field::Status | Field::Labels | Field::Tags => SearchField::unordered(&word),
        _ => SearchField::new(&word, kind),
    };
    let choices = match field {
        Field::Status => sources.statuses.clone(),
        Field::Priority => Priority::ALL
            .iter()
            .map(|each| each.search_word())
            .collect(),
        _ => Vec::new(),
    };
    SearchField { choices, ..found }
}

/// The field of the language for an own field: its kind, what may be asked of it, and the names
/// of its choices.
fn own_search_field(word: &str, definition: &Definition) -> SearchField {
    match definition.field.kind {
        FieldType::Text | FieldType::LongText | FieldType::WebAddress => {
            SearchField::new(word, FieldKind::Text)
        }
        FieldType::WholeNumber | FieldType::Decimal | FieldType::Duration => {
            SearchField::new(word, FieldKind::Number)
        }
        FieldType::Date | FieldType::DateTime => SearchField::new(word, FieldKind::Date),
        FieldType::PickOne | FieldType::PickSeveral => SearchField {
            choices: once_each(definition.choices.iter().map(|choice| choice.name.clone())),
            ..SearchField::new(word, FieldKind::Choice)
        },
        FieldType::YesNo => SearchField::yes_no(word),
        FieldType::Group => SearchField::unordered(word),
        FieldType::Person | FieldType::People => SearchField::new(word, FieldKind::Person),
        FieldType::Issue => SearchField::new(word, FieldKind::Issue),
    }
}

/// The words that belong to the built-in fields for everyone: those of the catalogue (also the
/// ones that cannot be searched yet) and those of the fields every issue has.
pub(crate) fn reserved_words() -> Vec<String> {
    Field::ALL
        .iter()
        .map(|field| field.search_word())
        .chain(IssueField::ALL.iter().map(|each| each.word().to_owned()))
        .collect()
}

/// Who owns a word of the search language.
#[derive(Clone, Copy, PartialEq, Eq)]
enum WordOwner {
    /// A built-in field, or a field every issue has: also one the person may not read or that
    /// cannot be searched yet.
    Language,
    /// An own field for issues: the first of them, in the order they are kept in.
    Field(FieldId),
}

/// Who owns this word, decided from the definitions alone, so it is the same for everybody. The one
/// rule for the words the search reads ([`own_words`]) and for the names that are refused
/// ([`word_is_taken`]). A field for projects owns no word: issues are what a search finds.
fn word_owner(word: &str, own: &[Definition]) -> Option<WordOwner> {
    if reserved_words().iter().any(|each| each == word) {
        return Some(WordOwner::Language);
    }
    own.iter()
        .find(|definition| {
            definition.field.applies_to == FieldFor::Issues
                && own_field_word(&definition.field.name) == word
        })
        .map(|definition| WordOwner::Field(definition.field.id))
}

/// Whether the word that `name` would make for an own field of this kind is owned by the language
/// or by another field (not `except`, the field itself when it is renamed). A name like that is
/// refused when a field is made or renamed (ADR-0005 amendment M5, decision 9), so a word never
/// changes hands.
pub(crate) fn word_is_taken(
    own: &[Definition],
    name: &str,
    applies_to: FieldFor,
    except: Option<FieldId>,
) -> bool {
    applies_to == FieldFor::Issues
        && match word_owner(&own_field_word(name), own) {
            None => false,
            Some(WordOwner::Language) => true,
            Some(WordOwner::Field(owner)) => Some(owner) != except,
        }
}

/// The word each own field is searched by, `None` for one that does not own it.
fn own_words(own: &[Definition]) -> Vec<Option<String>> {
    own.iter()
        .map(|definition| {
            let word = own_field_word(&definition.field.name);
            (word_owner(&word, own) == Some(WordOwner::Field(definition.field.id))).then_some(word)
        })
        .collect()
}

/// The words of the status groups in the search language.
fn status_group_names() -> Vec<String> {
    StatusGroup::ALL
        .iter()
        .map(|group| group.search_word())
        .collect()
}

/// The fields a search may use in these places: those of the catalogue and the own fields that
/// are readable in every place they apply to, and the fields every issue has.
fn fields_in(places: &[Place], sources: &Sources) -> Vec<Searchable> {
    let mut found: Vec<Searchable> = Vec::new();
    if places.is_empty() {
        return found;
    }
    for field in Field::ALL {
        let Some((target, kind)) = catalogue_target(*field) else {
            continue;
        };
        if places
            .iter()
            .all(|place| place.may_use(FieldKey::from(*field), field.person_kind()))
        {
            found.push(Searchable {
                field: catalogue_field(*field, kind, sources),
                target,
            });
        }
    }
    for (each, target, choices) in [
        (IssueField::Project, FieldTarget::Project, Vec::new()),
        (
            IssueField::Type,
            FieldTarget::Type,
            sources.issue_types.clone(),
        ),
        (
            IssueField::StatusGroup,
            FieldTarget::StatusGroup,
            status_group_names(),
        ),
        (IssueField::Parent, FieldTarget::Parent, Vec::new()),
        (IssueField::Created, FieldTarget::Created, Vec::new()),
        (IssueField::Changed, FieldTarget::Changed, Vec::new()),
        (IssueField::Closed, FieldTarget::Closed, Vec::new()),
    ] {
        found.push(Searchable {
            field: each.field(choices),
            target,
        });
    }
    for (definition, word) in sources.own.iter().zip(own_words(&sources.own)) {
        let Some(word) = word else {
            continue;
        };
        let reached: Vec<&Place> = reached_places(places, definition).collect();
        let readable = !reached.is_empty()
            && reached.iter().all(|place| {
                place.may_use(
                    FieldKey::Own(definition.field.id),
                    definition.field.kind.people_kind(),
                )
            });
        if readable {
            found.push(Searchable {
                field: own_search_field(&word, definition),
                target: FieldTarget::Own {
                    field: definition.field.id,
                    kind: definition.field.kind,
                },
            });
        }
    }
    // `contains` is offered where the index of the text search holds the text, and nowhere else.
    for each in &mut found {
        if each.target.text_place().is_none() {
            each.field
                .operators
                .retain(|operator| *operator != Operator::Contains);
        }
    }
    found
}
