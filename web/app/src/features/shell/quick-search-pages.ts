import { t } from "@bugaboo/messages";

import { placeAt, placesFor } from "./places.ts";
import type { QuickResult, QuickSearchAsker, QuickSearchSource } from "./quick-search-sources.ts";

/** Whether every word the person typed is in the text, in any order. */
function matches(text: string, query: string): boolean {
  const lower = text.toLowerCase();
  return query
    .toLowerCase()
    .split(/\s+/u)
    .filter((word) => word !== "")
    .every((word) => lower.includes(word));
}

/** The pages of the app: every screen that exists and that this person may open. */
function findPages(query: string, asker: QuickSearchAsker): readonly QuickResult[] {
  return placesFor(asker.permissions)
    .filter((place) => place.listed)
    .flatMap((place) => {
      const parent = place.parent === null ? undefined : placeAt(place.parent);
      const hint = parent === undefined || parent.parent === null ? undefined : t(parent.name);
      const title = t(place.name);
      return matches(`${title} ${hint ?? ""}`, query)
        ? [{ id: place.path, title, path: place.path, ...(hint === undefined ? {} : { hint }) }]
        : [];
    });
}

/** The source of the pages of the app (UX §6.2). Projects and issues are sources of their own. */
export const pagesSource: QuickSearchSource = {
  id: "pages",
  heading: t("search.pages"),
  search: findPages,
};
