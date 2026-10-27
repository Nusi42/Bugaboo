import { t } from "@bugaboo/messages";
import { type Page } from "@playwright/test";

import { ada, ownId } from "./support/accounts.ts";
import { ana, lea, roleName, signedInAdmin } from "./support/access.ts";
import { expectGoodInEveryLook } from "./support/checks.ts";
import { expect, test } from "./support/fixtures.ts";
import { openIssueWithDetails, signIn } from "./support/flows.ts";
import {
  assignTo,
  startWorking,
  expectStatus,
  statusButton,
  holdDescription,
  issueOf,
  openIssuesIn,
  prepareWeb,
  webIssues,
  issueNote,
  rowSaved,
} from "./support/issues.ts";
import { onThisLayout } from "./support/layout.ts";
import { issueAddress, projectIssuesAddress } from "../src/shared/paths.ts";

// The issue page and the issue list of a project (ISS-01, UX-04, M3 WP-06): an issue opens in the
// focus view or the side panel, short values change in place, a Guest sees Someone, a field that
// cannot be changed says why, and the demo frames of M2 are gone.

test.use({ preparation: { run: prepareWeb } });
test.describe.configure({ mode: "serial" });

/** The title of the issue that opens first, which is the heading of the issue page. */
function loginHeading(page: Page) {
  return page.getByRole("heading", { name: new RegExp(webIssues.login, "u") });
}

test("ux_04 the frames page of M2 is gone", async ({ page }) => {
  await signIn(page, ada);
  await page.goto("/frames");
  await expect(
    page.getByRole("heading", { level: 1, name: t("page.not-found.title") }),
  ).toBeVisible();
});

test("iss_01 the issues of a project are listed and one opens in the focus view", async ({
  page,
}) => {
  await signIn(page, ada);
  await page.goto(projectIssuesAddress("WEB"));
  const list = page.getByRole("grid", { name: t("issues.list"), exact: true });
  await expect(list.getByRole("link")).toHaveCount(3);
  await list.getByRole("link", { name: webIssues.login }).click();
  await expect(page).toHaveURL(new RegExp(`${issueAddress("WEB-1")}$`, "u"));
  await expect(loginHeading(page)).toBeVisible();
  await expect(page.getByText("Steps:")).toBeVisible();
  await expectGoodInEveryLook(page);
});

test("iss_01 status and priority change in place and say Saved", async ({ page }) => {
  await signIn(page, ada);
  await page.goto(issueAddress("WEB-1"));
  await expect(loginHeading(page)).toBeVisible();
  await startWorking(page);
  await expect(issueNote(page)).toBeVisible();
  await openIssueWithDetails(page, "WEB-1");
  await page.getByLabel(t("issue-field.priority"), { exact: true }).selectOption({
    label: t("priority.high"),
  });
  await expect(rowSaved(page, t("issue-field.priority"))).toBeVisible();
  // Kept: the page is read again from the server.
  await openIssueWithDetails(page, "WEB-1");
  await expectStatus(page, t("status.in-progress"));
  await expect(page.getByLabel(t("issue-field.priority"), { exact: true })).toHaveValue("high");
});

test("ux_04 the issue opens in the side panel when the person chose that view", async ({
  page,
  server,
}) => {
  const admin = await signedInAdmin(server);
  await openIssuesIn(admin, "panel");
  await signIn(page, ada);
  await page.goto(projectIssuesAddress("WEB"));
  const list = page.getByRole("grid", { name: t("issues.list"), exact: true });
  await list.getByRole("link", { name: webIssues.passkey }).click();
  await expect(page).toHaveURL(/open=WEB-2$/u);
  const panel = page.getByRole("complementary", { name: "WEB-2", exact: true });
  await expect(panel).toBeVisible();
  await onThisLayout(page, {
    computer: async () => {
      await expect(list.getByRole("link", { name: webIssues.login })).toBeVisible();
    },
    tablet: async () => {
      await expect(panel).toBeVisible();
    },
    phone: async () => {
      await expect(list).toBeHidden();
    },
  });
  await expectGoodInEveryLook(page);
  await page.keyboard.press("Escape");
  await expect(panel).toHaveCount(0);
  await openIssuesIn(admin, "focus");
});

test("iss_01 a Viewer sees the fields with a lock and the reason, and cannot change them", async ({
  page,
}) => {
  await signIn(page, ana);
  await page.goto(issueAddress("WEB-1"));
  await expect(loginHeading(page)).toBeVisible();
  await expect(
    page.getByText(t("issues.read-only", { field: t("issue-field.status") })),
  ).toBeVisible();
  await expect(statusButton(page)).toBeDisabled();
});

test("iss_01 a Guest sees a role or Someone where names are not for them", async ({
  page,
  server,
}) => {
  const admin = await signedInAdmin(server);
  const typo = await issueOf(admin, "WEB-3");
  await assignTo(admin, typo, await ownId(admin), 1);
  await holdDescription(admin, typo);
  await signIn(page, lea);
  await openIssueWithDetails(page, "WEB-3");
  await expect(
    page.getByText(t("person.role", { role: roleName.administrator }), { exact: true }),
  ).toBeVisible();
  await page.getByRole("button", { name: t("issues.description.edit"), exact: true }).click();
  await expect(page.getByText(/^Someone is editing the description/u)).toBeVisible();
});
