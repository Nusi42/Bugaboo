import { appPaths } from "@bugaboo/api-client";

/**
 * The addresses of the pages of the app, written once. Routes, links and redirects all use these,
 * so an address cannot differ between the page and the link to it (ODR). They are the addresses of
 * the UX design (§2.1): `/me/…` for *My settings*, `/settings/…` for *Organization settings*.
 */
export const paths = {
  home: "/",
  notifications: appPaths.notifications,
  setup: "/setup",
  signIn: appPaths.signIn,
  secondStep: "/sign-in/second-step",
  setUpSecondStep: "/sign-in/set-up-second-step",
  signUp: "/sign-up",
  invitation: appPaths.invitation,
  mySettings: "/me",
  appearance: "/me/appearance",
  region: "/me/region",
  notificationSettings: "/me/notifications",
  handle: "/me/handle",
  startPage: "/me/start-page",
  shortcuts: "/me/shortcuts",
  security: "/me/security",
  sessions: "/me/sessions",
  accessKeys: "/me/access-keys",
  organizationSettings: "/settings",
  people: "/settings/people",
  person: "/settings/people/$person",
  groups: "/settings/groups",
  issueTypes: "/settings/issue-types",
  layouts: "/settings/layouts",
  workflows: "/settings/workflows",
  relationKinds: "/settings/relation-kinds",
  labels: "/settings/labels",
  fields: "/settings/fields",
  field: "/settings/fields/$field",
  roles: "/settings/roles",
  role: "/settings/roles/$role",
  signInRules: "/settings/sign-in-rules",
  organizationSecurity: "/settings/security",
  themes: "/settings/themes",
  status: "/settings/status",
  mailServer: "/settings/email",
  projects: "/settings/projects",
  fieldPermissions: "/settings/field-permissions",
  peopleVisibility: "/settings/people-visibility",
  checkAccess: "/settings/check-access",
  project: "/p/$project",
  projectSettings: appPaths.projectSettings,
  projectRoles: "/p/$project/settings/roles",
  projectFields: "/p/$project/settings/fields",
  projectLayouts: "/p/$project/settings/layouts",
  projectIssueTypes: "/p/$project/settings/issue-types",
  projectLabels: "/p/$project/settings/labels",
  projectWorkflows: "/p/$project/settings/workflows",
  projectPeople: "/p/$project/settings/people",
  projectCheckAccess: "/p/$project/settings/check-access",
  projectIssues: "/p/$project/issues",
  issue: appPaths.issue,
} as const;

/** An address of the app, as the routes know it. */
export type Path = (typeof paths)[keyof typeof paths];

/** The parts of an address that change from page to page, by the name the route gives them. */
export const routeParam = {
  project: "project",
  person: "person",
  role: "role",
  field: "field",
  issue: "issue",
} as const;

/** The name of a part of an address that changes. */
export type RouteParam = (typeof routeParam)[keyof typeof routeParam];

/** The address of a route with one changing part filled in: `/p/$project` and `WEB` is `/p/WEB`. */
function filled(path: Path, param: RouteParam, value: string): string {
  return path.replace(`$${param}`, encodeURIComponent(value));
}

/** The address of a project: its short name, or an old one, in `/p/WEB`. */
export function projectAddress(shortName: string): string {
  return filled(paths.project, routeParam.project, shortName);
}

/** The address of a page below a project, like `/p/WEB/settings/fields`, with the short name filled in. */
export function addressInProject(path: Extract<Path, `/p/${string}`>, shortName: string): string {
  return filled(path, routeParam.project, shortName);
}

/** The address of the page of a person, `/settings/people/<ID>`. */
export function personAddress(id: string): string {
  return filled(paths.person, routeParam.person, id);
}

/** The address of *Groups* with one group open, `/settings/groups?open=<ID>`. */
export function groupAddress(id: string): string {
  return `${paths.groups}?open=${encodeURIComponent(id)}`;
}

/** The address of *Kinds of relation* with one kind open, `/settings/relation-kinds?open=<ID>`. */
export function relationKindAddress(id: string): string {
  return `${paths.relationKinds}?open=${encodeURIComponent(id)}`;
}

/** The address of *Labels and tags* with one group, label or tag open, `/settings/labels?open=<ID>`. */
export function labelsAddress(id: string): string {
  return `${paths.labels}?open=${encodeURIComponent(id)}`;
}

/** The address of *Issue types* with one type open, `/settings/issue-types?open=<ID>`. */
export function issueTypeAddress(id: string): string {
  return `${paths.issueTypes}?open=${encodeURIComponent(id)}`;
}

/**
 * The address of an editor of a place (layouts, workflows): of a project, or of the whole
 * organization without one, with what is open in it as the search part (`?form=…&type=<ID>`);
 * what is left out is not in the address.
 */
function editorAddress(
  organization: Path,
  project: Extract<Path, `/p/${string}`>,
  shortName: string | undefined,
  open: Readonly<Record<string, string | undefined>>,
): string {
  const base = shortName === undefined ? organization : addressInProject(project, shortName);
  const query = new URLSearchParams();
  for (const [name, value] of Object.entries(open)) {
    if (value !== undefined) {
      query.set(name, value);
    }
  }
  const text = query.toString();
  return text === "" ? base : `${base}?${text}`;
}

/**
 * The address of the editor of a layout, with the form and the issue type that are open
 * (`?form=new-issue&type=<ID>`).
 */
export function layoutsAddress(shortName: string | undefined, open: LayoutsOpen = {}): string {
  return editorAddress(paths.layouts, paths.projectLayouts, shortName, {
    form: open.form,
    type: open.type,
  });
}

/** What is open in the editor of layouts: the form (`issue-page`, `new-issue`) and the issue type's ID. */
export interface LayoutsOpen {
  readonly form?: string | undefined;
  readonly type?: string | undefined;
}

/** What is open in the editor of workflows: the ID of the issue type; none for all issue types. */
export interface WorkflowsOpen {
  readonly type?: string | undefined;
}

/** The address of the editor of workflows, with the issue type that is open (`?type=<ID>`). */
export function workflowsAddress(shortName: string | undefined, open: WorkflowsOpen = {}): string {
  return editorAddress(paths.workflows, paths.projectWorkflows, shortName, { type: open.type });
}

/** The address of the page of an own field, `/settings/fields/<ID>`. */
export function fieldAddress(id: string): string {
  return filled(paths.field, routeParam.field, id);
}

/** The address of the page of a role, `/settings/roles/<ID>`. */
export function roleAddress(id: string): string {
  return filled(paths.role, routeParam.role, id);
}

/** The address of an issue, `/issues/WEB-142`: the focus view, and the link that is shared. */
export function issueAddress(key: string): string {
  return filled(paths.issue, routeParam.issue, key);
}

/** What an issue page is opened at: a comment, or the change that happened at a moment. */
export type IssueTarget =
  | { readonly kind: "comment"; readonly comment: string }
  | { readonly kind: "change"; readonly at: string };

/** The names of the parts of an address that say where on the issue page to look. */
export const targetParam = { comment: "comment", change: "change" } as const;

/** The address of an issue with the comment or the change that *Open* of a notification goes to. */
export function issueTargetAddress(key: string, target: IssueTarget): string {
  const query = new URLSearchParams();
  if (target.kind === "comment") {
    query.set(targetParam.comment, target.comment);
  } else {
    query.set(targetParam.change, target.at);
  }
  return `${issueAddress(key)}?${query.toString()}`;
}

/** The address of the issues of a project, `/p/WEB/issues`. */
export function projectIssuesAddress(shortName: string): string {
  return addressInProject(paths.projectIssues, shortName);
}

/** The address of the issues of a project with one issue open in the side panel. */
export function issuePanelAddress(shortName: string, key: string): string {
  return `${projectIssuesAddress(shortName)}?open=${encodeURIComponent(key)}`;
}
