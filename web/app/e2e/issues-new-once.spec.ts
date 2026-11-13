import { t } from "@bugaboo/messages";
import { type Page } from "@playwright/test";

import { ada } from "./support/accounts.ts";
import { expect, test } from "./support/fixtures.ts";
import { signIn } from "./support/flows.ts";
import {
  newIssueDialog as dialog,
  openNewIssueDialog,
  prepareWeb,
  titleLabel,
} from "./support/issues.ts";
import { countRequests } from "./support/requests.ts";
import { holdUntilReleased } from "./support/slow.ts";
import { projectIssuesAddress } from "../src/shared/paths.ts";

// An issue is made once (ISS-11, M8 WP-13): after *Create the issue* the window reads the new issue
// before it opens it, and waits for that as it waited for the issue, so a second press makes nothing.

test.use({ preparation: { run: prepareWeb } });
test.describe.configure({ mode: "serial" });

/** Opens the window over the list of WEB with a title in it, and holds the read of `key`, the issue it makes. */
async function readyToCreate(page: Page, key: string, title: string) {
  await signIn(page, ada);
  await page.goto(projectIssuesAddress("WEB"));
  await openNewIssueDialog(page);
  await dialog(page).getByLabel(titleLabel(), { exact: true }).fill(title);
  // Its read waits until it is let go.
  return holdUntilReleased(page, `**/api/v1/issues/${key}`);
}

test("iss_11 a second press while the new issue is read before it opens makes no second issue", async ({
  page,
}) => {
  // WEB-1 to WEB-3 are there: the new issue is WEB-4.
  const held = await readyToCreate(page, "WEB-4", "Only once");
  const made = countRequests(page, "POST", "/api/v1/projects/WEB/issues");
  const create = dialog(page).getByRole("button", { name: t("issues.create"), exact: true });
  await create.click();
  // The issue is made, and the window reads it before it opens it: now the second press comes.
  await held.asked;
  await create.dispatchEvent("click");
  held.release();
  await expect(page.getByRole("heading", { name: /Only once/u })).toBeVisible();
  expect(made()).toBe(1);
});

test("iss_11 Enter in the title while the new issue is read before it opens makes no second issue", async ({
  page,
  browserName,
}) => {
  test.skip(
    browserName === "webkit",
    "WebKit sends no form on Enter when its button stands outside it (M8-F126): no second Enter to hold",
  );
  // After the test before, the new issue is WEB-5.
  const held = await readyToCreate(page, "WEB-5", "Once with Enter");
  const made = countRequests(page, "POST", "/api/v1/projects/WEB/issues");
  const title = dialog(page).getByLabel(titleLabel(), { exact: true });
  await title.press("Enter");
  await held.asked;
  await title.press("Enter");
  held.release();
  await expect(page.getByRole("heading", { name: /Once with Enter/u })).toBeVisible();
  expect(made()).toBe(1);
});
