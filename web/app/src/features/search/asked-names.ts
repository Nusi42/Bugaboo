/**
 * The names that only the server knows (UX design section 4.3, WP-02 task 5): people, groups,
 * projects and issues. The module says what to ask for and what the names start with; this asks
 * the routes that exist and gives each name as the language writes it. The names of people and
 * groups come from the server, which lists only those this person may use: the checker of the server still
 * decides when the search runs.
 */
import { dataOf, type Schema } from "@bugaboo/api-client";
import { t } from "@bugaboo/messages";
import type { AskKind, SearchLanguage } from "@bugaboo/wasm-search";
import { keepPreviousData, useQuery } from "@tanstack/react-query";

import { api } from "../../shared/api.ts";
import { queryKeys } from "../../shared/query-keys.ts";
import { issueSource, nameSource } from "../../shared/apply-suggestion.ts";
import { fieldFilter, nowhere } from "../../shared/search-tree.ts";

/** A name that can be taken into the text. */
export interface AskedName {
  /** What stands in the text, written as the language writes it. */
  readonly source: string;
  /** What the person reads: the name, with the short name or the title beside it. */
  readonly label: string;
  /** The name itself, for a filter that writes it through the language: the short name, the ID. */
  readonly value: string;
}

/** How many names are offered at once. */
const offered = 8;

/** The names of people or groups that the server says this person may use in a search. */
async function serverNamesStarting(
  language: SearchLanguage,
  kind: "people" | "groups",
  words: string,
): Promise<readonly AskedName[]> {
  const found: Schema<"SearchNamesBody"> = dataOf(
    await api.client.GET("/api/v1/search/names", {
      params: { query: { kind, starting_with: words, limit: offered } },
    }),
  );
  return found.names.map((name) => ({
    source: nameSource(language, name),
    label: name,
    value: name,
  }));
}

async function projectsStarting(words: string): Promise<readonly AskedName[]> {
  const page: Schema<"Page_ProjectItem"> = dataOf(
    await api.client.GET("/api/v1/projects", {
      params: { query: { search: words, limit: offered, fields: "short_name,name" } },
    }),
  );
  return page.items.flatMap((item) =>
    item.short_name === undefined || item.short_name === null
      ? []
      : [
          {
            source: item.short_name,
            value: item.short_name,
            label: t("search.found-project", { short: item.short_name, name: item.name ?? "" }),
          },
        ],
  );
}

async function issuesStarting(
  language: SearchLanguage,
  words: string,
): Promise<readonly AskedName[]> {
  // Words that are exactly an Issue ID are offered as that ID; the words are looked for in the titles.
  const id = language.issueIdOf(words);
  const titles: Schema<"Page_IssueItem"> = dataOf(
    await api.client.GET("/api/v1/search/issues", {
      params: {
        query: {
          q: language.print({
            filter: fieldFilter("title", {
              type: "contains",
              text: { text: words, span: nowhere },
              span: nowhere,
            }),
            sort: [],
            span: nowhere,
          }),
          limit: offered,
          fields: "readable_id,title",
        },
      },
    }),
  );
  const named = titles.items.flatMap((item) =>
    item.readable_id === undefined || item.readable_id === null
      ? []
      : [
          {
            source: issueSource(language, item.readable_id),
            value: item.readable_id,
            label: t("search.found-issue", { id: item.readable_id, title: item.title ?? "" }),
          },
        ],
  );
  return id === undefined ? named : [{ source: id, value: id, label: id }, ...named];
}

function namesStarting(
  language: SearchLanguage,
  what: AskKind,
  words: string,
): Promise<readonly AskedName[]> {
  if (what === "people" || what === "groups") {
    return serverNamesStarting(language, what, words);
  }
  return what === "projects" ? projectsStarting(words) : issuesStarting(language, words);
}

/** The names of one kind that start with the words typed; none when nothing is asked or the server will not say. */
export function useAskedNames(
  language: SearchLanguage,
  ask: { readonly what: AskKind; readonly startingWith: string } | undefined,
): readonly AskedName[] {
  const query = useQuery({
    queryKey: queryKeys.search.names(ask?.what ?? "", ask?.startingWith ?? ""),
    enabled: ask !== undefined,
    placeholderData: keepPreviousData,
    retry: false,
    queryFn: async () => {
      if (ask === undefined) {
        return [];
      }
      try {
        return await namesStarting(language, ask.what, ask.startingWith);
      } catch {
        // Not allowed to list, or the search was not understood: no names to offer, no mistake to show.
        return [];
      }
    },
  });
  return ask === undefined ? [] : (query.data ?? []);
}
