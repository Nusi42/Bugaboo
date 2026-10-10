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
  projects: {
    all: ["projects"],
    list: (filter: object, size: number) => ["projects", "list", filter, size],
    one: (address: string) => ["projects", "one", address],
    movePreview: (project: string, to: string | null) => ["projects", "move-preview", project, to],
  },
  issueTypes: {
    all: ["issue-types"],
    list: () => ["issue-types", "list"],
  },
  relationKinds: {
    all: ["relation-kinds"],
    list: () => ["relation-kinds", "list"],
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
  },
  followers: {
    all: ["followers"],
    state: (issue: string) => ["followers", "state", issue],
    list: (issue: string) => ["followers", "list", issue],
  },
  mentions: {
    all: ["mentions"],
    list: () => ["mentions", "list"],
    choices: (issue: string, search: string) => ["mentions", "choices", issue, search],
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
    answer: (person: string | undefined, project: string | undefined) => [
      "check-access",
      "answer",
      person ?? null,
      project ?? null,
    ],
  },
} as const;

/** An issue type was made or changed: the list of types is read again. */
export function refreshIssueTypes(): void {
  void queryClient.invalidateQueries({ queryKey: queryKeys.issueTypes.all });
}

/** A kind of relation was made or changed: the list of kinds is read again. */
export function refreshRelationKinds(): void {
  void queryClient.invalidateQueries({ queryKey: queryKeys.relationKinds.all });
}

/**
 * Reads again every query made by `api.query` (its key starts with the method and the route) whose
 * route starts with one of `routes`.
 */
function refreshRoutes(routes: readonly string[]): void {
  void queryClient.invalidateQueries({
    predicate: (query: Readonly<{ queryKey: readonly unknown[] }>) => {
      const route = query.queryKey[1];
      return typeof route === "string" && routes.some((each) => route.startsWith(each));
    },
  });
}

/** A sub-issue or a relation was added or taken away: what an issue stands among is read again. */
export function refreshIssueRelations(): void {
  void queryClient.invalidateQueries({ queryKey: queryKeys.issueRelations.all });
  void queryClient.invalidateQueries({ queryKey: queryKeys.history.all });
  refreshRoutes(["/api/v1/issues/{issue}/relations"]);
}

/** A comment was written, changed or deleted, or a follower came or went: the history is read again. */
export function refreshHistory(): void {
  void queryClient.invalidateQueries({ queryKey: queryKeys.history.all });
  void queryClient.invalidateQueries({ queryKey: queryKeys.followers.all });
  void queryClient.invalidateQueries({ queryKey: queryKeys.mentions.all });
  refreshRoutes([
    "/api/v1/issues/{issue}/follow",
    "/api/v1/me/mentions",
    "/api/v1/comments/{comment}/versions",
  ]);
}

/** Something about people, groups, roles or rows changed: all of it is read again where it is shown. */
export function refreshAccess(): void {
  for (const key of [
    queryKeys.people.all,
    queryKeys.groups.all,
    queryKeys.roles.all,
    queryKeys.assignments.all,
  ]) {
    void queryClient.invalidateQueries({ queryKey: key });
  }
}
