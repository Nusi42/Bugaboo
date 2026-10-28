//! Loading what the policy engine reads: the rows of *Who has which role*, the groups, the roles
//! and the levels they set, as one [`Policy`]; and from it the inputs for one person at one place.
//!
//! Which rows and rules apply at a place is decided once, by [`Policy::inputs_for`]. What is
//! loaded here is only narrowed in the query to what that decision can keep (the rows on the
//! organization and on the projects of the chain), so a request does not read the whole
//! organization.

use bb_core::access::{Assignment, GroupMember};
use bb_core::id::{GroupId, ProjectId, RoleId, UserId};
use bb_policy::guards::OrganizationState;
use bb_policy::{Inputs, LevelRules, Place, Policy, RoleData};
use sea_query::{Expr, ExprTrait as _};

use super::assignments::{assignment_select, assignments_of};
use super::groups::{member_select, read_member};
use super::levels::{Which, read_rules, read_typed_rules};
use super::roles::roles_in;
use super::{in_list, level_rules_of_roles_in};
use crate::db::{Reader, Transaction};
use crate::error::StoreError;
use crate::projects::chain_of;
use crate::schema::{AssignmentRow, GroupMemberRow};

/// What to load.
#[derive(Clone, Copy)]
pub(crate) enum Wanted<'a> {
    /// Everything, for everybody: what the preview of a move is made from.
    Everybody,
    /// The rows of one person (their own and their groups') wherever they are, with the roles
    /// those name. No levels: a decision about permissions does not need them.
    Person(UserId),
    /// The rows of these people (their own and their groups') wherever they are, with the roles
    /// those name. No levels: what a role label needs.
    People(&'a [UserId]),
    /// What one person's decision at a place needs: their rows on the organization and on the
    /// projects of the chain, the roles those name and the levels of those roles.
    PersonAt(UserId, &'a [ProjectId]),
    /// What the last-administrator rule needs: the rows on the whole organization that give a
    /// role, the roles those name and the members of the groups that hold them.
    Organization,
}

impl Wanted<'_> {
    /// The people whose rows are wanted; none for everybody's.
    fn people(&self) -> Option<&[UserId]> {
        match self {
            Self::Everybody | Self::Organization => None,
            Self::Person(person) | Self::PersonAt(person, _) => Some(std::slice::from_ref(person)),
            Self::People(people) => Some(people),
        }
    }
}

/// Everything the engine reads, as asked for.
pub(crate) async fn load_policy(
    reader: &mut impl Reader,
    wanted: Wanted<'_>,
) -> Result<Policy, StoreError> {
    let (members, assignments) = match wanted {
        Wanted::Organization => organization_rows(reader).await?,
        _ => rows_wanted(reader, wanted).await?,
    };
    let mut named: Vec<RoleId> = assignments.iter().filter_map(|row| row.role).collect();
    named.sort();
    named.dedup();
    let roles = match wanted {
        Wanted::Everybody => roles_in(reader, None).await?,
        _ if named.is_empty() => Vec::new(),
        _ => roles_in(reader, Some(&named)).await?,
    }
    .into_iter()
    .map(RoleData::from)
    .collect();
    let levels = match wanted {
        Wanted::Everybody => rules_of(reader, Which::All).await?,
        Wanted::PersonAt(_, chain) if !named.is_empty() => {
            rules_of(
                reader,
                Which::At {
                    roles: &named,
                    chain,
                },
            )
            .await?
        }
        // No role reaches the place: no level, but every own field is known (closed), as for a
        // person with a role (a move preview lists the field for who gets a role there).
        Wanted::PersonAt(..) => own_fields_only(reader).await?,
        _ => LevelRules::default(),
    };
    Ok(Policy {
        assignments,
        members,
        roles,
        rules: levels,
    })
}

/// The memberships and rows of one person or of everybody, as asked for.
async fn rows_wanted(
    reader: &mut impl Reader,
    wanted: Wanted<'_>,
) -> Result<(Vec<GroupMember>, Vec<Assignment>), StoreError> {
    let mut select = member_select();
    if let Some(people) = wanted.people() {
        select.and_where(Expr::col(GroupMemberRow::Person).is_in(in_list(people)));
    }
    let members = reader
        .fetch_all(&select)
        .await?
        .iter()
        .map(read_member)
        .collect::<Result<Vec<_>, _>>()?;
    let mut select = assignment_select();
    if let Some(people) = wanted.people() {
        let groups: Vec<GroupId> = members.iter().map(|member| member.group).collect();
        let mut whose = Expr::col(AssignmentRow::Person).is_in(in_list(people));
        if !groups.is_empty() {
            whose = whose.or(Expr::col(AssignmentRow::Group).is_in(in_list(&groups)));
        }
        select.and_where(whose);
    }
    if let Wanted::PersonAt(_, chain) = wanted {
        let mut here = Expr::col(AssignmentRow::Project).is_null();
        if !chain.is_empty() {
            here = here.or(Expr::col(AssignmentRow::Project).is_in(in_list(chain)));
        }
        select.and_where(here);
    }
    let assignments = assignments_of(reader, &select).await?;
    Ok((members, assignments))
}

/// The rows on the whole organization that give a role, and the members of the groups that hold
/// them.
async fn organization_rows(
    reader: &mut impl Reader,
) -> Result<(Vec<GroupMember>, Vec<Assignment>), StoreError> {
    let mut select = assignment_select();
    select
        .and_where(Expr::col(AssignmentRow::Project).is_null())
        .and_where(Expr::col(AssignmentRow::Role).is_not_null());
    let assignments = assignments_of(reader, &select).await?;
    let mut groups: Vec<GroupId> = assignments.iter().filter_map(|row| row.group).collect();
    groups.sort();
    groups.dedup();
    let mut members = Vec::new();
    if !groups.is_empty() {
        let mut select = member_select();
        select.and_where(Expr::col(GroupMemberRow::Group).is_in(in_list(&groups)));
        for row in reader.fetch_all(&select).await? {
            members.push(read_member(&row)?);
        }
    }
    Ok((members, assignments))
}

/// The levels of both kinds that roles have set.
pub(crate) async fn rules_of(
    reader: &mut impl Reader,
    which: Which<'_>,
) -> Result<LevelRules, StoreError> {
    Ok(LevelRules {
        fields: read_rules(reader, which).await?,
        people: read_rules(reader, which).await?,
        typed: read_typed_rules(reader, which).await?,
        own_fields: crate::fields::field_ids_in(reader).await?,
    })
}

/// What the engine needs to decide for `person` at any place, in one snapshot: their rows (their
/// own and their groups') wherever they are, the roles those name and the levels of those roles
/// for the organization and for all projects. Which of it applies at a place is decided by
/// [`Policy::inputs_from_rows`], not here.
pub(crate) async fn person_policy_in(
    reader: &mut impl Reader,
    person: UserId,
) -> Result<Policy, StoreError> {
    let mut policy = load_policy(reader, Wanted::Person(person)).await?;
    let roles = policy.role_ids();
    policy.rules = if roles.is_empty() {
        own_fields_only(reader).await?
    } else {
        level_rules_of_roles_in(reader, roles).await?
    };
    Ok(policy)
}

/// The levels of nobody: no rule, and the own fields there are, so that each has a level in an
/// evaluation (closed when no role says otherwise).
async fn own_fields_only(reader: &mut impl Reader) -> Result<LevelRules, StoreError> {
    Ok(LevelRules {
        own_fields: crate::fields::field_ids_in(reader).await?,
        ..LevelRules::default()
    })
}

/// Everything the engine needs for `person` at `place`, in one snapshot; `None` when the project
/// does not exist.
pub(crate) async fn inputs_in(
    transaction: &mut Transaction,
    person: UserId,
    place: Place,
) -> Result<Option<Inputs>, StoreError> {
    let chain = match place {
        Place::Organization => Vec::new(),
        Place::Project(project) => match chain_of(transaction, project).await? {
            Some(chain) => chain,
            None => return Ok(None),
        },
    };
    let policy = load_policy(transaction, Wanted::PersonAt(person, &chain)).await?;
    Ok(Some(policy.inputs_for(person, place, chain)))
}

/// What the last-administrator rule needs: the rows for the whole organization that give a role,
/// the roles, and the members of the groups that hold one.
pub(crate) async fn organization_state_in(
    transaction: &mut Transaction,
) -> Result<OrganizationState, StoreError> {
    let policy = load_policy(transaction, Wanted::Organization).await?;
    OrganizationState::of(&policy).map_err(|_| StoreError::Corrupt("a row for nobody"))
}
