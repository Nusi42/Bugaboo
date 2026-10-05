//! Signing in (AUTH-01…03, AUTH-06, AUTH-09, SEC-12, SEC-14): the use cases.
//!
//! The use cases are plain `async fn`s that take the context [`Ctx`] and an input type
//! (`session.rs`, `second_factor.rs`, `account.rs`, `invitations.rs`). They depend on storage only
//! through the port [`AuthStore`]. [`AuthApi`] lists them once for the HTTP layer: it is what
//! `bb-api` holds, with the store behind it erased, so `bb-api` never names an adapter.

mod account;
mod ctx;
mod invitations;
mod second_factor;
mod session;
mod store;
mod types;

use std::future::Future;
use std::pin::Pin;

use bb_policy::Authorized;

use crate::people::{
    AddGroupMember, DeleteGroup, GroupList, GroupMembers, GroupMembersQuery, GroupView,
    GroupsQuery, MakeGroup, RemoveGroupMember, RenameGroup, add_group_member, delete_group,
    group_members, groups, make_group, remove_group_member, rename_group,
};
use crate::projects::{MakeProject, ProjectView, make_project};
use crate::roles::{
    AccessAnswer, AssignRole, AssignmentList, AssignmentView, AssignmentsQuery, ChangeRole,
    CheckAccessQuery, DeleteRole, Levels, LevelsQuery, MakeRole, RemoveAssignment, ResetRole,
    RoleList, RoleView, RolesQuery, SetFieldLevel, SetPeopleLevel, assign_role, assignments,
    change_role, check_access, delete_role, make_role, remove_assignment, reset_role, role_levels,
    roles, set_field_level, set_people_level,
};

pub use account::{accept_invitation, change_password, complete_setup, issue_setup_code, sign_up};
pub use ctx::{AppError, AuthServices, Ctx, FailureLimit, NETWORK_KEY_PREFIX, RequestInfo};
pub(crate) use ctx::{by, change_failed, changed, entity, fixed_entity, internal, set_to};
pub use invitations::{
    change_sign_up_policy, create_invitation, invitation_lookup, invitations, revoke_invitation,
    sign_up_policy,
};
pub use second_factor::{
    BACKUP_CODE_COUNT, MAX_PASSKEYS, add_passkey, confirm_code_app, passkey_registration_options,
    passkey_second_step_options, passkey_sign_in, passkey_sign_in_options, remove_code_app,
    remove_passkey, renew_backup_codes, start_code_app, two_step,
};
pub use session::{
    authenticate, end_session, passkey_second_step, second_step, sessions, sign_in, sign_out,
    sign_out_everywhere, status,
};
pub use store::{Account, AuthSettings, AuthStore, SecurityEvent, SessionRecord};
pub use types::*;

/// A future that can move between threads, which is what an HTTP handler needs to wait for.
pub type Answer<'a, T> = Pin<Box<dyn Future<Output = Result<T, AppError>> + Send + 'a>>;

/// Declares [`AuthApi`] and implements it for [`AuthServices`]: one line per use case, the
/// name of the function above, its input and its output.
macro_rules! use_cases {
    ($($(#[$meta:meta])* $name:ident($input:ty) -> $output:ty;)+) => {
        /// Every use case of [`AuthApi`] by name, with the name of its input type: what tests read to
        /// make sure none is left out (a use case whose input ends in `Query` only reads).
        pub const USE_CASES: &[(&str, &str)] = &[$((stringify!($name), stringify!($input))),+];

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
                Box::pin(async move { $name(&self.ctx(request), input).await })
            })+
        }
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
    /// Makes a role.
    make_role(MakeRole) -> Authorized<RoleView>;
    /// Changes a role.
    change_role(ChangeRole) -> Authorized<RoleView>;
    /// Deletes a role.
    delete_role(DeleteRole) -> Done;
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
    set_field_level(SetFieldLevel) -> Done;
    /// Sets or takes away how a role sees a kind of people information.
    set_people_level(SetPeopleLevel) -> Done;
    /// Lists the levels a role has set in a place.
    role_levels(LevelsQuery) -> Authorized<Levels>;
    /// Checks what a person can do in a place, and why.
    check_access(CheckAccessQuery) -> Authorized<AccessAnswer>;
    /// Makes a project.
    make_project(MakeProject) -> Authorized<ProjectView>;
}
