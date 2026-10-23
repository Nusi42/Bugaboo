import type { Schema } from "@bugaboo/api-client";

import type { WhereItCounts, RowPlace } from "../../shared/access-words.ts";
import { api } from "../../shared/api.ts";
import { fieldList, linesFrom, pageSize, usePaged } from "../../shared/paged.ts";
import { namedPerson, type SubjectKind } from "../../shared/person.ts";
import { queryKeys } from "../../shared/query-keys.ts";

/** Whom a row is for. */
export interface Subject {
  readonly kind: SubjectKind;
  readonly id: string;
  readonly name: string;
}

/** A row of *Who has which role* that has what a line of the list needs. */
export interface RowLine {
  readonly id: string;
  readonly who: Subject;
  /** The role, or `null` for *No access*. */
  readonly role: Schema<"NamedRef"> | null;
  readonly place: RowPlace;
  readonly appliesTo: WhereItCounts;
  /** The row comes from above the place the list is for. */
  readonly inherited: boolean;
}

/** The subject of a row: a person shown by name, or a group. Nothing for a person who is not named. */
function subjectOf(item: Schema<"AssignmentItem">): Subject | undefined {
  const { group } = item;
  const person = namedPerson(item.person);
  if (person !== undefined) {
    return { kind: "person", ...person };
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
  return linesFrom(pages, rowLineOf);
}

/** The rows of the whole organization (`null`) or of a project (its ID), with the rows from above. */
export function useAssignments(project: string | null) {
  return usePaged({
    queryKey: queryKeys.assignments.list(project),
    read: (after) => {
      const query = { limit: pageSize.largest, ...after };
      return project === null
        ? api.client.GET("/api/v1/assignments", { params: { query } })
        : api.client.GET("/api/v1/projects/{project}/assignments", {
            params: { path: { project }, query },
          });
    },
  });
}

/** What a form of *Who has which role* chooses from. */
export type ChoiceKind = SubjectKind | "role";

/** One thing to choose: a person, a group or a role. */
export interface Choice {
  readonly id: string;
  readonly name: string;
}

/** The choices in the pages read so far. */
export function choicesOf(pages: readonly Schema<"Page_ChoiceItem">[] | undefined): Choice[] {
  return linesFrom(pages, (item) =>
    item.id === undefined || item.id === null || item.name === undefined || item.name === null
      ? undefined
      : { id: item.id, name: item.name },
  );
}

/** The people, groups or roles a row can be made from, found by name, a page at a time. */
export function useChoices(kind: ChoiceKind, project: string | null, search: string) {
  return usePaged({
    queryKey: queryKeys.assignments.choices(kind, project, search),
    read: (after) => {
      const query = {
        kind,
        fields: fieldList<Schema<"ChoiceItem">>(["id", "name"]),
        limit: kind === "role" ? pageSize.largest : pageSize.short,
        ...(search === "" ? {} : { search }),
        ...after,
      };
      return project === null
        ? api.client.GET("/api/v1/assignment-choices", { params: { query } })
        : api.client.GET("/api/v1/projects/{project}/assignment-choices", {
            params: { path: { project }, query },
          });
    },
  });
}
