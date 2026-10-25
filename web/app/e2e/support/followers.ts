import { type Page } from "@playwright/test";

import { pageSize } from "../../src/shared/paged.ts";

/** How many followers the long count has: a whole short page. */
export const pageOfFollowers = pageSize.short;

/**
 * Makes the server's list of followers of an issue look longer: the first follower repeated `count`
 * times, with a next page, so that the row says "20 or more followers".
 */
export async function withManyFollowers(page: Page, issue: string, count: number): Promise<void> {
  await page.route(`**/api/v1/issues/${issue}/followers*`, async (route) => {
    const body: unknown = await (await route.fetch()).json();
    const items: unknown =
      typeof body === "object" && body !== null ? Reflect.get(body, "items") : null;
    if (!Array.isArray(items)) {
      throw new TypeError("the list of followers has no items");
    }
    const known: readonly unknown[] = items;
    const first = known[0];
    await route.fulfill({
      json: { items: Array.from({ length: count }, () => first), next_cursor: "more" },
    });
  });
}
