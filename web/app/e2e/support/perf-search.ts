import { type Page } from "@playwright/test";

import { type Times } from "./perf.ts";

/** The requests of the first page of the Search page, by their path, in the words of the numbers. */
const requests = [
  ["/api/v1/search/issues/groups", "search page, groups"],
  ["/api/v1/search/issues/count", "search page, count"],
  ["/api/v1/search/issues", "search page, first page of issues"],
] as const;

/** How many kinds of request the first page makes. */
export const SEARCH_PAGE_REQUESTS = requests.length;

/** Times each of those requests while the page makes them, from the browser's own measure. */
export function timeSearchPage(page: Page, times: Readonly<Times>): void {
  page.on("response", async (response) => {
    await response.finished();
    const { pathname } = new URL(response.url());
    const kind = requests.find(([path]) => pathname === path)?.[1];
    const timing = response.request().timing();
    if (kind !== undefined && timing.responseEnd > 0) {
      times.add(kind, timing.responseEnd - timing.requestStart);
    }
  });
}
