//! The list of the use cases of [`AuthApi`], once (`use_cases!`): one line per use case, the name of
//! the function, its input and its output. ARCH-EVENTS and the API read this list.

use super::{
    AcceptAgent, AcceptInvitation, AccessAnswer, AccessKeyPage, AccessKeyRules,
    AccessKeyRulesQuery, AccessKeysQuery, AddChoice, AddComment, AddGroupMember, AddLabel,
    AddPasskey, AddRelation, AddTag, AddTime, AddWebAddress, AgentPage, AgentQuery, AgentView,
    AgentWorkQuery, AgentsQuery, AiGuide, AiGuideQuery, AiSetup, AiSetupQuery, Answer,
    AnswerQuestion, AppError, AppSession, Applied, ApplyManyChanges, ApplyProposal, ApprovePlan,
    AskAssistantQuery, AskQuestion, AskToFinish, AssignRole, AssignablePeoplePage,
    AssignablePeopleQuery, AssignmentChoicesQuery, AssignmentList, AssignmentView,
    AssignmentsQuery, AssistantAnswer, AssistantStatus, AssistantStatusQuery, AttachResult,
    AttachmentView, AuthServices, AuthStatus, AuthStore, AuthenticateQuery, Authenticated,
    AuthorizationAnswer, AuthorizationQuery, AuthorizationView, Authorized, BackupCodes,
    BookedWeek, BookedWeekQuery, CancelUploadQuery, ChangeAgent, ChangeAiSetup, ChangeAwayTime,
    ChangeChoice, ChangeField, ChangeHandle, ChangeIssue, ChangeIssueType, ChangeLabel,
    ChangeLabelGroup, ChangeOffers, ChangePassword, ChangeProject, ChangeProjectAi,
    ChangeRelationKind, ChangeRole, ChangeSearch, ChangeSetting, ChangeSignUpPolicy,
    ChangeStandingRule, ChangeStartTime, ChangeTag, ChangeTimeEntry, ChangeUsualDays,
    ChangeWorkDay, ChangedAwayTime, ChangedShare, ChangedUsualDays, ChangedWorkDay, ChangesPage,
    ChangesSinceQuery, CheckAccessQuery, CheckVersionsQuery, ChoicePage, CodeAppSetup,
    CombineChoices, CombineLabels, CombineTags, CommentVersions, CommentVersionsQuery, CommentView,
    CompleteSetup, ConfirmAiAgreement, ConfirmCodeApp, ConnectedApps, ConnectedAppsQuery, CopyRole,
    CreateAccessKey, CreateAgent, CreateField, CreateGroup, CreateInvitation, CreateIssue,
    CreateIssueType, CreateLabel, CreateLabelGroup, CreateLayout, CreateProject,
    CreateRelationKind, CreateRole, CreateStandingRule, CreateWorkflow, CreatedAccessKey,
    CreatedInvitation, DatesMoved, DecideAuthorization, DecideProposal, DeleteAccessKey,
    DeleteAttachment, DeleteComment, DeleteGroup, DeleteIssue, DeleteLayout, DeleteOldWorkDays,
    DeleteRole, DeleteSearch, DeleteTimeEntry, DeleteWorkflow, DeletedWorkDays,
    DocumentImagesQuery, Done, DownloadQuery, DraftSlot, DraftView, EditAnyway, EndAppRuns,
    EndAttempt, EndSession, EnterAwayTime, EntryDefaults, EntryDefaultsQuery, FieldPage,
    FieldPermissions, FieldPermissionsQuery, FieldQuery, FieldTypePage, FieldTypesQuery,
    FieldUsage, FieldView, FieldsQuery, FileDownload, FileRules, FileRulesQuery, FinishUpload,
    FollowIssue, Followers, FollowersQuery, FollowingQuery, FollowingState, FormLayout,
    FormLayoutQuery, GroupDetail, GroupList, GroupMembers, GroupMembersQuery, GroupQuery,
    GroupView, GroupsQuery, HandAgent, HandOver, HandOverView, HeartbeatAnswer, HeartbeatQuery,
    HistoryPage, HistoryQuery, ImageHosts, ImageHostsQuery, InvitationLookup,
    InvitationLookupQuery, InvitationPage, InvitationsQuery, IssueAgentWork, IssueDetail,
    IssueFiles, IssueFilesQuery, IssueLabelsAndTags, IssueLabelsAndTagsQuery, IssueQuery,
    IssueRelations, IssueRelationsQuery, IssueTime, IssueTimeEntries, IssueTimeEntriesQuery,
    IssueTimeQuery, IssueTypePage, IssueTypeView, IssueTypesQuery, IssueView, IssuesQuery,
    LabelChangePreviewQuery, LabelGroupPage, LabelGroupView, LabelGroupsQuery, LabelPage,
    LabelView, LabelsQuery, LayoutEditor, LayoutEditorQuery, Levels, LevelsQuery, LockHolder,
    LockOutcome, LockSetting, LookAtWork, MailServerQuery, MailServerStatus, ManyChangePreview,
    ManyChangePreviewQuery, ManyChangeResult, ManyChangeRunQuery, ManyChangeRunStatus,
    ManyChangeRunView, McpIdentity, McpIdentityQuery, MemberChoicesQuery, MentionChoices,
    MentionChoicesQuery, MoveDates, MoveIssue, MovePreview, MovePreviewQuery, MoveProject,
    MyAgentWork, MyAgentWorkQuery, MyHandle, MyHandleQuery, MyTimeEntriesQuery, NameChoices,
    NameChoicesQuery, NeverSentList, NeverSentQuery, NotificationPage, NotificationsQuery,
    OAuthMetadata, OAuthMetadataQuery, OfferRowPage, Offers, OffersQuery, OldWorkDays,
    OldWorkDaysQuery, OwnLoss, OwnLossQuery, PasskeyChallenge, PasskeyRegistrationQuery,
    PasskeySecondStep, PasskeySecondStepQuery, PasskeySignIn, PasskeySignInQuery, PasskeyView,
    PeopleOfHoursQuery, PeopleQuery, PeopleToCheckPage, PeopleToCheckQuery, PeopleVisibility,
    PeopleVisibilityQuery, PermissionCatalogue, PermissionsQuery, PersonDetail, PersonPage,
    PersonQuery, ProjectAi, ProjectAiQuery, ProjectChoicePage, ProjectChoicesQuery, ProjectDetail,
    ProjectList, ProjectListQuery, ProjectPage, ProjectQuery, ProjectShares, ProjectSharesQuery,
    ProjectStatuses, ProjectStatusesQuery, ProjectTimeQuery, ProjectView, ProjectsQuery,
    ProposalView, ProposeChange, PublishPreview, PublishPreviewQuery, PublishWorkflow, Published,
    ReadDraftQuery, ReadLockQuery, ReadNotifications, ReadableFieldsQuery, RegisterApp,
    RegisterOutcome, RelatedIssuePage, RelatedIssuesQuery, RelationKindPage, RelationKindView,
    RelationKindsQuery, RelationView, ReleaseLockQuery, ReleaseStuckLock, RemoveAssignment,
    RemoveAwayTime, RemoveCodeApp, RemoveGroupMember, RemoveLabel, RemovePasskey, RemoveRelation,
    RemoveTag, RemoveWebAddress, RenameAgent, RenameGroup, RenewBackupCodes, ReorderChoices,
    ReportAttempt, RequestInfo, RequireSessionQuery, ResetRole, ResetSetting, RestoreAttachment,
    RestoreComment, RestoreIssue, RestoreSearch, RestoredSearch, ReviewWork, RevokeInvitation,
    RoleDetail, RoleList, RoleQuery, RoleView, RolesQuery, RunChangesPage, RunChangesQuery,
    RunQuery, RunView, SaveComment, SaveCopy, SaveDescription, SaveDraftQuery, SaveFieldText,
    SaveLayout, SaveSearch, SavedSearchPage, SavedSearchQuery, SavedSearchView, SearchCount,
    SearchCountQuery, SearchFields, SearchGroups, SearchGroupsQuery, SearchPage, SearchQuery,
    SearcherQuery, SearchesQuery, SecondStep, SecondStepGap, SecondStepGapQuery,
    SeeServerStatusQuery, SendPartQuery, SendTestEmail, SessionEnded, SessionList, SessionsQuery,
    SetAgentLimits, SetAiKey, SetFieldLevel, SetMailPassword, SetNeverSent, SetParent,
    SetPeopleLevel, SetProjectList, SetShare, SettingList, SettingView, SettingsQuery,
    ShareChoicePage, ShareChoicesQuery, ShareSearch, SignIn, SignOut, SignOutEverywhere, SignUp,
    SignUpPolicy, SignUpPolicyQuery, StandingRulePage, StandingRuleView, StandingRulesQuery,
    StartAppSession, StartCodeApp, StartTimer, StartUploadQuery, StartWorkDay, Started,
    StatusQuery, StopAllAgents, StopSharingSearch, StopTimer, SubIssuePage, SubIssuesQuery,
    SwitchTimer, TagChangePreview, TagChangePreviewQuery, TagPage, TagSuggestions,
    TagSuggestionsQuery, TagView, TagsQuery, TakeBack, TakeBackOffer, TakeLockQuery, TakeWork,
    TapWorkDay, ThrowAwayDraftQuery, ThrowAwayTimer, TimeEntryPage, TimeEntryView, TimeTotals,
    TimerQuery, TimerSaved, TimerView, TokenOutcome, TokenRequest, ToolAnswer, ToolCallQuery,
    ToolList, ToolListQuery, Tried, TryAiProvider, TryWorkflowQuery, TwoStepQuery, TwoStepStatus,
    UndoRun, UndoView, UnfollowIssue, Unread, UnreadQuery, UploadStateQuery, UploadView,
    UseWorkflow, Versions, WebAddresses, WebAddressesQuery, WhatGoesWhere, WhatGoesWhereQuery,
    WhoMayChange, WhoMayChangeQuery, WorkDayHistory, WorkDayHistoryQuery, WorkDaysPage,
    WorkDaysQuery, WorkTimesQuery, WorkTimesView, WorkView, WorkflowEditor, WorkflowEditorQuery,
    WorkflowList, WorkflowsQuery, accept_agent, accept_invitation, access_key_rules, access_keys,
    add_choice, add_comment, add_group_member, add_label, add_passkey, add_relation, add_tag,
    add_time, add_web_address, agent, ai_guide, ai_setup, answer_question, apply_many_changes,
    apply_proposal, approve_plan, ask_assistant, ask_question, ask_to_finish, assign_role,
    assignable_people, assignment_choices, assignments, assistant_status, attach_result,
    authenticate, authorization, booked_week, call_tool, cancel_upload, change_agent,
    change_ai_setup, change_away_time, change_choice, change_field, change_handle, change_issue,
    change_issue_type, change_label, change_label_group, change_offers, change_password,
    change_project, change_project_ai, change_relation_kind, change_role, change_search,
    change_setting, change_sign_up_policy, change_standing_rule, change_start_time, change_tag,
    change_time_entry, change_usual_days, change_work_day, changes_since, check_access,
    check_versions, combine_choices, combine_labels, combine_tags, comment_versions,
    complete_setup, confirm_ai_agreement, confirm_code_app, connected_apps, copy_role,
    create_access_key, create_agent, create_field, create_group, create_invitation, create_issue,
    create_issue_type, create_label, create_label_group, create_layout, create_project,
    create_relation_kind, create_role, create_standing_rule, create_workflow, decide_authorization,
    decide_proposal, delete_access_key, delete_attachment, delete_comment, delete_group,
    delete_issue, delete_layout, delete_old_work_days, delete_role, delete_search,
    delete_time_entry, delete_workflow, document_image_hosts, download, edit_anyway, end_app_runs,
    end_attempt, end_session, enter_away_time, exchange_token, field, field_permissions,
    field_types, field_usage, fields, file_rules, finish_upload, follow_issue, followers,
    following, form_layout, get_run, group, group_members, groups, hand_agent, hand_over,
    heartbeat, history, image_hosts, invitation_lookup, invitations, issue, issue_agent_work,
    issue_files, issue_labels_and_tags, issue_relations, issue_time, issue_time_entries,
    issue_types, issues, label_change_preview, label_groups, labels, layout_editor, list_tools,
    lock_setting, look_at_work, mail_server_status, many_changes_run, mcp_identity, member_choices,
    mention_choices, move_dates, move_issue, move_project, my_agent_work, my_agents, my_handle,
    my_time_entries, name_choices, never_sent_list, notifications, oauth_metadata, offers,
    old_work_days, organization_agents, own_loss, passkey_registration_options,
    passkey_second_step, passkey_second_step_options, passkey_sign_in, passkey_sign_in_options,
    people, people_of_hours, people_to_check, people_visibility, permissions, person,
    preview_many_changes, preview_move, preview_undo_run, project, project_ai, project_choices,
    project_list, project_shares, project_statuses, project_time, projects, propose_change,
    publish_preview, publish_workflow, read_draft, read_lock, read_notifications, readable_fields,
    register_app, related_issues, relation_kinds, release_lock, release_stuck_lock,
    remove_assignment, remove_away_time, remove_code_app, remove_group_member, remove_label,
    remove_passkey, remove_relation, remove_tag, remove_web_address, rename_agent, rename_group,
    renew_backup_codes, reorder_choices, report_attempt, require_session, reset_role,
    reset_setting, restore_attachment, restore_comment, restore_issue, restore_search, review_work,
    revoke_invitation, role, role_levels, roles, run_changes, save_comment, save_copy,
    save_description, save_draft, save_field_text, save_layout, save_search, saved_search,
    saved_searches, search, search_count, search_groups, searcher, second_step, second_step_gap,
    see_server_status, send_part, send_test_email, sessions, set_agent_limits, set_ai_key,
    set_level, set_mail_password, set_never_sent, set_parent, set_project_list, set_share,
    settings, share_choices, share_search, sign_in, sign_out, sign_out_everywhere, sign_up,
    sign_up_policy, standing_rules, start_app_session, start_code_app, start_timer, start_upload,
    start_work_day, status, stop_all_agents, stop_many_changes_run, stop_sharing_search,
    stop_timer, sub_issues, switch_timer, tag_change_preview, tag_suggestions, tags, take_back,
    take_back_offer, take_lock, take_work, tap_work_day, throw_away_draft, throw_away_timer,
    time_entry_defaults, timer, try_ai_provider, try_workflow, two_step, undo_run, unfollow_issue,
    unread_notifications, upload_state, use_workflow, web_addresses, what_goes_where,
    who_may_change, work_day_history, work_days, work_times, workflow_editor, workflows,
};

use crate::plan::{
    AcceptWarning, AcceptedWarningView, MoveAfterQuery, MovePreviewPage, SetTimeFactor,
    TimeFactorQuery, TimeFactorView, TimelinePage, TimelineQuery, accept_warning,
    move_after_preview, set_time_factor, time_factor, timeline,
};

use super::{
    CopyTheme, CreateTheme, DeleteTheme, SaveTheme, ThemeDetail, ThemeExport, ThemeList,
    ThemeQuery, ThemeSheet, ThemeSheetQuery, ThemeView, ThemesQuery, UploadTheme, copy_theme,
    create_theme, delete_theme, download_theme, save_theme, theme, theme_draft_sheet, theme_sheet,
    themes, upload_theme,
};

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
    /// Creates new backup codes.
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
    /// Creates an access key.
    create_access_key(CreateAccessKey) -> Authorized<CreatedAccessKey>;
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
    /// Creates an invitation.
    create_invitation(CreateInvitation) -> Authorized<CreatedInvitation>;
    /// Takes an invitation back.
    revoke_invitation(RevokeInvitation) -> Done;
    /// Lists the open invitations.
    invitations(InvitationsQuery) -> Authorized<InvitationPage>;
    /// Shows which address an invitation is for.
    invitation_lookup(InvitationLookupQuery) -> InvitationLookup;
    /// Creates an account from an invitation.
    accept_invitation(AcceptInvitation) -> Started;
    /// Asks for an account without an invitation: a link goes to the address.
    sign_up(SignUp) -> Done;
    /// Who may sign up on their own.
    sign_up_policy(SignUpPolicyQuery) -> Authorized<SignUpPolicy>;
    /// Changes who may sign up on their own.
    change_sign_up_policy(ChangeSignUpPolicy) -> Authorized<SignUpPolicy>;
    /// Creates the first administrator with the setup code.
    complete_setup(CompleteSetup) -> Started;
    /// Creates a group.
    create_group(CreateGroup) -> Authorized<GroupView>;
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
    /// Creates a role.
    create_role(CreateRole) -> Authorized<RoleView>;
    /// Changes a role.
    change_role(ChangeRole) -> Authorized<RoleView>;
    /// Deletes a role.
    delete_role(DeleteRole) -> Done;
    /// Creates a role from a copy of another.
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
    /// Creates a project.
    create_project(CreateProject) -> Authorized<ProjectView>;
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
    /// Creates an issue type.
    create_issue_type(CreateIssueType) -> Authorized<IssueTypeView>;
    /// Changes the name, icon or retired state of an issue type.
    change_issue_type(ChangeIssueType) -> Authorized<IssueTypeView>;
    /// Shows where an issue type or a group of labels is offered.
    offers(OffersQuery) -> Authorized<OfferRowPage>;
    /// Changes where an issue type or a group of labels is offered.
    change_offers(ChangeOffers) -> Authorized<Offers>;
    /// The guide to AI for everybody, as HTML.
    ai_guide(AiGuideQuery) -> Authorized<AiGuide>;
    /// Lists the built-in fields and the projects marked *Never sent to AI*.
    never_sent_list(NeverSentQuery) -> Authorized<NeverSentList>;
    /// Marks a built-in field or a project *Never sent to AI*, or takes the mark away.
    set_never_sent(SetNeverSent) -> Authorized<NeverSentList>;
    /// Lists the tools a caller may use.
    list_tools(ToolListQuery) -> Authorized<ToolList>;
    /// Calls a tool as the person behind the session or key.
    call_tool(ToolCallQuery) -> Authorized<ToolAnswer>;
    /// Asks the assistant a question; it answers and proposes changes, and saves nothing.
    ask_assistant(AskAssistantQuery) -> Authorized<AssistantAnswer>;
    /// Whether the assistant is there at a place, and where its answers come from.
    assistant_status(AssistantStatusQuery) -> Authorized<AssistantStatus>;
    /// Saves a change the assistant proposed, as the person.
    apply_proposal(ApplyProposal) -> Authorized<Applied>;
    /// Lists the issues of a project a person sees.
    issues(IssuesQuery) -> Authorized<SearchPage>;
    /// A page of the issues a search finds, in the projects the person sees.
    search(SearchQuery) -> Authorized<SearchPage>;
    /// How many issues a search finds.
    search_count(SearchCountQuery) -> Authorized<SearchCount>;
    /// How many issues a search finds in each group of a field.
    search_groups(SearchGroupsQuery) -> Authorized<SearchGroups>;
    /// The person who searches, by a session or an access key.
    searcher(SearcherQuery) -> bb_core::id::UserId;
    /// The fields the person may search by, where a search looks.
    readable_fields(ReadableFieldsQuery) -> Authorized<SearchFields>;
    /// The names of people or groups the person may use in a search, starting with some words.
    name_choices(NameChoicesQuery) -> Authorized<NameChoices>;
    /// Lists the people an issue of a project can be assigned to.
    assignable_people(AssignablePeopleQuery) -> Authorized<AssignablePeoplePage>;
    /// Creates an issue.
    create_issue(CreateIssue) -> Authorized<IssueView>;
    /// Shows one issue by its readable or internal ID.
    issue(IssueQuery) -> Authorized<IssueDetail>;
    /// Changes the fields of an issue.
    change_issue(ChangeIssue) -> Authorized<IssueView>;
    /// Moves the dates of several issues in one run.
    move_dates(MoveDates) -> Authorized<DatesMoved>;
    /// A page of the timeline: rows with their bars, dependencies and warnings.
    timeline(TimelineQuery) -> Authorized<TimelinePage>;
    /// The preview of *Move it after*.
    move_after_preview(MoveAfterQuery) -> Authorized<MovePreviewPage>;
    /// *Keep and accept* a plan warning.
    accept_warning(AcceptWarning) -> Authorized<AcceptedWarningView>;
    /// The time factor of an issue.
    time_factor(TimeFactorQuery) -> Authorized<TimeFactorView>;
    /// Sets the time factor of an issue.
    set_time_factor(SetTimeFactor) -> Authorized<TimeFactorView>;
    /// The hosts images in text may come from, for a signed-in person.
    image_hosts(ImageHostsQuery) -> Authorized<ImageHosts>;
    /// The same for the document, before anybody signs in.
    document_image_hosts(DocumentImagesQuery) -> ImageHosts;
    /// Saves the description of an issue under the edit lock the person holds.
    save_description(SaveDescription) -> Authorized<IssueView>;
    /// Puts an issue into the trash.
    delete_issue(DeleteIssue) -> Done;
    /// Moves an issue into another project.
    move_issue(MoveIssue) -> Authorized<IssueView>;
    /// Takes an issue out of the trash.
    restore_issue(RestoreIssue) -> Authorized<IssueView>;
    /// Lists the kinds of relation.
    relation_kinds(RelationKindsQuery) -> Authorized<RelationKindPage>;
    /// Creates a kind of relation.
    create_relation_kind(CreateRelationKind) -> Authorized<RelationKindView>;
    /// Changes the names or the retired state of a kind of relation.
    change_relation_kind(ChangeRelationKind) -> Authorized<RelationKindView>;
    /// Shows the sub-issues and the related issues of an issue.
    issue_relations(IssueRelationsQuery) -> Authorized<IssueRelations>;
    /// Lists the sub-issues of an issue that the person may see.
    sub_issues(SubIssuesQuery) -> Authorized<SubIssuePage>;
    /// Lists the issues related to an issue whose other end the person may see.
    related_issues(RelatedIssuesQuery) -> Authorized<RelatedIssuePage>;
    /// Puts an issue under another one, or takes it out.
    set_parent(SetParent) -> Authorized<IssueRelations>;
    /// Adds a relation between two issues.
    add_relation(AddRelation) -> Authorized<RelationView>;
    /// Takes a relation away.
    remove_relation(RemoveRelation) -> Done;
    /// Writes a comment under an issue.
    add_comment(AddComment) -> Authorized<CommentView>;
    /// Saves the new text of a comment under its lock.
    save_comment(SaveComment) -> Authorized<CommentView>;
    /// Puts a comment into the trash.
    delete_comment(DeleteComment) -> Done;
    /// Takes a comment out of the trash.
    restore_comment(RestoreComment) -> Authorized<CommentView>;
    /// Shows every text a comment had.
    comment_versions(CommentVersionsQuery) -> Authorized<CommentVersions>;
    /// Lists what a writer may name after @.
    mention_choices(MentionChoicesQuery) -> Authorized<MentionChoices>;
    /// Lists the notifications of the person.
    notifications(NotificationsQuery) -> Authorized<NotificationPage>;
    /// How many notifications the person has not read.
    unread_notifications(UnreadQuery) -> Authorized<Unread>;
    /// The handle of the person.
    my_handle(MyHandleQuery) -> Authorized<MyHandle>;
    /// Changes the handle of the person.
    change_handle(ChangeHandle) -> Authorized<MyHandle>;
    /// Reads the setup of AI of the organization.
    ai_setup(AiSetupQuery) -> Authorized<AiSetup>;
    /// Changes the settings of AI of the organization.
    change_ai_setup(ChangeAiSetup) -> Authorized<AiSetup>;
    /// Enters, changes or removes the key of the AI provider.
    set_ai_key(SetAiKey) -> Authorized<Done>;
    /// Confirms that a data processing agreement with the AI provider is in place.
    confirm_ai_agreement(ConfirmAiAgreement) -> Authorized<AiSetup>;
    /// Tells what goes where when AI is on.
    what_goes_where(WhatGoesWhereQuery) -> Authorized<WhatGoesWhere>;
    /// Tries the AI provider with one short call.
    try_ai_provider(TryAiProvider) -> Authorized<Done>;
    /// Reads the switch of AI of one project.
    project_ai(ProjectAiQuery) -> Authorized<ProjectAi>;
    /// Switches AI on or off for one project.
    change_project_ai(ChangeProjectAi) -> Authorized<ProjectAi>;
    /// Enters, changes or removes the password of the mail server.
    set_mail_password(SetMailPassword) -> Authorized<Done>;
    /// Tells whether the password of the mail server is entered.
    mail_server_status(MailServerQuery) -> Authorized<MailServerStatus>;
    /// Sends a test e-mail to the person who asks.
    send_test_email(SendTestEmail) -> Authorized<Done>;
    /// Marks notifications of the person as read.
    read_notifications(ReadNotifications) -> Done;
    /// Starts an upload of a file to an issue or a comment.
    start_upload(StartUploadQuery) -> Authorized<UploadView>;
    /// Shows how far an upload is, so an interrupted one goes on.
    upload_state(UploadStateQuery) -> Authorized<UploadView>;
    /// Sends the next part of an upload.
    send_part(SendPartQuery) -> Authorized<UploadView>;
    /// Throws an unfinished upload away.
    cancel_upload(CancelUploadQuery) -> Done;
    /// Finishes an upload: the file becomes an attachment.
    finish_upload(FinishUpload) -> Authorized<AttachmentView>;
    /// Lists the files of an issue.
    issue_files(IssueFilesQuery) -> Authorized<IssueFiles>;
    /// Says what the person may add to an issue and the limits of its project.
    file_rules(FileRulesQuery) -> Authorized<FileRules>;
    /// Puts an attachment into the trash.
    delete_attachment(DeleteAttachment) -> Done;
    /// Takes an attachment out of the trash.
    restore_attachment(RestoreAttachment) -> Authorized<AttachmentView>;
    /// Opens a file or its preview through a signed link.
    download(DownloadQuery) -> FileDownload;
    /// Shows the history of an issue.
    history(HistoryQuery) -> Authorized<HistoryPage>;
    /// Follows an issue.
    follow_issue(FollowIssue) -> Done;
    /// Stops following an issue.
    unfollow_issue(UnfollowIssue) -> Done;
    /// Lists who follows an issue.
    followers(FollowersQuery) -> Authorized<Followers>;
    /// Whether the person follows an issue.
    following(FollowingQuery) -> Authorized<FollowingState>;
    /// The work days of a stretch of dates: one's own or another person's.
    work_days(WorkDaysQuery) -> Authorized<WorkDaysPage>;
    /// A person's work times: usual work days, work hours, away times and the choice of holidays.
    work_times(WorkTimesQuery) -> Authorized<WorkTimesView>;
    /// New usual work days and work hours of a person.
    change_usual_days(ChangeUsualDays) -> Authorized<ChangedUsualDays>;
    /// Enters an away time.
    enter_away_time(EnterAwayTime) -> Authorized<ChangedAwayTime>;
    /// Changes an away time.
    change_away_time(ChangeAwayTime) -> Authorized<ChangedAwayTime>;
    /// Removes an away time.
    remove_away_time(RemoveAwayTime) -> Done;
    /// Sets the share of a person in a project.
    set_share(SetShare) -> Authorized<ChangedShare>;
    /// The people of a project with their shares there.
    project_shares(ProjectSharesQuery) -> Authorized<ProjectShares>;
    /// *Start work day*.
    start_work_day(StartWorkDay) -> Authorized<ChangedWorkDay>;
    /// *Start break*, *End break* or *End work day*.
    tap_work_day(TapWorkDay) -> Authorized<ChangedWorkDay>;
    /// New times of a work day, one's own or another person's.
    change_work_day(ChangeWorkDay) -> Authorized<ChangedWorkDay>;
    /// The history of a work day.
    work_day_history(WorkDayHistoryQuery) -> Authorized<WorkDayHistory>;
    /// One's own week compared with the time booked on issues.
    booked_week(BookedWeekQuery) -> Authorized<BookedWeek>;
    /// The people whose working hours the reader may open, to choose one from.
    people_of_hours(PeopleOfHoursQuery) -> Authorized<ChoicePage>;
    /// How many work days are older than the keep period.
    old_work_days(OldWorkDaysQuery) -> Authorized<OldWorkDays>;
    /// Deletes the work days older than the keep period.
    delete_old_work_days(DeleteOldWorkDays) -> Authorized<DeletedWorkDays>;
    /// Lists the saved searches a person made, or the ones shared with them.
    saved_searches(SearchesQuery) -> Authorized<SavedSearchPage>;
    /// Opens one saved search.
    saved_search(SavedSearchQuery) -> Authorized<SavedSearchView>;
    /// Lists the groups or projects a person may share a saved search with.
    share_choices(ShareChoicesQuery) -> Authorized<ShareChoicePage>;
    /// Saves a search under a name.
    save_search(SaveSearch) -> Authorized<SavedSearchView>;
    /// Changes a saved search.
    change_search(ChangeSearch) -> Authorized<SavedSearchView>;
    /// Shares a saved search with a group or a project.
    share_search(ShareSearch) -> Authorized<SavedSearchView>;
    /// Stops sharing a saved search with a group or a project.
    stop_sharing_search(StopSharingSearch) -> Authorized<SavedSearchView>;
    /// Puts a saved search into the trash.
    delete_search(DeleteSearch) -> Done;
    /// Takes a saved search out of the trash.
    restore_search(RestoreSearch) -> Authorized<RestoredSearch>;
    /// Saves a copy of a saved search for the person.
    save_copy(SaveCopy) -> Authorized<SavedSearchView>;
    /// Shows the list a project shows.
    project_list(ProjectListQuery) -> Authorized<ProjectList>;
    /// Chooses the saved search a project shows as its list.
    set_project_list(SetProjectList) -> Authorized<ProjectList>;
    /// Lists the own fields a person sees.
    fields(FieldsQuery) -> Authorized<FieldPage>;
    /// Lists the kinds of information a field can hold.
    field_types(FieldTypesQuery) -> Authorized<FieldTypePage>;
    /// Shows one own field.
    field(FieldQuery) -> Authorized<FieldView>;
    /// Says where an own field is used.
    field_usage(FieldQuery) -> Authorized<FieldUsage>;
    /// Creates an own field.
    create_field(CreateField) -> Authorized<FieldView>;
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
    /// Says what a change to many issues would do to each issue.
    preview_many_changes(ManyChangePreviewQuery) -> Authorized<ManyChangePreview>;
    /// Changes many issues at once after the preview.
    apply_many_changes(ApplyManyChanges) -> Authorized<ManyChangeResult>;
    /// Shows how far a change to many issues that goes on in the background is.
    many_changes_run(ManyChangeRunQuery) -> Authorized<ManyChangeRunView>;
    /// Stops a change to many issues that goes on in the background.
    stop_many_changes_run(ManyChangeRunQuery) -> Authorized<ManyChangeRunStatus>;
    /// Shows a run of an AI app, an agent or a change to many issues.
    get_run(RunQuery) -> Authorized<RunView>;
    /// Lists the changes of a run the reader may see, a page at a time.
    run_changes(RunChangesQuery) -> Authorized<RunChangesPage>;
    /// Says what undoing a run would do.
    preview_undo_run(RunQuery) -> Authorized<UndoView>;
    /// Undoes a run in one step.
    undo_run(UndoRun) -> Authorized<UndoView>;
    /// Shows the tags and labels of an issue.
    issue_labels_and_tags(IssueLabelsAndTagsQuery) -> Authorized<IssueLabelsAndTags>;
    /// Puts a tag on an issue by its name.
    add_tag(AddTag) -> Authorized<IssueLabelsAndTags>;
    /// Takes a tag off an issue.
    remove_tag(RemoveTag) -> Authorized<IssueLabelsAndTags>;
    /// Puts a label on an issue.
    add_label(AddLabel) -> Authorized<IssueLabelsAndTags>;
    /// Takes a label off an issue.
    remove_label(RemoveLabel) -> Authorized<IssueLabelsAndTags>;
    /// Lists the tags to offer while a person types.
    tag_suggestions(TagSuggestionsQuery) -> Authorized<TagSuggestions>;
    /// Lists the tags of the organization for the people who look after them.
    tags(TagsQuery) -> Authorized<TagPage>;
    /// Says what renaming or combining a tag would do.
    tag_change_preview(TagChangePreviewQuery) -> Authorized<TagChangePreview>;
    /// Changes the name, color or projects of a tag.
    change_tag(ChangeTag) -> Authorized<TagView>;
    /// Moves every issue from a tag to another and takes the first away.
    combine_tags(CombineTags) -> Authorized<TagView>;
    /// Lists the labels of the organization a page at a time.
    labels(LabelsQuery) -> Authorized<LabelPage>;
    /// Lists the groups of labels of the organization a page at a time.
    label_groups(LabelGroupsQuery) -> Authorized<LabelGroupPage>;
    /// Creates a group of labels.
    create_label_group(CreateLabelGroup) -> Authorized<LabelGroupView>;
    /// Changes a group of labels.
    change_label_group(ChangeLabelGroup) -> Authorized<LabelGroupView>;
    /// Creates a label.
    create_label(CreateLabel) -> Authorized<LabelView>;
    /// Changes a label.
    change_label(ChangeLabel) -> Authorized<LabelView>;
    /// Says what renaming or combining a label would do.
    label_change_preview(LabelChangePreviewQuery) -> Authorized<TagChangePreview>;
    /// Moves every issue from a label to another and takes the first away.
    combine_labels(CombineLabels) -> Authorized<LabelView>;
    /// Shows which fields a form shows to the person who asks.
    form_layout(FormLayoutQuery) -> Authorized<FormLayout>;
    /// Shows the layout of a place to somebody who may change it.
    layout_editor(LayoutEditorQuery) -> Authorized<LayoutEditor>;
    /// Creates the layout of a place as a copy of the one that counts there.
    create_layout(CreateLayout) -> Authorized<LayoutEditor>;
    /// Saves the sections and fields of a layout, under its edit lock.
    save_layout(SaveLayout) -> Authorized<LayoutEditor>;
    /// Takes the layout of a place away.
    delete_layout(DeleteLayout) -> Done;
    /// Lists the own themes the person may use.
    themes(ThemesQuery) -> Authorized<ThemeList>;
    /// Shows one own theme with the pairs that are hard to read.
    theme(ThemeQuery) -> Authorized<ThemeDetail>;
    /// Gives an own theme as a file.
    download_theme(ThemeQuery) -> Authorized<ThemeExport>;
    /// Gives the style sheet of an own theme.
    theme_sheet(ThemeSheetQuery) -> ThemeSheet;
    /// Gives the style sheet of the person's own draft of a theme.
    theme_draft_sheet(ThemeSheetQuery) -> ThemeSheet;
    /// Makes a new own theme.
    create_theme(CreateTheme) -> Authorized<ThemeView>;
    /// Saves an own theme under its edit lock.
    save_theme(SaveTheme) -> Authorized<ThemeDetail>;
    /// Keeps an own theme as a new one.
    copy_theme(CopyTheme) -> Authorized<ThemeDetail>;
    /// Makes an own theme from a file.
    upload_theme(UploadTheme) -> Authorized<ThemeDetail>;
    /// Takes an own theme away.
    delete_theme(DeleteTheme) -> Done;
    /// Lists the statuses the issues of a project can have.
    project_statuses(ProjectStatusesQuery) -> Authorized<ProjectStatuses>;
    /// Shows the workflow that counts at a place to somebody who may change it.
    workflow_editor(WorkflowEditorQuery) -> Authorized<WorkflowEditor>;
    /// Lists the workflows there are, for choosing one for a place.
    workflows(WorkflowsQuery) -> Authorized<WorkflowList>;
    /// Creates the workflow of a place as a copy of the one that counts there.
    create_workflow(CreateWorkflow) -> Authorized<WorkflowEditor>;
    /// Says which workflow a place uses.
    use_workflow(UseWorkflow) -> Authorized<WorkflowEditor>;
    /// Publishes a workflow under its edit lock.
    publish_workflow(PublishWorkflow) -> Authorized<Published>;
    /// Says which statuses publishing a workflow would drop, and which issues are in them.
    publish_preview(PublishPreviewQuery) -> Authorized<PublishPreview>;
    /// Takes a workflow away.
    delete_workflow(DeleteWorkflow) -> Done;
    /// Tells which status changes a person would get on an issue.
    try_workflow(TryWorkflowQuery) -> Authorized<Tried>;
    /// Tells which roles may change a field in the issues of a project.
    who_may_change(WhoMayChangeQuery) -> Authorized<WhoMayChange>;
    /// Makes an agent for the person.
    create_agent(CreateAgent) -> Authorized<AgentView>;
    /// Lists the person's own agents.
    my_agents(AgentsQuery) -> Authorized<AgentPage>;
    /// Lists every agent of the organization, for an administrator.
    organization_agents(AgentsQuery) -> Authorized<AgentPage>;
    /// Shows one agent.
    agent(AgentQuery) -> Authorized<AgentView>;
    /// Gives an agent another name.
    rename_agent(RenameAgent) -> Authorized<AgentView>;
    /// Gives an agent other limits.
    set_agent_limits(SetAgentLimits) -> Authorized<AgentView>;
    /// Pauses, resumes or stops an agent.
    change_agent(ChangeAgent) -> Authorized<AgentView>;
    /// Stops every agent of the person.
    stop_all_agents(StopAllAgents) -> Done;
    /// Offers an agent to another person.
    hand_agent(HandAgent) -> Done;
    /// Accepts an agent that was handed over.
    accept_agent(AcceptAgent) -> Done;
    /// Takes back the offer of an agent, or declines it.
    take_back_offer(TakeBackOffer) -> Done;
    /// Opens the run of a session of an AI app.
    start_app_session(StartAppSession) -> AppSession;
    /// Closes the runs of sessions of AI apps that ended or were idle too long.
    end_app_runs(EndAppRuns) -> Done;
    /// Says where the sign-in of AI apps is.
    oauth_metadata(OAuthMetadataQuery) -> OAuthMetadata;
    /// Says who a call to the MCP server is from.
    mcp_identity(McpIdentityQuery) -> McpIdentity;
    /// Registers an AI app that comes back to this computer, or to a web address an administrator
    /// allowed.
    register_app(RegisterApp) -> RegisterOutcome;
    /// Says what an AI app asks, for the page that asks the person.
    authorization(AuthorizationQuery) -> Authorized<AuthorizationView>;
    /// The person says *Allow* or *Cancel* to an AI app.
    decide_authorization(DecideAuthorization) -> Authorized<AuthorizationAnswer>;
    /// Swaps a code or a renewing token of an AI app for tokens.
    exchange_token(TokenRequest) -> TokenOutcome;
    /// Lists the AI apps the person allowed.
    connected_apps(ConnectedAppsQuery) -> Authorized<ConnectedApps>;
    /// Ends what the person allowed an AI app.
    take_back(TakeBack) -> Done;
    /// Lists the web addresses AI apps may come back to.
    web_addresses(WebAddressesQuery) -> Authorized<WebAddresses>;
    /// Allows a web address for AI apps.
    add_web_address(AddWebAddress) -> Done;
    /// Takes a web address away.
    remove_web_address(RemoveWebAddress) -> Done;
    /// Hands issues over to one of the person's agents.
    hand_over(HandOver) -> Authorized<HandOverView>;
    /// Lists the current work of the person's own agents.
    my_agent_work(MyAgentWorkQuery) -> Authorized<MyAgentWork>;
    /// Shows the agent work on an issue.
    issue_agent_work(AgentWorkQuery) -> Authorized<IssueAgentWork>;
    /// An agent takes an issue that was handed over to it and starts an attempt.
    take_work(TakeWork) -> Authorized<WorkView>;
    /// An agent asks what changed in the issues it may see since a mark.
    changes_since(ChangesSinceQuery) -> Authorized<ChangesPage>;
    /// An agent reports its attempt: the plan, the time and the cost it reports.
    report_attempt(ReportAttempt) -> Authorized<WorkView>;
    /// An agent asks a question and waits for the answer.
    ask_question(AskQuestion) -> Authorized<WorkView>;
    /// An agent attaches a link or a check to its attempt.
    attach_result(AttachResult) -> Authorized<WorkView>;
    /// An agent says its attempt is ready, or gives up.
    end_attempt(EndAttempt) -> Authorized<WorkView>;
    /// Answers the question of an agent.
    answer_question(AnswerQuestion) -> Authorized<WorkView>;
    /// Approves the plan of an agent, or answers it with a note.
    approve_plan(ApprovePlan) -> Authorized<WorkView>;
    /// Accepts the result of an agent or sends it back.
    review_work(ReviewWork) -> Authorized<WorkView>;
    /// Marks work that was accepted by itself as looked at.
    look_at_work(LookAtWork) -> Authorized<WorkView>;
    /// Makes a standing rule for one of the person's agents.
    create_standing_rule(CreateStandingRule) -> Authorized<StandingRuleView>;
    /// Renews or stops a standing rule.
    change_standing_rule(ChangeStandingRule) -> Authorized<StandingRuleView>;
    /// Lists the standing rules of one agent.
    standing_rules(StandingRulesQuery) -> Authorized<StandingRulePage>;
    /// An agent proposes a change under a standing rule.
    propose_change(ProposeChange) -> Authorized<ProposalView>;
    /// Applies or discards a proposal of a standing rule.
    decide_proposal(DecideProposal) -> Authorized<ProposalView>;
    /// Shows the timer of the person.
    timer(TimerQuery) -> Authorized<TimerView>;
    /// Starts the timer of the person on an issue.
    start_timer(StartTimer) -> Authorized<TimerView>;
    /// Stops the timer and saves the time as an entry.
    stop_timer(StopTimer) -> Authorized<TimerSaved>;
    /// Saves the time so far and starts the timer on another issue.
    switch_timer(SwitchTimer) -> Authorized<TimerSaved>;
    /// Moves the start of the running timer.
    change_start_time(ChangeStartTime) -> Authorized<TimerView>;
    /// Stops the timer and saves nothing.
    throw_away_timer(ThrowAwayTimer) -> Authorized<TimerView>;
    /// Books time on an issue by hand.
    add_time(AddTime) -> Authorized<TimeEntryView>;
    /// Changes one of the person's own time entries.
    change_time_entry(ChangeTimeEntry) -> Authorized<TimeEntryView>;
    /// Deletes one of the person's own time entries.
    delete_time_entry(DeleteTimeEntry) -> Done;
    /// Lists the person's own time entries of some dates.
    my_time_entries(MyTimeEntriesQuery) -> Authorized<TimeEntryPage>;
    /// Lists the time entries on an issue.
    issue_time_entries(IssueTimeEntriesQuery) -> Authorized<IssueTimeEntries>;
    /// Tells what a new time entry on an issue starts with.
    time_entry_defaults(EntryDefaultsQuery) -> Authorized<EntryDefaults>;
    /// Shows the time of an issue, and its totals with its sub-issues.
    issue_time(IssueTimeQuery) -> Authorized<IssueTime>;
    /// Shows the totals of the time of a project.
    project_time(ProjectTimeQuery) -> Authorized<TimeTotals>;
}
