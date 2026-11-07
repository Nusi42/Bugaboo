//! The tables of groups, roles, who has which role, levels and access keys.

use super::{Iden, IdenStatic, StoredTable};

/// Groups of people. The table is called `person_group`: `group` is a word of SQL.
#[derive(Clone, Copy, Iden)]
pub(crate) enum GroupRow {
    #[iden = "person_group"]
    Table,
    Id,
    Name,
    Version,
}

#[derive(Clone, Copy, Iden)]
pub(crate) enum GroupMemberRow {
    #[iden = "group_member"]
    Table,
    Group,
    Person,
    AddedAt,
}

#[derive(Clone, Copy, Iden)]
pub(crate) enum RoleRow {
    #[iden = "role"]
    Table,
    Id,
    Name,
    Starting,
    Position,
    TwoStep,
    Version,
}

#[derive(Clone, Copy, Iden)]
pub(crate) enum RolePermissionRow {
    #[iden = "role_permission"]
    Table,
    Role,
    Permission,
}

/// *Who has which role*: a person or a group, a role or *No access* (no role), and where it counts.
#[derive(Clone, Copy, Iden)]
pub(crate) enum AssignmentRow {
    #[iden = "assignment"]
    Table,
    Id,
    Person,
    Group,
    Role,
    Project,
    AppliesTo,
    GivenAt,
}

#[derive(Clone, Copy, IdenStatic)]
pub(crate) enum RoleFieldLevelRow {
    #[iden = "role_field_level"]
    Table,
    Role,
    Field,
    Level,
}

#[derive(Clone, Copy, IdenStatic)]
pub(crate) enum ProjectFieldLevelRow {
    #[iden = "project_field_level"]
    Table,
    Project,
    Role,
    Field,
    Level,
}

#[derive(Clone, Copy, IdenStatic)]
pub(crate) enum RolePeopleLevelRow {
    #[iden = "role_people_level"]
    Table,
    Role,
    Kind,
    Level,
}

#[derive(Clone, Copy, IdenStatic)]
pub(crate) enum ProjectPeopleLevelRow {
    #[iden = "project_people_level"]
    Table,
    Project,
    Role,
    Kind,
    Level,
}

/// The access keys of people (AUTH-07). Only the hash of the secret is stored.
#[derive(Clone, Copy, Iden)]
pub(crate) enum AccessKeyRow {
    #[iden = "access_key"]
    Table,
    Id,
    Owner,
    Name,
    Prefix,
    SecretHash,
    Choice,
    CreatedAt,
    ExpiresAt,
    LastUsedAt,
    Agent,
}

/// The single permissions of a key whose choice is *More options*.
#[derive(Clone, Copy, Iden)]
pub(crate) enum AccessKeyPermissionRow {
    #[iden = "access_key_permission"]
    Table,
    Key,
    Permission,
}

/// The projects a key is limited to.
#[derive(Clone, Copy, Iden)]
pub(crate) enum AccessKeyProjectRow {
    #[iden = "access_key_project"]
    Table,
    Key,
    Project,
}

/// The people who were administrators in M1, until the first start gives them the role.
#[derive(Clone, Copy, Iden)]
pub(crate) enum LegacyAdministratorRow {
    #[iden = "legacy_administrator"]
    Table,
    Person,
}

/// One row, once the starting roles were made.
#[derive(Clone, Copy, Iden)]
pub(crate) enum StartingRolesMadeRow {
    #[iden = "starting_roles_made"]
    Table,
    MadeAt,
}

/// The starting roles that came after the others were made, still to make once (migration 32).
#[derive(Clone, Copy, Iden)]
pub(crate) enum StartingRoleDueRow {
    #[iden = "starting_role_due"]
    Table,
    Key,
}

stored_tables! {
    GroupRow => bb_core::access::Group {
        Id => id,
        Name => name,
        Version => version,
    },
    GroupMemberRow => bb_core::access::GroupMember {
        Group => group,
        Person => person,
        AddedAt => added_at,
    },
    RoleRow => bb_core::access::Role {
        Id => id,
        Name => name,
        Starting => starting,
        Position => position,
        TwoStep => two_step,
        Version => version,
    },
    RolePermissionRow => bb_core::access::RolePermission {
        Role => role,
        Permission => permission,
    },
    AssignmentRow => bb_core::access::Assignment {
        Id => id,
        Person => person,
        Group => group,
        Role => role,
        Project => project,
        AppliesTo => applies_to,
        GivenAt => given_at,
    },
    RoleFieldLevelRow => bb_core::access::RoleFieldLevel {
        Role => role,
        Field => field,
        Level => level,
    },
    ProjectFieldLevelRow => bb_core::access::ProjectFieldLevel {
        Project => project,
        Role => role,
        Field => field,
        Level => level,
    },
    RolePeopleLevelRow => bb_core::access::RolePeopleLevel {
        Role => role,
        Kind => kind,
        Level => level,
    },
    ProjectPeopleLevelRow => bb_core::access::ProjectPeopleLevel {
        Project => project,
        Role => role,
        Kind => kind,
        Level => level,
    },
    LegacyAdministratorRow => bb_core::access::LegacyAdministrator {
        Person => person,
    },
    StartingRolesMadeRow => bb_core::access::StartingRolesMade {
        MadeAt => made_at,
    },
    StartingRoleDueRow => bb_core::access::StartingRoleDue {
        Key => key,
    },
    AccessKeyRow => bb_core::access_key::AccessKey {
        Id => id,
        Owner => owner,
        Name => name,
        Prefix => prefix,
        SecretHash => secret_hash,
        Choice => choice,
        CreatedAt => created_at,
        ExpiresAt => expires_at,
        LastUsedAt => last_used_at,
        Agent => agent,
    },
    AccessKeyPermissionRow => bb_core::access_key::AccessKeyPermission {
        Key => key,
        Permission => permission,
    },
    AccessKeyProjectRow => bb_core::access_key::AccessKeyProject {
        Key => key,
        Project => project,
    },
}
