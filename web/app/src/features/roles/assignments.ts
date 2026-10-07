import { ApiError, type Schema } from "@bugaboo/api-client";
import { useInfiniteQuery } from "@tanstack/react-query";

import { assignmentsKey } from "../../shared/access-cache.ts";
import type { WhereItCounts, RowPlace } from "../../shared/access-words.ts";
import { api } from "../../shared/api.ts";

/** How many rows, and how many choices, one page holds. */
const ROWS_SIZE = 100;
const CHOICES_SIZE = 20;

/** Whom a row is for. */
export interface Subject {
  readonly kind: "person" | "group";
  readonly id: string;
  readonly name: string;
}

/** A row of *Who has which role* that has what a line of the list needs. */
export interface RowLine {
  readonly id: string;
  readonly who: Subject;
  /** The role, or `null` for *No access*. */
  readonly role: Schema<"RoleRef"> | null;
  readonly place: RowPlace;
  readonly appliesTo: WhereItCounts;
  /** The row comes from above the place the list is for. */
  readonly inherited: boolean;
}

/** The subject of a row: a person shown by name, or a group. Nothing for a person who is not named. */
function subjectOf(item: Schema<"AssignmentItem">): Subject | undefined {
  const { person, group } = item;
  if (person !== undefined && person !== null && person.shown === "name") {
    return { kind: "person", id: person.id, name: person.name };
  }
  return group === undefined || group === null
    ? undefined
    : { kind: "group", id: group.id, name: group.name };
}

/** The line of a row, or nothing for an item that came without what it needs. */
export function rowLineOf(item: Schema<"AssignmentItem">): RowLine | undefined {
  const who = subjectOf(item);
  const { id, place, applies_to: appliesTo } = item;
  if (
    who === undefined ||
    id === undefined ||
    id === null ||
    place === undefined ||
    place === null ||
    appliesTo === undefined ||
    appliesTo === null
  ) {
    return undefined;
  }
  return { id, who, role: item.role ?? null, place, appliesTo, inherited: item.inherited === true };
}

/** The rows in the pages read so far. */
export function rowsOf(pages: readonly Schema<"Page_AssignmentItem">[] | undefined): RowLine[] {
  return (pages ?? []).flatMap((page) => page.items.flatMap((item) => rowLineOf(item) ?? []));
}

/** The rows of the whole organization (`null`) or of a project (its ID), with the rows from above. */
export function useAssignments(project: string | null) {
  return useInfiniteQuery({
    queryKey: [...assignmentsKey, "list", project],
    initialPageParam: null as string | null,
    queryFn: async ({ pageParam }: { readonly pageParam: string | null }) => {
      const query = { limit: ROWS_SIZE, ...(pageParam === null ? {} : { cursor: pageParam }) };
      const { data } =
        project === null
          ? await api.client.GET("/api/v1/assignments", { params: { query } })
          : await api.client.GET("/api/v1/projects/{project}/assignments", {
              params: { path: { project }, query },
            });
      if (data === undefined) {
        throw new ApiError(0, undefined);
      }
      return data;
    },
    getNextPageParam: (last: Schema<"Page_AssignmentItem">) => last.next_cursor ?? null,
  });
}

/** What a form of *Who has which role* chooses from. */
export type ChoiceKind = "person" | "group" | "role";

/** One thing to choose: a person, a group or a role. */
export interface Choice {
  readonly id: string;
  readonly name: string;
}

/** The choices in the pages read so far. */
export function choicesOf(pages: readonly Schema<"Page_ChoiceItem">[] | undefined): Choice[] {
  return (pages ?? []).flatMap((page) =>
    page.items.flatMap((item) =>
      item.id === undefined || item.id === null || item.name === undefined || item.name === null
        ? []
        : [{ id: item.id, name: item.name }],
    ),
  );
}

/** The people, groups or roles a row can be made from, found by name, a page at a time. */
export function useChoices(kind: ChoiceKind, project: string | null, search: string) {
  return useInfiniteQuery({
    queryKey: [...assignmentsKey, "choices", kind, project, search],
    initialPageParam: null as string | null,
    queryFn: async ({ pageParam }: { readonly pageParam: string | null }) => {
      const query = {
        kind,
        fields: "id,name",
        limit: kind === "role" ? ROWS_SIZE : CHOICES_SIZE,
        ...(search === "" ? {} : { search }),
        ...(pageParam === null ? {} : { cursor: pageParam }),
      };
      const { data } =
        project === null
          ? await api.client.GET("/api/v1/assignment-choices", { params: { query } })
          : await api.client.GET("/api/v1/projects/{project}/assignment-choices", {
              params: { path: { project }, query },
            });
      if (data === undefined) {
        throw new ApiError(0, undefined);
      }
      return data;
    },
    getNextPageParam: (last: Schema<"Page_ChoiceItem">) => last.next_cursor ?? null,
  });
}
