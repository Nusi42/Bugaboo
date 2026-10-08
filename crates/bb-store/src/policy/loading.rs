//! Loading what the policy engine reads: the rows of *Who has which role*, the groups, the roles
//! and the levels they set, as one [`Policy`]; and from it the inputs for one person at one place.
//!
//! Which rows and rules apply at a place is decided once, by [`Policy::inputs_for`]. What is
//! loaded here is only narrowed in the query to what that decision can keep (the rows on the
//! organization and on the projects of the chain), so a request does not read the whole
//! organization.

use bb_core::id::{GroupId, ProjectId, RoleId, UserId};
use bb_policy::guards::{Holder, OrganizationState};
use bb_policy::{Inputs, LevelRules, Place, Policy, RoleData};
use sea_query::{Expr, ExprTrait as _, Query};

use super::assignments::{assignment_select, assignments_of};
use super::groups::{member_select, read_member};
use super::in_list;
use super::levels::{Which, read_rules};
use super::roles::roles_in;
use crate::db::{Reader, StoredId as _, Transaction};
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
    /// What one person's decision at a place needs: their rows on the organization and on the
    /// projects of the chain, the roles those name and the levels of those roles.
    PersonAt(UserId, &'a [ProjectId]),
}

impl Wanted<'_> {
    const fn person(self) -> Option<UserId> {
        match self {
            Self::Everybody => None,
            Self::Person(person) | Self::PersonAt(person, _) => Some(person),
        }
    }
}

/// Everything the engine reads, as asked for.
pub(crate) async fn load_policy(
    reader: &mut impl Reader,
    wanted: Wanted<'_>,
) -> Result<Policy, StoreError> {
    let mut select = member_select();
    if let Some(person) = wanted.person() {
        select.and_where(Expr::col(GroupMemberRow::Person).eq(person.stored()));
    }
    let members = reader
        .fetch_all(&select)
        .await?
        .iter()
        .map(read_member)
        .collect::<Result<Vec<_>, _>>()?;
    let mut select = assignment_select();
    if let Some(person) = wanted.person() {
        let groups: Vec<GroupId> = members.iter().map(|member| member.group).collect();
        let mut whose = Expr::col(AssignmentRow::Person).eq(person.stored());
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
        _ => LevelRules::default(),
    };
    Ok(Policy {
        assignments,
        members,
        roles,
        rules: levels,
    })
}

/// The levels of both kinds that roles have set.
pub(crate) async fn rules_of(
    reader: &mut impl Reader,
    which: Which<'_>,
) -> Result<LevelRules, StoreError> {
    Ok(LevelRules {
        fields: read_rules(reader, which).await?,
        people: read_rules(reader, which).await?,
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
    let mut select = assignment_select();
    select
        .and_where(Expr::col(AssignmentRow::Project).is_null())
        .and_where(Expr::col(AssignmentRow::Role).is_not_null());
    let assignments = assignments_of(transaction, &select).await?;
    let mut rows = Vec::new();
    let mut groups = Vec::new();
    for assignment in &assignments {
        let Some(role) = assignment.role else {
            continue;
        };
        match (assignment.person, assignment.group) {
            (Some(person), _) => rows.push((Holder::Person(person), role)),
            (None, Some(group)) => {
                rows.push((Holder::Group(group), role));
                groups.push(group);
            }
            (None, None) => return Err(StoreError::Corrupt("a row for nobody")),
        }
    }
    groups.sort();
    groups.dedup();
    let mut members = Vec::new();
    if !groups.is_empty() {
        let mut select = Query::select();
        select
            .columns([GroupMemberRow::Group, GroupMemberRow::Person])
            .from(GroupMemberRow::Table)
            .and_where(Expr::col(GroupMemberRow::Group).is_in(in_list(&groups)));
        for row in transaction.fetch_all(&select).await? {
            members.push((
                row.id::<GroupId>(GroupMemberRow::Group)?,
                row.id::<UserId>(GroupMemberRow::Person)?,
            ));
        }
    }
    let role_ids: Vec<RoleId> = rows.iter().map(|(_, role)| *role).collect();
    let roles = if role_ids.is_empty() {
        Vec::new()
    } else {
        roles_in(transaction, Some(&role_ids)).await?
    };
    Ok(OrganizationState {
        roles: roles.into_iter().map(RoleData::from).collect(),
        rows,
        members,
    })
}
