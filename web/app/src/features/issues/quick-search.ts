import type { ReadonlyDeep } from "@bugaboo/api-client";
import { t } from "@bugaboo/messages";
import { loadSearchLanguage } from "@bugaboo/wasm-search";

import { api } from "../../shared/api.ts";
import { fieldList } from "../../shared/paged.ts";
import { issueAddress } from "../../shared/paths.ts";
import type { QuickResult, QuickSearchSource } from "../shell/index.ts";
import type { ListedIssue } from "./issues.ts";
import { titleSearchText } from "../../shared/title-search.ts";

/** How many issues quick search shows for words (M5-Q3). */
export const ISSUE_RESULTS = 10;

/**
 * Whether a text looks like the ID of an issue (ADR-0021): a short name, a hyphen and a code, as
 * in `WEB-142` or `web-k7q2x`. Only for the order of the results and for the question to the
 * server; the server decides what an ID is and whether the person may see the issue.
 */
export function looksLikeIssueId(text: string): boolean {
  return /^[a-z][a-z0-9]*-[a-z0-9]+$/iu.test(text.trim());
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

/** The issue with this ID, if it exists and the person may see it; a hidden one and a missing one are the same. */
async function issueWithId(text: string): Promise<readonly QuickResult[]> {
  const { data } = await api.client.GET("/api/v1/issues/{issue}", {
    params: { path: { issue: text.trim() } },
  });
  return data === undefined ? [] : resultOf(data.issue);
}

/** The first issues whose title holds the words, among those the person may see. */
async function issuesWithWords(words: string): Promise<readonly QuickResult[]> {
  const q = titleSearchText(await loadSearchLanguage(), words);
  if (q === undefined) {
    return [];
  }
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
  firstFor: looksLikeIssueId,
  search: async (query): Promise<readonly QuickResult[]> => {
    const [byId, byWords] = await Promise.all([
      looksLikeIssueId(query) ? issueWithId(query) : [],
      issuesWithWords(query),
    ]);
    const seen = new Set(byId.map((each) => each.id));
    return [...byId, ...byWords.filter((each) => !seen.has(each.id))];
  },
};
