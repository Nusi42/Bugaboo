import { type Schema, dataOf } from "@bugaboo/api-client";
import { useQuery } from "@tanstack/react-query";

import { api } from "../../shared/api.ts";
import { fieldList, linesFrom, pageSize, usePaged } from "../../shared/paged.ts";
import { namedPerson, personName, type NamedPerson } from "../../shared/person.ts";
import { queryKeys } from "../../shared/query-keys.ts";

/** A person in a list, as the server sends it: the fields can be missing. */
export type ListItem = Schema<"PersonItem">;

/** The fields the list and the page of a person need. */
const SHOWN_FIELDS = fieldList<ListItem>(["person", "email", "standing", "groups", "roles"]);

/** A person that has what a line of the list needs (the fields can be missing in an answer). */
export interface PersonLine extends NamedPerson {
  readonly email: string;
  /** The groups the person is in, when the viewer may see groups. */
  readonly groups: readonly Schema<"NamedRef">[] | null;
  readonly roles: readonly Schema<"RoleAtRef">[];
  /** Whether the person may sign in, or is switched off (ADR-0044, decision 5). */
  readonly standing: Schema<"Standing">;
}

/** The line of a person, or nothing for one that is not shown by name (nobody who may ask is). */
export function lineOf(item: ListItem): PersonLine | undefined {
  const person = namedPerson(item.person);
  if (person === undefined) {
    return undefined;
  }
  return {
    ...person,
    email: item.email ?? "",
    groups: item.groups ?? null,
    roles: item.roles ?? [],
    standing: item.standing ?? "active",
  };
}

/** What the page of a person is called: their name, or *Former user K7Q2* once they are removed. */
export function pageNameOf(item: ListItem): string | undefined {
  const person = item.person ?? undefined;
  return person?.shown === "name" || person?.shown === "former" ? personName(person) : undefined;
}

/** The lines of the pages read so far. */
export function linesOf(pages: readonly Schema<"Page_PersonItem">[] | undefined): PersonLine[] {
  return linesFrom(pages, lineOf);
}

/** The people that the search finds, a page at a time (all of them for an empty search). */
export function usePeople(search: string) {
  return usePaged({
    queryKey: queryKeys.people.list(search),
    read: (after) =>
      api.client.GET("/api/v1/people", {
        params: {
          query: {
            fields: SHOWN_FIELDS,
            limit: pageSize.usual,
            ...(search === "" ? {} : { search }),
            ...after,
          },
        },
      }),
  });
}

/** How often the page of a person who is being removed asks again: the job takes seconds to minutes. */
const REMOVING_READ_MS = 2000;

/** One person with their groups and roles; while they are being removed, read again every few seconds. */
export function usePerson(id: string) {
  return useQuery({
    queryKey: queryKeys.people.one(id),
    queryFn: async () =>
      dataOf(
        await api.client.GET("/api/v1/people/{person}", {
          params: { path: { person: id } },
        }),
      ),
    // While Bugaboo still removes the person, the page asks again, so it shows the end of it.
    refetchInterval: (
      query: Readonly<{ state: Readonly<{ data: Schema<"PersonDetailBody"> | undefined }> }>,
    ) => (query.state.data?.removing === true ? REMOVING_READ_MS : false),
  });
}
