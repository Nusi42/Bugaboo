import { t } from "@bugaboo/messages";

import { projectIssuesAddress } from "../src/shared/paths.ts";
import { ana } from "./support/access.ts";
import { ada } from "./support/accounts.ts";
import { expect, test } from "./support/fixtures.ts";
import { signIn } from "./support/flows.ts";
import { layoutOf } from "./support/layout.ts";
import { grid, openList, prepareLook, rowOf } from "./support/list-look.ts";
import { listInFourThemes, tickIssues, screenshotFolder } from "./support/list-pictures.ts";
import { webIssues } from "./support/issues.ts";

// The pictures of the issue list (M5 WP-08, task 8): the list in the four themes on a computer and
// on a phone, grouped and not, compact and comfortable, ticked, with the columns panel, empty, and
// with an error. Taken only when `BB_SCREENSHOT_DIR` is set; the builder looks at each.

test.skip(screenshotFolder === undefined, "Set BB_SCREENSHOT_DIR to take the screenshots.");

test.use({ preparation: { run: prepareLook } });
test.describe.configure({ mode: "serial" });

test("iss_01 pictures: the list grouped by status group", async ({ page }) => {
  await openList(page);
  await listInFourThemes(page, "grouped-status");
});

test("cfg_02 pictures: grouped by priority, compact, and not grouped", async ({ page }) => {
  await openList(page);
  await page.getByLabel(t("issues.group-by"), { exact: true }).selectOption({
    label: t("issue-field.priority"),
  });
  await expect(grid(page).getByRole("link", { name: webIssues.login })).toBeVisible();
  await listInFourThemes(page, "grouped-priority");
  await page.getByRole("radio", { name: t("appearance.spacing.compact"), exact: true }).check();
  await listInFourThemes(page, "compact");
  await page.getByLabel(t("issues.group-by"), { exact: true }).selectOption({
    label: t("issues.group-by-none"),
  });
  await listInFourThemes(page, "plain");
});

test("cfg_02 pictures: two issues ticked, and the head of the list ticked", async ({ page }) => {
  test.skip(
    layoutOf(page) === "phone",
    "the tick box of a phone is shown in the cards, with the others",
  );
  await openList(page);
  await tickIssues(page, ["WEB-1", "WEB-2"]);
  await expect(rowOf(page, webIssues.login)).toHaveAttribute("data-state", /selected/u);
  await listInFourThemes(page, "ticked");
});

test("cfg_02 pictures: Customize, then Columns", async ({ page }) => {
  test.skip(layoutOf(page) === "phone", "a phone has cards and no columns to choose");
  await openList(page);
  await page.getByRole("button", { name: t("issues.customize"), exact: true }).click();
  await expect(page.getByRole("complementary", { name: t("issues.customize") })).toBeVisible();
  await listInFourThemes(page, "customize");
});

test("iss_01 pictures: no issue matches, and the search that takes too long", async ({ page }) => {
  await signIn(page, ada);
  await page.goto(
    `${projectIssuesAddress("WEB")}?s=${encodeURIComponent("title contains nothing")}`,
  );
  await expect(page.getByText(t("search.found-none"))).toBeVisible();
  await listInFourThemes(page, "empty");
  await page.route("**/api/v1/search/issues/count*", (route) =>
    route.fulfill({
      status: 408,
      contentType: "application/problem+json",
      body: JSON.stringify({
        code: "search.too-slow",
        message_key: "error.search.too-slow",
        params: {},
        status: 408,
        title: "This search took too long.",
        type: "urn:bugaboo:error:search.too-slow",
      }),
    }),
  );
  await page.route("**/api/v1/search/issues/groups*", (route) =>
    route.fulfill({
      status: 408,
      contentType: "application/problem+json",
      body: JSON.stringify({
        code: "search.too-slow",
        message_key: "error.search.too-slow",
        params: {},
        status: 408,
        title: "This search took too long.",
        type: "urn:bugaboo:error:search.too-slow",
      }),
    }),
  );
  await page.goto(projectIssuesAddress("WEB"));
  await expect(page.getByRole("alert")).toContainText(t("error.search.too-slow"));
  await listInFourThemes(page, "error");
});

test("iss_01 pictures: the list on the Search page", async ({ page }) => {
  await signIn(page, ana);
  await page.goto("/search");
  await expect(grid(page).getByRole("link", { name: webIssues.login })).toBeVisible();
  await listInFourThemes(page, "search-page");
});
