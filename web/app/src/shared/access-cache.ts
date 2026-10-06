import { queryClient } from "./query-client.ts";

/**
 * The keys of what the screens of people, groups, roles and *Who has which role* keep in the query
 * cache. They are defined once, here, because a change in one of them shows in the others: the list
 * of people holds each person's groups and roles, a group is used by rows, a role by people.
 */
export const peopleKey = ["people"] as const;
export const groupsKey = ["groups"] as const;
export const rolesKey = ["roles"] as const;
export const assignmentsKey = ["assignments"] as const;

/** Something about people, groups, roles or rows changed: all of it is read again where it is shown. */
export function refreshAccess(): void {
  for (const key of [peopleKey, groupsKey, rolesKey, assignmentsKey]) {
    void queryClient.invalidateQueries({ queryKey: key });
  }
}
