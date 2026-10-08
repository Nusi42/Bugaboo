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
