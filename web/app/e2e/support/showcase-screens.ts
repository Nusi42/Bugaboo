import {
  at,
  openCheckAccess,
  openInvitation,
  openPeople,
  openMovePreview,
  openQuickSearch,
  openTree,
  secondStep,
} from "./showcase-opens.ts";
import {
  openIssueDetails,
  openNewIssue,
  openNewIssueMinimized,
  openNewIssueWide,
} from "./showcase-issue-opens.ts";
import { ISSUE, ISSUE_PROJECT, PROJECT, type Who } from "./showcase-sample.ts";
import { type Device, type Screen } from "./showcase.ts";
import {
  addressInProject,
  groupAddress,
  issueAddress,
  paths,
  personAddress,
  projectAddress,
  projectIssuesAddress,
  roleAddress,
} from "../../src/shared/paths.ts";
import { issuePanelAddress } from "../../src/shared/search-address.ts";

/** One screen of the tour: its name in the file names, one plain sentence, who sees it, how to open it. */
function screen(
  slug: string,
  who: Who,
  title: string,
  open: Screen["open"],
  only?: Device,
): Screen {
  return only === undefined ? { slug, who, title, open } : { slug, who, title, only, open };
}

/** The screens of *My settings*, which an administrator opens like everybody else. */
const mine = (slug: string, title: string, address: string): Screen =>
  screen(slug, "administrator", `My settings: ${title}`, at(address));

/** The screens of *Organization settings*. */
const organization = (slug: string, title: string, open: Screen["open"]): Screen =>
  screen(slug, "administrator", `Organization settings: ${title}`, open);

/** The tabs of the settings of a project. */
const projectTab = (
  slug: string,
  title: string,
  address: Parameters<typeof addressInProject>[0],
): Screen =>
  screen(
    slug,
    "administrator",
    `Project settings: ${title}`,
    at(addressInProject(address, PROJECT)),
  );

/**
 * The screens, in the order of the tour. The number in the file names is the place in this list,
 * so a screen added here moves the numbers of those after it.
 */
export const screens: readonly Screen[] = [
  screen("sign-in", "nobody", "The sign-in page, where everybody starts", at(paths.signIn)),
  screen(
    "second-step",
    "nobody",
    "The second step of signing in, for a person with a code app",
    secondStep,
  ),
  screen("home", "administrator", "The start page of an administrator", at(paths.home)),
  screen(
    "project-tree",
    "administrator",
    "The tree of projects and sub-projects (on a phone, in the More sheet)",
    openTree,
  ),
  screen(
    "project-overview",
    "administrator",
    "The overview of one project",
    at(projectAddress(PROJECT)),
  ),
  projectTab("project-settings", "name and place", paths.projectSettings),
  projectTab("project-roles", "who has which role in this project", paths.projectRoles),
  projectTab("project-fields", "which fields each role sees in this project", paths.projectFields),
  projectTab("project-people", "which people each role sees in this project", paths.projectPeople),
  projectTab(
    "project-check-access",
    "check what a person can do in this project",
    paths.projectCheckAccess,
  ),
  screen(
    "project-move",
    "administrator",
    "Moving a project: the preview says who gains and who loses access",
    openMovePreview,
  ),
  screen("quick-search", "administrator", "Quick search, open over a page", openQuickSearch),
  organization(
    "organization-settings",
    "all the topics for an administrator",
    at(paths.organizationSettings),
  ),
  organization("projects", "all the projects, with the form that makes one", at(paths.projects)),
  organization("people", "everybody who has an account, and the open invitations", openPeople),
  organization("invitations", "making an invitation, with the link to send", openInvitation),
  organization(
    "person",
    "the page of one person, with their groups and roles",
    at((ids) => personAddress(ids.person)),
  ),
  organization("groups", "the groups", at(paths.groups)),
  organization(
    "group",
    "one group open in the side panel, with its members",
    at((ids) => groupAddress(ids.group)),
  ),
  organization(
    "roles",
    "the roles, and who has which role in the whole organization",
    at(paths.roles),
  ),
  organization(
    "role",
    "the page of one role and what it allows",
    at((ids) => roleAddress(ids.role)),
  ),
  organization("field-permissions", "which fields each role sees", at(paths.fieldPermissions)),
  organization("people-visibility", "which people each role sees", at(paths.peopleVisibility)),
  organization("check-access", "what one person can do, project by project", openCheckAccess),
  organization("sign-in-rules", "the rules for signing in", at(paths.signInRules)),
  organization("themes", "the look of the whole organization", at(paths.themes)),
  organization("server-status", "how the server is doing", at(paths.status)),
  mine("my-settings", "all the topics of the person themselves", paths.mySettings),
  mine("appearance", "the theme, size and spacing, with samples", paths.appearance),
  mine("region", "time zone, date and number formats", paths.region),
  mine("start-page", "the page that opens first", paths.startPage),
  mine("keyboard-shortcuts", "the keyboard shortcuts", paths.shortcuts),
  mine("security", "password, code app, passkeys and backup codes", paths.security),
  mine("sessions", "the places where the person is signed in", paths.sessions),
  mine("access-keys", "access keys for scripts", paths.accessKeys),
  screen(
    "project-issues",
    "administrator",
    "The issues of a project, with a box to tick for each",
    at(projectIssuesAddress(PROJECT)),
  ),
  screen("home-team-member", "teamMember", "The start page of a Team member", at(paths.home)),
  screen(
    "project-team-member",
    "teamMember",
    "A project as a Team member sees it",
    at(projectAddress(PROJECT)),
  ),
  screen(
    "no-access-team-member",
    "teamMember",
    "What a Team member finds when they open a page they may not use",
    at(paths.organizationSettings),
  ),
  screen("home-contributor", "contributor", "The start page of a Contributor", at(paths.home)),
  screen(
    "project-contributor",
    "contributor",
    "A project as a Contributor sees it",
    at(projectAddress(PROJECT)),
  ),
  screen("home-guest", "guest", "The start page of a Guest", at(paths.home)),
  screen("project-guest", "guest", "A project as a Guest sees it", at(projectAddress(PROJECT))),
  // The screens of issues (M3), after the others so the numbers of the pictures before them stay.
  screen(
    "issue",
    "administrator",
    "One issue in the focus view: its fields, description, parts, relations and history",
    at(issueAddress(ISSUE)),
  ),
  screen(
    "issue-panel",
    "administrator",
    "The issues of a project with one issue open in the side panel",
    at(issuePanelAddress(undefined, ISSUE_PROJECT, ISSUE)),
  ),
  screen(
    "new-issue",
    "administrator",
    "The new-issue form with its project, type, title and description",
    openNewIssue,
  ),
  organization("issue-types", "the kinds of issue", at(paths.issueTypes)),
  organization("fields", "the own fields of issues", at(paths.fields)),
  organization("relation-kinds", "the kinds of relation between issues", at(paths.relationKinds)),
  organization("labels", "the labels and tags of issues", at(paths.labels)),
  organization("layouts", "which fields the forms of issues show, and to whom", at(paths.layouts)),
  screen(
    "new-issue-wide",
    "administrator",
    "The new-issue window made wide: the title and the description in the middle, the fields at the side",
    openNewIssueWide,
    "computer",
  ),
  screen(
    "new-issue-minimized",
    "administrator",
    "The new-issue window parked as a bar at the bottom of the page, with its draft kept",
    openNewIssueMinimized,
    "computer",
  ),
  screen(
    "issue-details",
    "administrator",
    "The details of one issue opened under its status, as a phone shows them",
    openIssueDetails,
    "phone",
  ),
  screen(
    "issue-team-member",
    "teamMember",
    "One issue as a Team member sees it",
    at(issueAddress(ISSUE)),
  ),
];
