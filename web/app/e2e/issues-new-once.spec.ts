import { t } from "@bugaboo/messages";

import { ada } from "./support/accounts.ts";
import { expect, test } from "./support/fixtures.ts";
import { signIn } from "./support/flows.ts";
import {
  newIssueDialog as dialog,
  openNewIssueDialog,
  prepareWeb,
  titleLabel,
} from "./support/issues.ts";
import { answerTo, countRequests } from "./support/requests.ts";
import { holdUntilReleased } from "./support/slow.ts";
import { projectIssuesAddress } from "../src/shared/paths.ts";

// An issue is made once (ISS-11, M8 WP-13): after *Create the issue* the window reads the new issue
// before it opens it, and waits for that as it waited for the issue, so a second press makes nothing.

test.use({ preparation: { run: prepareWeb } });

test("iss_11 a second press while the new issue is read before it opens makes no second issue", async ({
  page,
}) => {
  await signIn(page, ada);
  await page.goto(projectIssuesAddress("WEB"));
  await openNewIssueDialog(page);
  await dialog(page).getByLabel(titleLabel(), { exact: true }).fill("Only once");
  // WEB-1 to WEB-3 are there: the new issue is WEB-4, and its read waits until it is let go.
  const release = await holdUntilReleased(page, "**/api/v1/issues/WEB-4");
  const made = countRequests(page, "POST", "/api/v1/projects/WEB/issues");
  const create = dialog(page).getByRole("button", { name: t("issues.create"), exact: true });
  const answered = answerTo(page, "POST", "/api/v1/projects/WEB/issues");
  await create.click();
  await answered;
  await create.dispatchEvent("click");
  release();
  await expect(page.getByRole("heading", { name: /Only once/u })).toBeVisible();
  expect(made()).toBe(1);
});
