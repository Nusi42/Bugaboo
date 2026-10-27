import { ApiError, dataOf, type Schema } from "@bugaboo/api-client";
import { keepPreviousData, useQuery } from "@tanstack/react-query";

import { api } from "../../shared/api.ts";
import { listWord } from "../../shared/list-columns.ts";
import { fieldList, linesFrom, pageSize, usePaged } from "../../shared/paged.ts";
import { known } from "../../shared/present.ts";
import { queryKeys } from "../../shared/query-keys.ts";

/** An issue in a list, as the server sends it: the fields can be missing. */
type FoundItem = Schema<"IssueItem">;

/**
 * What a row needs whatever the columns are: its ID, its title, the status and priority it is
 * tinted and edged by (R4), and the project its title opens in.
 */
const BASE_FIELDS = ["id", "readable_id", "title", "status", "priority", "project"] as const;

/**
 * The field of an issue each column needs, by the search word of the column. A word that is not
 * here is a field of the organization, which an issue carries in `own_fields`.
 */
const fieldOfWord: Readonly<Record<string, keyof FoundItem>> = {
  [listWord.type]: "issue_type",
  [listWord.assignedTo]: "assigned_to",
  [listWord.changed]: "updated_at",
  [listWord.created]: "created_at",
  [listWord.closed]: "closed_at",
  [listWord.reportedBy]: "reported_by",
  [listWord.labels]: "labels",
  [listWord.project]: "project",
};

/** The words whose values the row already has, and which add nothing to the fields asked for. */
const inBase: ReadonlySet<string> = new Set([
  listWord.id,
  listWord.title,
  listWord.status,
  listWord.priority,
  listWord.statusGroup,
]);

/** What a column of this search word asks of the server: nothing, one field, or the own fields. */
function fieldFor(word: string): keyof FoundItem | undefined {
  return inBase.has(word) ? undefined : (fieldOfWord[word] ?? "own_fields");
}

/**
 * The `fields` of a request for a page of the list (PERF-02): what a row needs and what the
 * columns show, and no more.
 */
export function fieldsFor(columns: readonly string[]): string {
  const asked = new Set<keyof FoundItem>(BASE_FIELDS);
  for (const word of columns) {
    const field = fieldFor(word);
    if (field !== undefined) {
      asked.add(field);
    }
  }
  return fieldList<FoundItem>([...asked]);
}

/**
 * What the list asks for when the server refuses the fields of the columns: a field that some
 * issue of the page hides is refused for the whole page, like one that does not exist (SEC-06),
 * and the list must still show the issues. The cells of the columns it leaves out stay empty.
 */
const FALLBACK_FIELDS = fieldList<FoundItem>([
  ...BASE_FIELDS,
  "issue_type",
  "assigned_to",
  "updated_at",
]);

/** An issue that has what a row needs; the rest depends on the columns. */
export interface FoundLine {
  readonly id: string;
  /** The readable ID (`WEB-142`). */
  readonly key: string;
  readonly title: string;
  readonly status: Schema<"StatusRef">;
  readonly group: Schema<"StatusGroup">;
  readonly priority: Schema<"Priority">;
  readonly issueType: Schema<"IssueTypeRef"> | undefined;
  readonly assignedTo: readonly Schema<"PersonRef">[];
  readonly project: Schema<"ProjectRef"> | undefined;
  readonly labels: readonly Schema<"LabelRefBody">[];
  readonly reportedBy: Schema<"PersonRef"> | undefined;
  readonly createdAt: string | undefined;
  readonly changedAt: string | undefined;
  readonly closedAt: string | undefined;
  /** The values of the fields of the organization, by the ID of the field. */
  readonly own: Readonly<Record<string, unknown>>;
}

/** The row of an issue, or nothing for an item that came without what a row needs. */
export function foundLineOf(item: FoundItem): FoundLine | undefined {
  const [id, key, title] = [known(item.id), known(item.readable_id), known(item.title)];
  const [status, priority] = [known(item.status), known(item.priority)];
  if (
    id === undefined ||
    key === undefined ||
    title === undefined ||
    status === undefined ||
    priority === undefined
  ) {
    return undefined;
  }
  return {
    id,
    key,
    title,
    status,
    group: status.status_group,
    priority,
    issueType: known(item.issue_type),
    assignedTo: item.assigned_to ?? [],
    project: known(item.project),
    labels: item.labels ?? [],
    reportedBy: known(item.reported_by),
    createdAt: known(item.created_at),
    changedAt: known(item.updated_at),
    closedAt: known(item.closed_at),
    own: item.own_fields ?? {},
  };
}

/** What a search is: its canonical text and the project it looks in (all of them without). */
export interface FoundSearch {
  readonly text: string;
  /** The ID of the project; none looks in every project the person sees. */
  readonly within: string | undefined;
}

function queryOf({ text, within }: FoundSearch) {
  return {
    ...(text === "" ? {} : { q: text }),
    ...(within === undefined ? {} : { within }),
  };
}

/** The issues a search finds, a page at a time, with what the columns show. */
export function useFoundIssues(search: FoundSearch, columns: readonly string[], enabled = true) {
  const fields = fieldsFor(columns);
  const page = (asked: string) => (after: { readonly cursor?: string }) =>
    api.client.GET("/api/v1/search/issues", {
      params: {
        query: { ...queryOf(search), fields: asked, limit: pageSize.usual, ...after },
      },
    });
  return usePaged(
    {
      queryKey: queryKeys.search.found(search.text, search.within, fields),
      read: async (after) => {
        try {
          return await page(fields)(after);
        } catch (error) {
          if (error instanceof ApiError && error.problem?.code === "request.unknown-field") {
            return page(FALLBACK_FIELDS)(after);
          }
          throw error;
        }
      },
    },
    { placeholderData: keepPreviousData, enabled },
  );
}

/** How many issues a search finds: the number the count line says. */
export function useFoundCount(search: FoundSearch) {
  return useQuery({
    queryKey: queryKeys.search.foundCount(search.text, search.within),
    placeholderData: keepPreviousData,
    queryFn: async () =>
      dataOf(
        await api.client.GET("/api/v1/search/issues/count", {
          params: { query: queryOf(search) },
        }),
      ).issues,
  });
}

/** The groups of a search by a field, in the server's order, with how many issues each holds. */
export function useFoundGroups(search: FoundSearch, by: string | null) {
  return useQuery({
    queryKey: queryKeys.search.foundGroups(search.text, search.within, by ?? ""),
    placeholderData: keepPreviousData,
    enabled: by !== null,
    queryFn: async () =>
      dataOf(
        await api.client.GET("/api/v1/search/issues/groups", {
          params: { query: { ...queryOf(search), by: by ?? "" } },
        }),
      ),
  });
}

/** The ID of an issue of a page, if the page has it. */
function idOf(item: FoundItem): string | undefined {
  return known(item.id);
}

/** One page of IDs: the ID alone is asked for (PERF-02). */
async function idPage(search: FoundSearch, cursor: string | undefined) {
  return dataOf(
    await api.client.GET("/api/v1/search/issues", {
      params: {
        query: {
          ...queryOf(search),
          fields: fieldList<FoundItem>(["id"]),
          limit: pageSize.largest,
          ...(cursor === undefined ? {} : { cursor }),
        },
      },
    }),
  );
}

/**
 * The IDs of the issues a search finds, up to `most`, page after page: what *Tick all results*
 * ticks.
 */
export async function readIssueIds(
  search: FoundSearch,
  most: number,
  cursor?: string,
  before: readonly string[] = [],
): Promise<readonly string[]> {
  const page = await idPage(search, cursor);
  const ids = [...before, ...linesFrom([page], idOf)];
  const next = known(page.next_cursor);
  return next === undefined || ids.length >= most
    ? ids.slice(0, most)
    : readIssueIds(search, most, next, ids);
}
