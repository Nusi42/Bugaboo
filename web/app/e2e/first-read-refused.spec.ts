import type { Page } from "@playwright/test";

import { t } from "@bugaboo/messages";

import { ada } from "./support/accounts.ts";
import { expect, refusal, test } from "./support/fixtures.ts";
import { signIn } from "./support/flows.ts";
import { prepareWeb } from "./support/issues.ts";
import { layoutOf } from "./support/layout.ts";
import { holdUntilReleased } from "./support/slow.ts";
import { issueAddress, paths, projectIssuesAddress } from "../src/shared/paths.ts";

// A screen that holds its parts until their first reads are in (UX §6.4) stops holding when a read is refused: the
// screen shows, and says what went wrong, never stays empty. The links of the menu are never held.

test.use({ preparation: { run: prepareWeb } });

/** What the person reads when the server refused a read of theirs. */
const REFUSED = t("error.access.forbidden");

/** Refuses every read of the API at `path` (with any query), as a server that says no. */
async function refuseReadsOf(page: Page, path: string): Promise<void> {
  await page.route(new RegExp(`^https://[^/]+${path}([?].*)?$`, "u"), (route) =>
    route.fulfill(refusal(403, "access.forbidden")),
  );
}

/** The refusal on the screen, where the part that could not be read stands. */
function refusalShown(page: Page) {
  return page.getByText(REFUSED).first();
}

test("ux_01 the timesheet shows, and says so, when the booked week cannot be read", async ({
  page,
}) => {
  await signIn(page, ada);
  await refuseReadsOf(page, "/api/v1/me/work-days/booked");
  await page.goto(paths.time);
  await expect(refusalShown(page)).toBeVisible();
  await expect(page.getByRole("heading", { level: 1, name: t("time.sheet.title") })).toBeVisible();
});

test("ux_01 the labels and tags show, and say so, when the labels cannot be read", async ({
  page,
}) => {
  await signIn(page, ada);
  await refuseReadsOf(page, "/api/v1/labels");
  await page.goto(paths.labels);
  await expect(refusalShown(page)).toBeVisible();
  await expect(page.getByRole("heading", { level: 1, name: t("nav.labels") })).toBeVisible();
});

test("ux_01 the list of a project shows, and says so, when its rows cannot be read", async ({
  page,
}) => {
  await signIn(page, ada);
  await refuseReadsOf(page, "/api/v1/search/issues");
  await page.goto(projectIssuesAddress("WEB"));
  await expect(refusalShown(page)).toBeVisible();
});

test("ux_01 an issue shows its sections, and says so, when its related issues cannot be read", async ({
  page,
}) => {
  await signIn(page, ada);
  await refuseReadsOf(page, "/api/v1/issues/[^/]+/related-issues");
  await page.goto(issueAddress("WEB-1"));
  await expect(refusalShown(page)).toBeVisible();
  await expect(page.getByRole("heading", { name: t("history.title") })).toBeVisible();
});

test("ux_01 the links of the menu can be used while its sections are still read", async ({
  page,
}) => {
  test.skip(layoutOf(page) !== "computer", "Only a computer has the menu at its side.");
  await signIn(page, ada);
  const held = await holdUntilReleased(page, "**/api/v1/projects?*");
  await page.goto(paths.home);
  await held.asked;
  // The project tree waits for its read; the links of the menu do not.
  const settings = page
    .getByRole("navigation", { name: t("shell.menu") })
    .getByRole("link", { name: t("nav.organization-settings"), exact: true });
  await settings.focus();
  await expect(settings).toBeFocused();
  held.release();
});
