//! Signing in (AUTH-01…03, AUTH-06, AUTH-09, SEC-12, SEC-14): the use cases.
//!
//! The use cases are plain `async fn`s that take the context [`Ctx`] and an input type
//! (`session.rs`, `second_factor.rs`, `account.rs`, `invitations.rs`). They depend on storage only
//! through the port [`AuthStore`]. [`AuthApi`] lists them once for the HTTP layer: it is what
//! `bb-api` holds, with the store behind it erased, so `bb-api` never names an adapter.

#[macro_use]
mod use_cases_macro;

mod access_keys;
mod account;
mod api;
mod ctx;
mod error;
mod events;
mod invitations;
mod limit;
mod oauth;
mod oauth_actor;
mod oauth_admin;
mod oauth_rate;
mod oauth_session;
mod oauth_store;
mod oauth_token;
mod oauth_types;
mod request_session;
mod second_factor;
mod session;
mod sign_up;
mod store;
mod types;

use std::future::Future;
use std::pin::Pin;

use bb_policy::Authorized;

use crate::access::{
    FieldPermissions, FieldPermissionsQuery, OwnLoss, OwnLossQuery, PeopleToCheckPage,
    PeopleToCheckQuery, PeopleVisibility, PeopleVisibilityQuery, WhoMayChange, WhoMayChangeQuery,
    field_permissions, own_loss, people_to_check, people_visibility, who_may_change,
};
use crate::agent_work::{
    AgentWorkQuery, AnswerQuestion, ApprovePlan, AskQuestion, AttachResult, ChangesPage,
    ChangesSinceQuery, EndAttempt, HandOver, HandOverView, IssueAgentWork, LookAtWork,
    ReportAttempt, ReviewWork, TakeWork, WorkView, answer_question, approve_plan, ask_question,
    attach_result, changes_since, end_attempt, hand_over, issue_agent_work, look_at_work,
    report_attempt, review_work, take_work,
};
use crate::agent_work::{
    ChangeStandingRule, CreateStandingRule, DecideProposal, ProposalView, ProposeChange,
    StandingRulePage, StandingRuleView, StandingRulesQuery, change_standing_rule,
    create_standing_rule, decide_proposal, propose_change, standing_rules,
};
use crate::agent_work::{MyAgentWork, MyAgentWorkQuery, my_agent_work};
use crate::agents::{
    AcceptAgent, AgentPage, AgentQuery, AgentView, AgentsQuery, ChangeAgent, CreateAgent,
    HandAgent, RenameAgent, SetAgentLimits, StopAllAgents, TakeBackOffer, accept_agent, agent,
    change_agent, create_agent, hand_agent, my_agents, organization_agents, rename_agent,
    set_agent_limits, stop_all_agents, take_back_offer,
};
use crate::ai::assistant::{
    Applied, ApplyProposal, AskAssistantQuery, AssistantAnswer, AssistantStatus,
    AssistantStatusQuery, apply_proposal, ask_assistant, assistant_status,
};
use crate::ai::tools::{ToolAnswer, ToolCallQuery, ToolList, ToolListQuery, call_tool, list_tools};
use crate::ai::{
    AiSetup, AiSetupQuery, ChangeAiSetup, ChangeProjectAi, ConfirmAiAgreement, NeverSentList,
    NeverSentQuery, ProjectAi, ProjectAiQuery, SetAiKey, SetNeverSent, TryAiProvider,
    WhatGoesWhere, WhatGoesWhereQuery, ai_setup, change_ai_setup, change_project_ai,
    confirm_ai_agreement, never_sent_list, project_ai, set_ai_key, set_never_sent, try_ai_provider,
    what_goes_where,
};
use crate::comments::{
    AddComment, CommentVersions, CommentVersionsQuery, CommentView, DeleteComment, MentionChoices,
    MentionChoicesQuery, RestoreComment, SaveComment, add_comment, comment_versions,
    delete_comment, mention_choices, restore_comment, save_comment,
};
use crate::editing::{
    AskToFinish, CheckVersionsQuery, DraftSlot, DraftView, EditAnyway, HeartbeatAnswer,
    HeartbeatQuery, LockHolder, LockOutcome, ReadDraftQuery, ReadLockQuery, ReleaseLockQuery,
    ReleaseStuckLock, SaveDraftQuery, TakeLockQuery, ThrowAwayDraftQuery, Versions, ask_to_finish,
    check_versions, edit_anyway, heartbeat, read_draft, read_lock, release_lock,
    release_stuck_lock, save_draft, take_lock, throw_away_draft,
};
use crate::fields::{
    AddChoice, ChangeChoice, ChangeField, CombineChoices, CreateField, FieldPage, FieldQuery,
    FieldTypePage, FieldTypesQuery, FieldUsage, FieldView, FieldsQuery, ReorderChoices,
    SaveFieldText, add_choice, change_choice, change_field, combine_choices, create_field, field,
    field_types, field_usage, fields, reorder_choices, save_field_text,
};
use crate::files::{
    AttachmentView, CancelUploadQuery, DeleteAttachment, DownloadQuery, FileDownload, FileRules,
    FileRulesQuery, FinishUpload, IssueFiles, IssueFilesQuery, RestoreAttachment, SendPartQuery,
    StartUploadQuery, UploadStateQuery, UploadView, cancel_upload, delete_attachment, download,
    file_rules, finish_upload, issue_files, restore_attachment, send_part, start_upload,
    upload_state,
};
use crate::follow::{
    FollowIssue, Followers, FollowersQuery, FollowingQuery, FollowingState, UnfollowIssue,
    follow_issue, followers, following, unfollow_issue,
};
use crate::history::{HistoryPage, HistoryQuery, history};
use crate::issues::{
    AssignablePeoplePage, AssignablePeopleQuery, ChangeIssue, ChangeIssueType, CreateIssue,
    CreateIssueType, DeleteIssue, IssueDetail, IssueQuery, IssueTypePage, IssueTypeView,
    IssueTypesQuery, IssueView, IssuesQuery, MoveIssue, RestoreIssue, SaveDescription,
    assignable_people, change_issue, change_issue_type, create_issue, create_issue_type,
    delete_issue, issue, issue_types, issues, move_issue, restore_issue, save_description,
};
use crate::layouts::{
    CreateLayout, DeleteLayout, FormLayout, FormLayoutQuery, LayoutEditor, LayoutEditorQuery,
    SaveLayout, create_layout, delete_layout, form_layout, layout_editor, save_layout,
};
use crate::many_changes::{
    ApplyManyChanges, ManyChangePreview, ManyChangePreviewQuery, ManyChangeResult,
    ManyChangeRunQuery, ManyChangeRunStatus, ManyChangeRunView, apply_many_changes,
    many_changes_run, preview_many_changes, stop_many_changes_run,
};
use crate::notifications::{
    ChangeHandle, MailServerQuery, MailServerStatus, MyHandle, MyHandleQuery, NotificationPage,
    NotificationsQuery, ReadNotifications, SendTestEmail, SetMailPassword, Unread, UnreadQuery,
    change_handle, mail_server_status, my_handle, notifications, read_notifications,
    send_test_email, set_mail_password, unread_notifications,
};
use crate::offers::{ChangeOffers, OfferRowPage, Offers, OffersQuery, change_offers, offers};
use crate::people::{
    AddGroupMember, AssignmentChoicesQuery, ChoicePage, CreateGroup, DeleteGroup, GroupDetail,
    GroupList, GroupMembers, GroupMembersQuery, GroupQuery, GroupView, GroupsQuery,
    MemberChoicesQuery, PeopleQuery, PersonDetail, PersonPage, PersonQuery, RemoveGroupMember,
    RenameGroup, add_group_member, create_group, delete_group, group, group_members, groups,
    member_choices, people, person, remove_group_member, rename_group,
};
use crate::projects::{
    ChangeProject, CreateProject, MovePreview, MovePreviewQuery, MoveProject, ProjectDetail,
    ProjectPage, ProjectQuery, ProjectView, ProjectsQuery, change_project, create_project,
    move_project, preview_move, project, projects,
};
use crate::relations::{
    AddRelation, ChangeRelationKind, CreateRelationKind, IssueRelations, IssueRelationsQuery,
    RelatedIssuePage, RelatedIssuesQuery, RelationKindPage, RelationKindView, RelationKindsQuery,
    RelationView, RemoveRelation, SetParent, SubIssuePage, SubIssuesQuery, add_relation,
    change_relation_kind, create_relation_kind, issue_relations, related_issues, relation_kinds,
    remove_relation, set_parent, sub_issues,
};
use crate::roles::{
    AccessAnswer, AssignRole, AssignmentList, AssignmentView, AssignmentsQuery, ChangeRole,
    CheckAccessQuery, CopyRole, CreateRole, DeleteRole, Levels, LevelsQuery, PermissionCatalogue,
    PermissionsQuery, RemoveAssignment, ResetRole, RoleDetail, RoleList, RoleQuery, RoleView,
    RolesQuery, SetFieldLevel, SetPeopleLevel, assign_role, assignment_choices, assignments,
    change_role, check_access, copy_role, create_role, delete_role, permissions, remove_assignment,
    reset_role, role, role_levels, roles, set_level,
};
use crate::runs::{
    RunChangesPage, RunChangesQuery, RunQuery, RunView, UndoRun, UndoView, get_run,
    preview_undo_run, run_changes, undo_run,
};
use crate::saved_searches::{
    ChangeSearch, DeleteSearch, ProjectList, ProjectListQuery, RestoreSearch, RestoredSearch,
    SaveCopy, SaveSearch, SavedSearchPage, SavedSearchQuery, SavedSearchView, SearchesQuery,
    SetProjectList, ShareChoicePage, ShareChoicesQuery, ShareSearch, StopSharingSearch,
    change_search, delete_search, project_list, restore_search, save_copy, save_search,
    saved_search, saved_searches, set_project_list, share_choices, share_search,
    stop_sharing_search,
};
use crate::search::{
    NameChoices, NameChoicesQuery, ReadableFieldsQuery, SearchCount, SearchCountQuery,
    SearchFields, SearchGroups, SearchGroupsQuery, SearchPage, SearchQuery, SearcherQuery,
    name_choices, readable_fields, search, search_count, search_groups, searcher,
};
use crate::settings::{
    ChangeSetting, DocumentImagesQuery, ImageHosts, ImageHostsQuery, LockSetting, ResetSetting,
    SettingList, SettingView, SettingsQuery, change_setting, document_image_hosts, image_hosts,
    lock_setting, reset_setting, settings,
};
use crate::tags::{
    AddLabel, AddTag, ChangeLabel, ChangeLabelGroup, ChangeTag, CombineLabels, CombineTags,
    CreateLabel, CreateLabelGroup, IssueLabelsAndTags, IssueLabelsAndTagsQuery,
    LabelChangePreviewQuery, LabelGroupPage, LabelGroupView, LabelGroupsQuery, LabelPage,
    LabelView, LabelsQuery, RemoveLabel, RemoveTag, TagChangePreview, TagChangePreviewQuery,
    TagPage, TagSuggestions, TagSuggestionsQuery, TagView, TagsQuery, add_label, add_tag,
    change_label, change_label_group, change_tag, combine_labels, combine_tags, create_label,
    create_label_group, issue_labels_and_tags, label_change_preview, label_groups, labels,
    remove_label, remove_tag, tag_change_preview, tag_suggestions, tags,
};
use crate::time::totals::{
    IssueTime, IssueTimeQuery, ProjectTimeQuery, TimeTotals, issue_time, project_time,
};
use crate::time::{
    AddTime, ChangeStartTime, ChangeTimeEntry, DeleteTimeEntry, EntryDefaults, EntryDefaultsQuery,
    IssueTimeEntries, IssueTimeEntriesQuery, MyTimeEntriesQuery, StartTimer, StopTimer,
    SwitchTimer, ThrowAwayTimer, TimeEntryPage, TimeEntryView, TimerQuery, TimerSaved, TimerView,
    add_time, change_start_time, change_time_entry, delete_time_entry, issue_time_entries,
    my_time_entries, start_timer, stop_timer, switch_timer, throw_away_timer, time_entry_defaults,
    timer,
};
use crate::workflows::{
    CreateWorkflow, DeleteWorkflow, ProjectStatuses, ProjectStatusesQuery, PublishPreview,
    PublishPreviewQuery, PublishWorkflow, Published, Tried, TryWorkflowQuery, UseWorkflow,
    WorkflowEditor, WorkflowEditorQuery, WorkflowList, WorkflowsQuery, create_workflow,
    delete_workflow, project_statuses, publish_preview, publish_workflow, try_workflow,
    use_workflow, workflow_editor, workflows,
};
use crate::working_hours::{
    BookedWeek, BookedWeekQuery, ChangeWorkDay, ChangedWorkDay, DeleteOldWorkDays, DeletedWorkDays,
    OldWorkDays, OldWorkDaysQuery, PeopleOfHoursQuery, StartWorkDay, TapWorkDay, WorkDayHistory,
    WorkDayHistoryQuery, WorkDaysPage, WorkDaysQuery, booked_week, change_work_day,
    delete_old_work_days, old_work_days, people_of_hours, start_work_day, tap_work_day,
    work_day_history, work_days,
};

#[cfg(test)]
pub(crate) use access_keys::narrowing_of;
pub use access_keys::{
    access_key_rules, access_keys, create_access_key, delete_access_key, project_choices,
    require_session, see_server_status,
};
pub use account::{accept_invitation, change_password, complete_setup, issue_setup_code};
pub use api::{AuthApi, USE_CASES};
pub use ctx::{AuthServices, Ctx, RequestInfo, locked};
pub(crate) use ctx::{EventMaker, KnownProjects};
pub use error::AppError;
pub(crate) use error::{Stops, change_failed, if_there, internal, nothing_if_aborted};
pub(crate) use events::{by, changed, entity, fixed_entity, set_to};
pub use invitations::{
    change_sign_up_policy, create_invitation, invitation_lookup, invitations, revoke_invitation,
    sign_up_policy,
};
#[cfg(any(test, feature = "testing"))]
pub use limit::NoFailureLimit;
pub use limit::{FailureLimit, NETWORK_KEY_PREFIX};
pub use oauth::{authorization, connected_apps, decide_authorization, register_app, take_back};
pub use oauth_admin::clean_old_grants;
pub use oauth_admin::{
    add_web_address, mcp_identity, oauth_metadata, remove_web_address, web_addresses,
};
pub use oauth_session::{end_app_runs, start_app_session};
pub use oauth_store::{GrantRecord, GrantWithApp, OAuthCleaned, OAuthStore};
pub use oauth_token::exchange_token;
pub use oauth_types::*;
pub use second_factor::{
    BACKUP_CODE_COUNT, MAX_PASSKEYS, add_passkey, confirm_code_app, passkey_registration_options,
    passkey_second_step_options, passkey_sign_in, passkey_sign_in_options, remove_code_app,
    remove_passkey, renew_backup_codes, start_code_app, two_step,
};
pub(crate) use session::purge as purge_sessions;
pub use session::{
    authenticate, end_session, passkey_second_step, second_step, second_step_gap, sessions,
    sign_in, sign_out, sign_out_everywhere, status,
};
pub use sign_up::{SignUpStore, sign_up};
pub use store::{
    AccessKeyRecord, AccessKeyStore, Account, AccountStore, AuthSettings, AuthStore,
    InvitationStore, PeoplePosition, SearchIn, SecondStepStore, SecurityEvent, SessionRecord,
    SessionStore,
};
pub use types::*;

/// A future that can move between threads, which is what an HTTP handler needs to wait for.
pub type Answer<'a, T> = Pin<Box<dyn Future<Output = Result<T, AppError>> + Send + 'a>>;
