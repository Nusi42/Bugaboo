import type { ReadonlyDeep, Schema } from "@bugaboo/api-client";
import { t } from "@bugaboo/messages";
import { containsEveryWord } from "@bugaboo/ui";

import { api } from "../../shared/api.ts";
import { fieldList, pageSize } from "../../shared/paged.ts";
import { savedSearchAddress } from "../../shared/paths.ts";
import type { QuickResult, QuickSearchSource } from "./quick-search-sources.ts";

/** How many saved searches quick search shows. */
const RESULTS = 8;

type Item = ReadonlyDeep<Schema<"SavedSearchItem">>;

/** The result of a saved search; the ones shared with the person say so. */
function resultOf(item: Item, scope: Schema<"SearchSet">): QuickResult {
  const found = { id: item.id, title: item.name, path: savedSearchAddress(item.id) };
  return scope === "shared" ? { ...found, hint: t("search.shared-with-me") } : found;
}

/** The saved searches of one set (the ones the person made, or the ones shared with them) that the words name. */
async function namedLike(
  query: string,
  scope: Schema<"SearchSet">,
): Promise<readonly QuickResult[]> {
  const { data } = await api.client.GET("/api/v1/saved-searches", {
    params: {
      query: {
        scope,
        fields: fieldList<Item>(["id", "name"]),
        limit: pageSize.largest,
      },
    },
  });
  return (data?.items ?? [])
    .filter((item: Item) => containsEveryWord(item.name, query))
    .map((item: Item) => resultOf(item, scope));
}

/**
 * Saved searches as a source of quick search (CFG-08): the ones the person made and the ones shared
 * with them whose name holds every word typed. The server answers only with the ones the person
 * may open, so nothing here decides who sees what.
 */
export const savedSearchesSource: QuickSearchSource = {
  id: "saved-searches",
  heading: t("search.saved-searches"),
  paused: true,
  search: async (query): Promise<readonly QuickResult[]> => {
    if (query.trim() === "") {
      return [];
    }
    const [mine, shared] = await Promise.all([
      namedLike(query, "mine"),
      namedLike(query, "shared"),
    ]);
    return [...mine, ...shared].slice(0, RESULTS);
  },
};
