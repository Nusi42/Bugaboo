import { ApiError, type ReadonlyDeep } from "@bugaboo/api-client";
import { t } from "@bugaboo/messages";
import type { SearchLanguage } from "@bugaboo/wasm-search";

import { api } from "../../shared/api.ts";
import { fieldList } from "../../shared/paged.ts";
import { issueAddress } from "../../shared/paths.ts";
import type { QuickResult, QuickSearchSource } from "../shell/index.ts";
import type { ListedIssue } from "./issues.ts";
import { titleSearchText } from "../../shared/title-search.ts";

/** How many issues quick search shows for words (M5-Q3). */
export const ISSUE_RESULTS = 10;

/**
 * The module of the search language, once it was loaded: it is fetched when words are first
 * searched, not before. Its rule of what an Issue ID is (ADR-0021) is the only one; the order of
 * the groups asks it too, and has the answer by the time there is a group to order.
 */
let language: SearchLanguage | undefined;

async function searchLanguage(): Promise<SearchLanguage> {
  const { loadSearchLanguage } = await import("@bugaboo/wasm-search");
  language ??= await loadSearchLanguage();
  return language;
}

/**
 * The Issue ID that the words are, as the module reads it; none when they are not exactly one or
 * when the module is not loaded yet. The server decides whether the issue exists and may be seen.
 */
export function issueIdIn(words: string): string | undefined {
  return language?.issueIdOf(words);
}

/** What a line of the answer needs: the readable ID and the title. */
type Line = ReadonlyDeep<Pick<ListedIssue, "readable_id" | "title">>;

function resultOf(item: Line): QuickResult[] {
  const key = item.readable_id;
  const title = item.title;
  return typeof key === "string" && typeof title === "string"
    ? [{ id: key, title, hint: key, path: issueAddress(key) }]
    : [];
}

/** The status with which the server says there is no such issue, or none the person may see. */
const NOT_FOUND = 404;

/** The problem with which the server refuses a text of the search language (the checker's mistakes). */
const SEARCH_INVALID = "search.invalid";

/** The server says there is no such issue: nothing to show, not a broken source. */
function isNoIssue(error: unknown): boolean {
  return error instanceof ApiError && error.status === NOT_FOUND;
}

/** The server refuses the text as a search (too many words, too long): it finds nothing. */
function isRefusedText(error: unknown): boolean {
  return error instanceof ApiError && error.problem?.code === SEARCH_INVALID;
}

/**
 * The issue with this ID, if it exists and the person may see it; a hidden one and a missing one
 * are the same: the client throws for both, and both are nothing here.
 */
async function issueWithId(id: string): Promise<readonly QuickResult[]> {
  try {
    const { data } = await api.client.GET("/api/v1/issues/{issue}", {
      params: { path: { issue: id } },
    });
    return data === undefined ? [] : resultOf(data.issue);
  } catch (error) {
    if (isNoIssue(error)) {
      return [];
    }
    throw error;
  }
}

/** The first issues whose title holds the words, among those the person may see. */
async function issuesWithWords(
  module: SearchLanguage,
  words: string,
): Promise<readonly QuickResult[]> {
  const q = titleSearchText(module, words);
  if (q === undefined) {
    return [];
  }
  try {
    const { data } = await api.client.GET("/api/v1/search/issues", {
      params: {
        query: {
          q,
          fields: fieldList<ListedIssue>(["readable_id", "title"]),
          limit: ISSUE_RESULTS,
        },
      },
    });
    return (data?.items ?? []).flatMap((item: Line) => resultOf(item));
  } catch (error) {
    if (isRefusedText(error)) {
      return [];
    }
    throw error;
  }
}

/**
 * Issues as a source of quick search (CFG-08): the issue with the ID that was typed, and the first
 * issues whose title holds the words. Both come from what the person may see, and a text that is no
 * ID asks no ID. The first is not repeated among the second.
 */
export const issuesSource: QuickSearchSource = {
  id: "issues",
  heading: t("search.issues"),
  paused: true,
  firstFor: (query) => issueIdIn(query) !== undefined,
  search: async (query): Promise<readonly QuickResult[]> => {
    const module = await searchLanguage();
    const id = module.issueIdOf(query);
    const [byId, byWords] = await Promise.all([
      id === undefined ? [] : issueWithId(id),
      issuesWithWords(module, query),
    ]);
    const seen = new Set(byId.map((each) => each.id));
    return [...byId, ...byWords.filter((each) => !seen.has(each.id))];
  },
};
