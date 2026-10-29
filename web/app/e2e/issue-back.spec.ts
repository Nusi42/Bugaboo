import { t } from "@bugaboo/messages";
import { type Page } from "@playwright/test";

import { paths, projectIssuesAddress } from "../src/shared/paths.ts";
import { ada } from "./support/accounts.ts";
import { expect, test } from "./support/fixtures.ts";
import { signIn } from "./support/flows.ts";
import { webIssues } from "./support/issues.ts";
import { prepareLook, rowOf } from "./support/list-look.ts";
import { ask, result } from "./support/quick-search.ts";

// Back from an issue (UX §3.4): to the page the person came from, with its search, or, where they
// did not come from a page, to the list of the issue's project.

test.use({ preparation: { run: prepareLook } });

/** Presses *Back to list* of the issue that is open, once it is read (the project it belongs to is known then). */
async function back(page: Page): Promise<void> {
  await expect(page.getByRole("heading", { level: 1, name: webIssues.login })).toBeVisible();
  await page.getByRole("button", { name: t("focus.back"), exact: true }).click();
}

test("ux_3_4 Back from an issue opened on the Search page returns to that search", async ({
  page,
}) => {
  await signIn(page, ada);
  await page.goto(`${paths.search}?s=priority+is+not+low`);
  await rowOf(page, webIssues.login).getByRole("link", { name: webIssues.login }).click();
  await expect(page).toHaveURL(/\/issues\/WEB-1/u);
  await back(page);
  await expect.poll(() => new URL(page.url()).pathname).toBe(paths.search);
  expect(new URL(page.url()).searchParams.get("s")).toBe("priority is not Low");
});

test("ux_3_4 Back from an issue found by Quick search returns to the page it was opened over", async ({
  page,
}) => {
  await signIn(page, ada);
  await page.goto(paths.home);
  await expect(page.getByRole("heading", { level: 1 })).toBeVisible();
  await ask(page, "password");
  await result(page, webIssues.login).click();
  await expect(page).toHaveURL(/\/issues\/WEB-1/u);
  await back(page);
  await expect.poll(() => new URL(page.url()).pathname).toBe(paths.home);
});

test("ux_3_4 Back from an issue opened by its address returns to the list of its project", async ({
  page,
}) => {
  await signIn(page, ada);
  await page.goto("/issues/WEB-1");
  await back(page);
  await expect.poll(() => new URL(page.url()).pathname).toBe(projectIssuesAddress("WEB"));
});
