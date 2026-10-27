import { type Page } from "@playwright/test";

import { projectIssuesAddress } from "../src/shared/paths.ts";
import { ada } from "./support/accounts.ts";
import { expect, test } from "./support/fixtures.ts";
import { signIn } from "./support/flows.ts";
import { webIssues } from "./support/issues.ts";
import { grid, rowOf } from "./support/list-look.ts";
import { panelCloseOf, preparePanel } from "./support/list-panel.ts";

// The issue open in the side panel is one more part of the address of the list (UX §4.2, §2.1):
// opening it and closing it leave the search, the sort, the group, the columns and the spacing as
// they were.

test.use({ preparation: { run: preparePanel } });
test.describe.configure({ mode: "serial" });

const kept = {
  s: "priority is not low sort by title",
  group: "priority",
  spacing: "compact",
  cols: "status,id,title,priority",
} as const;

/** The parts of the address of the page, as the app wrote them. */
function parts(page: Page): Record<string, string> {
  return Object.fromEntries(new URL(page.url()).searchParams);
}

test("ux_4_2 opening an issue in the panel and closing it keeps the search, the sort, the group and the view", async ({
  page,
}) => {
  await signIn(page, ada);
  await page.goto(`${projectIssuesAddress("WEB")}?${new URLSearchParams(kept).toString()}`);
  await expect(rowOf(page, webIssues.login)).toBeVisible();
  await expect(grid(page)).toBeVisible();
  await rowOf(page, webIssues.login).getByRole("link", { name: webIssues.login }).click();
  await expect.poll(() => parts(page)["open"]).toBe("WEB-1");
  expect(parts(page)).toEqual({ ...kept, open: "WEB-1" });
  const close = panelCloseOf(page);
  await close.click();
  await expect.poll(() => parts(page)["open"]).toBeUndefined();
  expect(parts(page)).toEqual(kept);
  // The list is still the one asked for: sorted by title, in groups of priority.
  await expect(rowOf(page, webIssues.login)).toBeVisible();
});
