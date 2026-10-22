import type { EditWhat } from "@bugaboo/api-client";

import { queryClient } from "./query-client.ts";

/**
 * The keys of what the screens keep in the query cache. They are defined once, here, and nowhere
 * else, because a change in one place shows in others: the list of people holds each person's
 * groups and roles, a group is used by rows, a role by people, a project by every list of
 * projects. (The keys of the queries that `api.query` makes for itself come from their options.)
 */
export const queryKeys = {
  people: {
    all: ["people"],
    list: (search: string) => ["people", "list", search],
    one: (id: string) => ["people", "one", id],
  },
  groups: {
    all: ["groups"],
    list: () => ["groups", "list"],
    one: (id: string) => ["groups", "one", id],
    members: (group: string) => ["groups", "members", group],
    choices: (group: string, search: string) => ["groups", "choices", group, search],
  },
  roles: {
    all: ["roles"],
    lists: () => ["roles", "list"],
    list: (which: string) => ["roles", "list", which],
    one: (id: string) => ["roles", "one", id],
    catalogue: () => ["roles", "catalogue"],
  },
  assignments: {
    all: ["assignments"],
    list: (project: string | null) => ["assignments", "list", project],
    choices: (kind: string, project: string | null, search: string) => [
      "assignments",
      "choices",
      kind,
      project,
      search,
    ],
  },
  fields: {
    /** The projects offered while choosing where a field is used. */
    projectChoices: (search: string) => ["fields", "projects", search],
  },
  projects: {
    all: ["projects"],
    list: (filter: object, size: number) => ["projects", "list", filter, size],
    one: (address: string) => ["projects", "one", address],
    movePreview: (project: string, to: string | null) => ["projects", "move-preview", project, to],
  },
  layouts: {
    /** The draft that a refused save of a layout kept, by layout. */
    draft: (layout: string) => ["layouts", "draft", layout],
  },
  editing: {
    /** Who is editing a thing now (a layout, a workflow): what its edit lock is about. */
    now: (what: EditWhat) => ["editing", "now", what.record.kind, what.record.id, what.part],
  },
  labels: {
    all: ["labels"],
    /** The groups of labels; with a project, each says whether the project offers it. */
    groups: (project?: string) => ["labels", "groups", project ?? null],
    /** The labels; with a project, those of groups that can reach it. */
    list: (project?: string) => ["labels", "list", project ?? null],
  },
  issueTypes: {
    all: ["issue-types"],
    /** The types; with a project, each says whether the project offers it. */
    list: (project?: string) => ["issue-types", "list", project ?? null],
    /** The lines that say where one type is offered. */
    offers: (issueType: string) => ["issue-types", "offers", issueType],
  },
  relationKinds: {
    all: ["relation-kinds"],
    list: () => ["relation-kinds", "list"],
  },
  tags: {
    all: ["tags"],
    list: () => ["tags", "list"],
  },
  issueRelations: {
    all: ["issue-relations"],
    header: (issue: string) => ["issue-relations", "header", issue],
    subIssues: (issue: string) => ["issue-relations", "sub-issues", issue],
    related: (issue: string) => ["issue-relations", "related", issue],
  },
  history: {
    all: ["history"],
    list: (issue: string, tab: string) => ["history", "list", issue, tab],
    versions: (comment: string) => ["history", "versions", comment],
    /** Who changed a long text last, as of one version of the issue. */
    changedBy: (issue: string, part: string, version: number) => [
      "history",
      "changed-by",
      issue,
      part,
      version,
    ],
  },
  followers: {
    all: ["followers"],
    state: (issue: string) => ["followers", "state", issue],
    list: (issue: string) => ["followers", "list", issue],
  },
  notifications: {
    /** A page-by-page list of notifications; it starts with the route like `api.query` keys do. */
    list: (filter: string, size: number) => ["get", "/api/v1/notifications", "list", filter, size],
  },
  issues: {
    all: ["issues"],
    lists: () => ["issues", "list"],
    list: (project: string) => ["issues", "list", project],
    one: (address: string) => ["issues", "one", address],
    statusChanges: (project: string) => ["issues", "status-changes", project],
    people: (project: string, search: string, field = "", issue = "", issueType = "") => [
      "issues",
      "assignable-people",
      project,
      search,
      field,
      issue,
      issueType,
    ],
    drafts: (project: string) => ["issues", "new-draft", project],
  },
  invitations: {
    all: ["invitations"],
    one: (secret: string) => ["invitations", "one", secret],
  },
  checkAccess: {
    people: (project: string | undefined, search: string) => [
      "check-access",
      "people",
      project ?? null,
      search,
    ],
    projects: (search: string) => ["check-access", "projects", search],
    answer: (
      person: string | undefined,
      project: string | undefined,
      issueType: string | undefined,
    ) => ["check-access", "answer", person ?? null, project ?? null, issueType ?? null],
  },
} as const;

/** The reads that hold an own field, its choices or the values of one. */
const FIELD_READS = [
  "/api/v1/fields",
  "/api/v1/fields/{field}",
  "/api/v1/fields/{field}/used-by",
  "/api/v1/field-types",
  "/api/v1/field-permissions",
  "/api/v1/layouts/form",
  "/api/v1/layouts/editor",
  "/api/v1/issues",
  "/api/v1/issues/{issue}",
  "/api/v1/projects",
  "/api/v1/projects/{project}",
] as const;

/** Which reads to do again: by the start of their key, or by a test of their key. */
type Rereading = Readonly<{
  queryKey?: readonly unknown[];
  predicate?: (query: Readonly<{ queryKey: readonly unknown[] }>) => boolean;
}>;

/**
 * Reads again what matches `filters`, now that something changed. A first read that began before
 * the change is on its way with the state of before, and a read that is on its way is not started
 * again (it has no data to keep): it is cancelled first, so the screen is left with a read that
 * began after the change.
 */
function reread(filters: Rereading): void {
  void queryClient.cancelQueries(filters).then(() => queryClient.invalidateQueries(filters));
}

/** Reads the lists of issues and what an issue shows about itself again: issues changed. */
export function refreshIssues(): void {
  reread({ queryKey: queryKeys.issues.all });
}

/** Reads again every answer of these routes of the API (their keys are made by the client). */
function refreshReads(paths: readonly string[]): void {
  for (const path of paths) {
    reread({ queryKey: ["get", path] });
  }
}

/** An own field or one of its choices changed: every answer that holds fields is read again. */
export function refreshFields(): void {
  refreshReads(FIELD_READS);
}

/** The answers that show the tags and labels of an issue or the lists of them. */
const LABEL_READS = [
  "/api/v1/issues/{issue}/labels-and-tags",
  "/api/v1/projects/{project}/tag-suggestions",
  "/api/v1/issues/{issue}/history",
] as const;

/** A label or tag changed: the answers that show them are read again. */
export function refreshLabelReads(): void {
  reread({ queryKey: queryKeys.labels.all });
  refreshReads(LABEL_READS);
}

/** A layout was made, saved or taken away: the forms and the editor read it again. */
export function refreshLayouts(): void {
  refreshReads(["/api/v1/layouts/form", "/api/v1/layouts/editor"]);
}

/** A save of a layout was refused: the draft it kept is read again. */
export function refreshLayoutDraft(layout: string): void {
  reread({ queryKey: queryKeys.layouts.draft(layout) });
}

/** The answers that show a workflow: the editor, the lists of workflows and what an issue can change to. */
const WORKFLOW_READS = [
  "/api/v1/workflows/editor",
  "/api/v1/workflows",
  "/api/v1/projects/{project}/statuses",
] as const;

/** A workflow was created, published or taken away: the editor and what issues offer are read again. */
export function refreshWorkflows(): void {
  refreshReads(WORKFLOW_READS);
  refreshIssues();
}

/** An issue type was made or changed: the list of types is read again. */
export function refreshIssueTypes(): void {
  reread({ queryKey: queryKeys.issueTypes.all });
}

/** A kind of relation was made or changed: the list of kinds is read again. */
export function refreshRelationKinds(): void {
  reread({ queryKey: queryKeys.relationKinds.all });
}

/**
 * Reads again every query made by `api.query` (its key starts with the method and the route) whose
 * route starts with one of `routes`.
 */
function refreshRoutes(routes: readonly string[]): void {
  reread({
    predicate: (query: Readonly<{ queryKey: readonly unknown[] }>) => {
      const route = query.queryKey[1];
      return typeof route === "string" && routes.some((each) => route.startsWith(each));
    },
  });
}

/** A file was added, deleted or brought back: its list and the history (M4-F50) are read again. */
export function refreshIssueFiles(): void {
  refreshHistory();
  refreshReads(["/api/v1/issues/{issue}/files"]);
}

/** A sub-issue or a relation was added or taken away: what an issue stands among is read again. */
export function refreshIssueRelations(): void {
  reread({ queryKey: queryKeys.issueRelations.all });
  reread({ queryKey: queryKeys.history.all });
  refreshRoutes(["/api/v1/issues/{issue}/relations"]);
}

/** A notification was read: the list and the number on the menu are read again. */
export function refreshNotifications(): void {
  refreshRoutes(["/api/v1/notifications"]);
}

/** The mail server changed (its settings or its password): the answers about it are read again. */
export function refreshMailServer(): void {
  refreshRoutes(["/api/v1/mail", "/api/v1/settings"]);
}

/** A field, a comment, a file or a follower changed: the history shows it the moment it was made. */
export function refreshHistory(): void {
  reread({ queryKey: queryKeys.history.all });
  reread({ queryKey: queryKeys.followers.all });
  refreshRoutes(["/api/v1/issues/{issue}/follow", "/api/v1/comments/{comment}/versions"]);
}

/** Something about people, groups, roles or rows changed: all of it is read again where it is shown. */
export function refreshAccess(): void {
  for (const key of [
    queryKeys.people.all,
    queryKeys.groups.all,
    queryKeys.roles.all,
    queryKeys.assignments.all,
  ]) {
    reread({ queryKey: key });
  }
}
