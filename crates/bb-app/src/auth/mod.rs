//! Signing in (AUTH-01…03, AUTH-06, AUTH-09, SEC-12, SEC-14): the use cases.
//!
//! The use cases are plain `async fn`s that take the context [`Ctx`] and an input type
//! (`session.rs`, `second_factor.rs`, `account.rs`, `invitations.rs`). They depend on storage only
//! through the port [`AuthStore`]. [`AuthApi`] lists them once for the HTTP layer: it is what
//! `bb-api` holds, with the store behind it erased, so `bb-api` never names an adapter.

mod access_keys;
mod account;
mod ctx;
mod error;
mod events;
mod invitations;
mod limit;
mod request_session;
mod second_factor;
mod session;
mod store;
mod types;

use std::future::Future;
use std::pin::Pin;

use bb_policy::Authorized;

use crate::access::{
    FieldPermissions, FieldPermissionsQuery, OwnLoss, OwnLossQuery, PeopleToCheckPage,
    PeopleToCheckQuery, PeopleVisibility, PeopleVisibilityQuery, field_permissions, own_loss,
    people_to_check, people_visibility,
};
use crate::editing::{
    AskToFinish, CheckVersionsQuery, DraftSlot, DraftView, EditAnyway, HeartbeatAnswer,
    HeartbeatQuery, LockHolder, LockOutcome, ReadDraftQuery, ReadLockQuery, ReleaseLockQuery,
    ReleaseStuckLock, SaveDraftQuery, TakeLockQuery, ThrowAwayDraftQuery, Versions, ask_to_finish,
    check_versions, edit_anyway, heartbeat, read_draft, read_lock, release_lock,
    release_stuck_lock, save_draft, take_lock, throw_away_draft,
};
use crate::fields::{
    AddChoice, ChangeChoice, ChangeField, CombineChoices, FieldPage, FieldQuery, FieldTypePage,
    FieldTypesQuery, FieldUsage, FieldView, FieldsQuery, MakeField, ReorderChoices, SaveFieldText,
    add_choice, change_choice, change_field, combine_choices, field, field_types, field_usage,
    fields, make_field, reorder_choices, save_field_text,
};
use crate::issues::{
    AssignablePeoplePage, AssignablePeopleQuery, ChangeIssue, ChangeIssueType, DeleteIssue,
    IssueDetail, IssuePage, IssueQuery, IssueTypePage, IssueTypeView, IssueTypesQuery, IssueView,
    IssuesQuery, MakeIssue, MakeIssueType, RestoreIssue, SaveDescription, assignable_people,
    change_issue, change_issue_type, delete_issue, issue, issue_types, issues, make_issue,
    make_issue_type, restore_issue, save_description,
};
use crate::people::{
    AddGroupMember, AssignmentChoicesQuery, ChoicePage, DeleteGroup, GroupDetail, GroupList,
    GroupMembers, GroupMembersQuery, GroupQuery, GroupView, GroupsQuery, MakeGroup,
    MemberChoicesQuery, PeopleQuery, PersonDetail, PersonPage, PersonQuery, RemoveGroupMember,
    RenameGroup, add_group_member, delete_group, group, group_members, groups, make_group,
    member_choices, people, person, remove_group_member, rename_group,
};
use crate::projects::{
    ChangeProject, MakeProject, MovePreview, MovePreviewQuery, MoveProject, ProjectDetail,
    ProjectPage, ProjectQuery, ProjectView, ProjectsQuery, change_project, make_project,
    move_project, preview_move, project, projects,
};
use crate::roles::{
    AccessAnswer, AssignRole, AssignmentList, AssignmentView, AssignmentsQuery, ChangeRole,
    CheckAccessQuery, CopyRole, DeleteRole, Levels, LevelsQuery, MakeRole, PermissionCatalogue,
    PermissionsQuery, RemoveAssignment, ResetRole, RoleDetail, RoleList, RoleQuery, RoleView,
    RolesQuery, SetFieldLevel, SetPeopleLevel, assign_role, assignment_choices, assignments,
    change_role, check_access, copy_role, delete_role, make_role, permissions, remove_assignment,
    reset_role, role, role_levels, roles, set_level,
};
use crate::settings::{
    ChangeSetting, DocumentImagesQuery, ImageHosts, ImageHostsQuery, LockSetting, ResetSetting,
    SettingList, SettingView, SettingsQuery, change_setting, document_image_hosts, image_hosts,
    lock_setting, reset_setting, settings,
};

#[cfg(test)]
pub(crate) use access_keys::narrowing_of;
pub use access_keys::{
    access_key_rules, access_keys, delete_access_key, make_access_key, project_choices,
    require_session, see_server_status,
};
pub use account::{accept_invitation, change_password, complete_setup, issue_setup_code, sign_up};
pub(crate) use ctx::EventMaker;
pub use ctx::{AuthServices, Ctx, RequestInfo};
pub use error::AppError;
pub(crate) use error::{Stops, change_failed, if_there, internal};
pub(crate) use events::{by, changed, entity, fixed_entity, set_to};
pub use invitations::{
    change_sign_up_policy, create_invitation, invitation_lookup, invitations, revoke_invitation,
    sign_up_policy,
};
#[cfg(any(test, feature = "testing"))]
pub use limit::NoFailureLimit;
pub use limit::{FailureLimit, NETWORK_KEY_PREFIX};
pub use second_factor::{
    BACKUP_CODE_COUNT, MAX_PASSKEYS, add_passkey, confirm_code_app, passkey_registration_options,
    passkey_second_step_options, passkey_sign_in, passkey_sign_in_options, remove_code_app,
    remove_passkey, renew_backup_codes, start_code_app, two_step,
};
pub use session::{
    authenticate, end_session, passkey_second_step, second_step, second_step_gap, sessions,
    sign_in, sign_out, sign_out_everywhere, status,
};
pub use store::{
    AccessKeyRecord, AccessKeyStore, Account, AccountStore, AuthSettings, AuthStore,
    InvitationStore, PeoplePosition, SearchIn, SecondStepStore, SecurityEvent, SessionRecord,
    SessionStore,
};
pub use types::*;

/// A future that can move between threads, which is what an HTTP handler needs to wait for.
pub type Answer<'a, T> = Pin<Box<dyn Future<Output = Result<T, AppError>> + Send + 'a>>;

/// Declares [`AuthApi`] and implements it for [`AuthServices`]: one line per use case, the
/// name of the function above, its input and its output.
macro_rules! use_cases {
    ($($(#[$meta:meta])* $name:ident($input:ty) -> $output:ty $(= $via:ident)?;)+) => {
        /// Every use case of [`AuthApi`] by the name of the function that does it, with the name
        /// of its input type: what tests read to make sure none is left out (a use case whose
        /// input ends in `Query` only reads). Two requests may share one function (`= via`).
        pub const USE_CASES: &[(&str, &str)] =
            &[$((function_name!($name $($via)?), stringify!($input))),+];

        /// Every use case the HTTP layer calls, with the store behind it erased.
        pub trait AuthApi: Send + Sync {
            /// Whether `presented` (the `x-csrf-token` header) is the token of the session
            /// behind `session_token` (the cookie).
            fn csrf_matches(&self, session_token: &str, presented: &str) -> bool;

            $($(#[$meta])*
            fn $name(&self, request: RequestInfo, input: $input) -> Answer<'_, $output>;)+
        }

        impl<S: AuthStore + 'static> AuthApi for AuthServices<S> {
            fn csrf_matches(&self, session_token: &str, presented: &str) -> bool {
                Self::csrf_matches(self, session_token, presented)
            }

            $(fn $name(&self, request: RequestInfo, input: $input) -> Answer<'_, $output>
            {
                Box::pin(async move {
                    let ctx = self.ctx(request);
                    match function!($name $($via)?)(&ctx, input).await {
                        // The one place where a stale version becomes the structured answer.
                        Err(AppError::Stale(stale)) => Err(ctx.conflict_of(&stale).await),
                        answer => answer,
                    }
                })
            })+
        }
    };
}

/// The name of the function that does a use case: its own name, or the one it shares.
macro_rules! function_name {
    ($name:ident) => {
        stringify!($name)
    };
    ($name:ident $via:ident) => {
        stringify!($via)
    };
}

/// The function that does a use case, as a path to call.
macro_rules! function {
    ($name:ident) => {
        $name
    };
    ($name:ident $via:ident) => {
        $via
    };
}

use_cases! {
    /// Whether setup is open and whether people may sign up.
    status(StatusQuery) -> AuthStatus;
    /// Who is behind a session.
    authenticate(AuthenticateQuery) -> Authenticated;
    /// Signs in with e-mail address and password.
    sign_in(SignIn) -> Started;
    /// Finishes signing in with a code.
    second_step(SecondStep) -> Started;
    /// Starts a passkey as the second step.
    passkey_second_step_options(PasskeySecondStepQuery) -> PasskeyChallenge;
    /// Finishes signing in with a passkey after the password.
    passkey_second_step(PasskeySecondStep) -> Started;
    /// Starts signing in with a passkey alone.
    passkey_sign_in_options(PasskeySignInQuery) -> PasskeyChallenge;
    /// Signs in with a passkey alone.
    passkey_sign_in(PasskeySignIn) -> Started;
    /// Ends the session of the request.
    sign_out(SignOut) -> Done;
    /// Ends all sessions of the person.
    sign_out_everywhere(SignOutEverywhere) -> Done;
    /// Ends one session of the person.
    end_session(EndSession) -> SessionEnded;
    /// Lists the sessions of the person.
    sessions(SessionsQuery) -> SessionList;
    /// Changes the password.
    change_password(ChangePassword) -> Started;
    /// How the second step is set up.
    two_step(TwoStepQuery) -> TwoStepStatus;
    /// Starts setting up the code app.
    start_code_app(StartCodeApp) -> CodeAppSetup;
    /// Finishes setting up the code app.
    confirm_code_app(ConfirmCodeApp) -> BackupCodes;
    /// Switches the code app off.
    remove_code_app(RemoveCodeApp) -> Done;
    /// Makes new backup codes.
    renew_backup_codes(RenewBackupCodes) -> BackupCodes;
    /// Starts adding a passkey.
    passkey_registration_options(PasskeyRegistrationQuery) -> PasskeyChallenge;
    /// Adds a passkey.
    add_passkey(AddPasskey) -> PasskeyView;
    /// Removes a passkey.
    remove_passkey(RemovePasskey) -> Done;
    /// Judges who asks before the rest of a request is read.
    require_session(RequireSessionQuery) -> Done;
    /// Whether the person (or the access key) may see how the server is doing.
    see_server_status(SeeServerStatusQuery) -> Authorized<Done>;
    /// Makes an access key.
    make_access_key(MakeAccessKey) -> Authorized<CreatedAccessKey>;
    /// Deletes an access key.
    delete_access_key(DeleteAccessKey) -> Done;
    /// Lists the access keys of the person.
    access_keys(AccessKeysQuery) -> Authorized<AccessKeyPage>;
    /// How long a new access key lives in this organization.
    access_key_rules(AccessKeyRulesQuery) -> Authorized<AccessKeyRules>;
    /// Lists the projects a person may choose for an access key.
    project_choices(ProjectChoicesQuery) -> Authorized<ProjectChoicePage>;
    /// How many people with a role have no second step yet.
    second_step_gap(SecondStepGapQuery) -> Authorized<SecondStepGap>;
    /// Makes an invitation.
    create_invitation(CreateInvitation) -> Authorized<CreatedInvitation>;
    /// Takes an invitation back.
    revoke_invitation(RevokeInvitation) -> Done;
    /// Lists the open invitations.
    invitations(InvitationsQuery) -> Authorized<InvitationPage>;
    /// Shows which address an invitation is for.
    invitation_lookup(InvitationLookupQuery) -> InvitationLookup;
    /// Makes an account from an invitation.
    accept_invitation(AcceptInvitation) -> Started;
    /// Makes an account without an invitation.
    sign_up(SignUp) -> Started;
    /// Who may sign up on their own.
    sign_up_policy(SignUpPolicyQuery) -> Authorized<SignUpPolicy>;
    /// Changes who may sign up on their own.
    change_sign_up_policy(ChangeSignUpPolicy) -> Authorized<SignUpPolicy>;
    /// Makes the first administrator with the setup code.
    complete_setup(CompleteSetup) -> Started;
    /// Makes a group.
    make_group(MakeGroup) -> Authorized<GroupView>;
    /// Renames a group.
    rename_group(RenameGroup) -> Authorized<GroupView>;
    /// Deletes a group.
    delete_group(DeleteGroup) -> Done;
    /// Puts a person into a group.
    add_group_member(AddGroupMember) -> Done;
    /// Takes a person out of a group.
    remove_group_member(RemoveGroupMember) -> Done;
    /// Lists the groups.
    groups(GroupsQuery) -> Authorized<GroupList>;
    /// Lists the people in a group.
    group_members(GroupMembersQuery) -> Authorized<GroupMembers>;
    /// Shows one group with what it is used for.
    group(GroupQuery) -> Authorized<GroupDetail>;
    /// Lists the people, with their groups and roles.
    people(PeopleQuery) -> Authorized<PersonPage>;
    /// Shows one person with their groups and roles.
    person(PersonQuery) -> Authorized<PersonDetail>;
    /// Lists the people that can be put into a group.
    member_choices(MemberChoicesQuery) -> Authorized<ChoicePage>;
    /// Lists the people, groups or roles a row of *Who has which role* can be made from.
    assignment_choices(AssignmentChoicesQuery) -> Authorized<ChoicePage>;
    /// Makes a role.
    make_role(MakeRole) -> Authorized<RoleView>;
    /// Changes a role.
    change_role(ChangeRole) -> Authorized<RoleView>;
    /// Deletes a role.
    delete_role(DeleteRole) -> Done;
    /// Makes a role from a copy of another.
    copy_role(CopyRole) -> Authorized<RoleView>;
    /// Shows one role with what it is used for.
    role(RoleQuery) -> Authorized<RoleDetail>;
    /// Lists the permissions of the catalogue.
    permissions(PermissionsQuery) -> Authorized<PermissionCatalogue>;
    /// Gives a starting role its starting permissions back.
    reset_role(ResetRole) -> Authorized<RoleView>;
    /// Lists the roles.
    roles(RolesQuery) -> Authorized<RoleList>;
    /// Gives a role, or *No access*, to a person or a group.
    assign_role(AssignRole) -> Authorized<AssignmentView>;
    /// Takes a row of *Who has which role* away.
    remove_assignment(RemoveAssignment) -> Done;
    /// Lists *Who has which role* for a place.
    assignments(AssignmentsQuery) -> Authorized<AssignmentList>;
    /// Sets or takes away the level of a field for a role.
    set_field_level(SetFieldLevel) -> Done = set_level;
    /// Sets or takes away how a role sees a kind of people information.
    set_people_level(SetPeopleLevel) -> Done = set_level;
    /// Lists the levels a role has set in a place.
    role_levels(LevelsQuery) -> Authorized<Levels>;
    /// Checks what a person can do in a place, and why.
    check_access(CheckAccessQuery) -> Authorized<AccessAnswer>;
    /// Lists which level every role has for every field.
    field_permissions(FieldPermissionsQuery) -> Authorized<FieldPermissions>;
    /// Lists how every role sees every kind of people information.
    people_visibility(PeopleVisibilityQuery) -> Authorized<PeopleVisibility>;
    /// Lists the people *Check access* can choose from.
    people_to_check(PeopleToCheckQuery) -> Authorized<PeopleToCheckPage>;
    /// Says what a change of who has which role would take away from the person who makes it.
    own_loss(OwnLossQuery) -> Authorized<OwnLoss>;
    /// Makes a project.
    make_project(MakeProject) -> Authorized<ProjectView>;
    /// Lists the settings of the organization, of a project or of the person who asks.
    settings(SettingsQuery) -> Authorized<SettingList>;
    /// Changes the value of a setting on one scope.
    change_setting(ChangeSetting) -> Authorized<SettingView>;
    /// Takes the value of a setting on one scope away (*Reset to default*).
    reset_setting(ResetSetting) -> Authorized<SettingView>;
    /// Locks or unlocks the organization's value of a setting.
    lock_setting(LockSetting) -> Authorized<SettingView>;
    /// Changes the name or the short name of a project.
    change_project(ChangeProject) -> Authorized<ProjectView>;
    /// Moves a project, after the preview.
    move_project(MoveProject) -> Authorized<ProjectView>;
    /// Says what moving a project would do.
    preview_move(MovePreviewQuery) -> Authorized<MovePreview>;
    /// Shows one project by its address.
    project(ProjectQuery) -> Authorized<ProjectDetail>;
    /// Lists the projects a person sees.
    projects(ProjectsQuery) -> Authorized<ProjectPage>;
    /// Takes the lock on a long text, or says who is editing it.
    take_lock(TakeLockQuery) -> Authorized<LockOutcome>;
    /// Says who is editing a long text, without taking the lock.
    read_lock(ReadLockQuery) -> Authorized<LockHolder>;
    /// Renews the lock on a long text and keeps the text so far as the draft.
    heartbeat(HeartbeatQuery) -> Authorized<HeartbeatAnswer>;
    /// Lets go of a lock.
    release_lock(ReleaseLockQuery) -> Done;
    /// Asks the person who is editing to finish.
    ask_to_finish(AskToFinish) -> Authorized<Done>;
    /// Takes the editing over from the person who is editing.
    edit_anyway(EditAnyway) -> Authorized<LockOutcome>;
    /// Releases a lock that is stuck.
    release_stuck_lock(ReleaseStuckLock) -> Authorized<Done>;
    /// Keeps a text as the draft of the person.
    save_draft(SaveDraftQuery) -> Authorized<DraftView>;
    /// The draft of the person for a target.
    read_draft(ReadDraftQuery) -> Authorized<DraftSlot>;
    /// Throws the draft of the person away.
    throw_away_draft(ThrowAwayDraftQuery) -> Done;
    /// The versions of the records on the screen.
    check_versions(CheckVersionsQuery) -> Authorized<Versions>;
    /// Lists the issue types.
    issue_types(IssueTypesQuery) -> Authorized<IssueTypePage>;
    /// Makes an issue type.
    make_issue_type(MakeIssueType) -> Authorized<IssueTypeView>;
    /// Changes the name, icon or retired state of an issue type.
    change_issue_type(ChangeIssueType) -> Authorized<IssueTypeView>;
    /// Lists the issues of a project a person sees.
    issues(IssuesQuery) -> Authorized<IssuePage>;
    /// Lists the people an issue of a project can be assigned to.
    assignable_people(AssignablePeopleQuery) -> Authorized<AssignablePeoplePage>;
    /// Makes an issue.
    make_issue(MakeIssue) -> Authorized<IssueView>;
    /// Shows one issue by its readable or internal ID.
    issue(IssueQuery) -> Authorized<IssueDetail>;
    /// Changes the fields of an issue.
    change_issue(ChangeIssue) -> Authorized<IssueView>;
    /// The hosts images in text may come from, for a signed-in person.
    image_hosts(ImageHostsQuery) -> Authorized<ImageHosts>;
    /// The same for the document, before anybody signs in.
    document_image_hosts(DocumentImagesQuery) -> ImageHosts;
    /// Saves the description of an issue under the edit lock the person holds.
    save_description(SaveDescription) -> Authorized<IssueView>;
    /// Puts an issue into the trash.
    delete_issue(DeleteIssue) -> Done;
    /// Takes an issue out of the trash.
    restore_issue(RestoreIssue) -> Authorized<IssueView>;
    /// Lists the own fields a person sees.
    fields(FieldsQuery) -> Authorized<FieldPage>;
    /// Lists the kinds of information a field can hold.
    field_types(FieldTypesQuery) -> Authorized<FieldTypePage>;
    /// Shows one own field.
    field(FieldQuery) -> Authorized<FieldView>;
    /// Says where an own field is used.
    field_usage(FieldQuery) -> Authorized<FieldUsage>;
    /// Makes an own field.
    make_field(MakeField) -> Authorized<FieldView>;
    /// Changes an own field.
    change_field(ChangeField) -> Authorized<FieldView>;
    /// Adds a choice to an own field.
    add_choice(AddChoice) -> Authorized<FieldView>;
    /// Changes a choice of an own field.
    change_choice(ChangeChoice) -> Authorized<FieldView>;
    /// Puts the choices of an own field in a new order.
    reorder_choices(ReorderChoices) -> Authorized<FieldView>;
    /// Moves every value of a choice to another choice.
    combine_choices(CombineChoices) -> Authorized<FieldView>;
    /// Saves the long text of an own field of an issue, under its edit lock.
    save_field_text(SaveFieldText) -> Authorized<IssueView>;
}
