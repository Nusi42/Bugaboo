import { t } from "@bugaboo/messages";
import { type Page } from "@playwright/test";

import { ada, bob } from "./support/accounts.ts";
import { signedInAdmin } from "./support/access.ts";
import { expectGoodInEveryLook } from "./support/checks.ts";
import { expect, test } from "./support/fixtures.ts";
import { signIn } from "./support/flows.ts";
import { prepareWeb } from "./support/issues.ts";
import { labelIdOf, showDetails } from "./support/labels.ts";
import { issueAddress, paths } from "../src/shared/paths.ts";

// Labels and tags (TAG-01, TAG-02, TAG-03, M3 WP-08): the page of the people who look after them,
// the tags and labels on an issue, and renaming with a preview of what it does to the issues.

test.use({ preparation: { run: prepareWeb } });
test.describe.configure({ mode: "serial" });

/** The list of one kind on the page of labels and tags. */
function listNamed(page: Page, name: string) {
  return page.getByRole("list", { name, exact: true });
}

test("tag_02 a group and a label are made on the page, and the label is put on an issue", async ({
  page,
}) => {
  await signIn(page, ada);
  await page.goto(paths.labels);
  await expect(page.getByRole("heading", { level: 1, name: t("nav.labels") })).toBeVisible();
  await page.getByLabel(t("labels.field-group-name"), { exact: true }).fill("Area");
  await page.getByRole("button", { name: t("labels.make-group"), exact: true }).click();
  await expect(page.getByText(t("labels.group-made", { name: "Area" }))).toBeVisible();
  await page.getByLabel(t("labels.field-label-name"), { exact: true }).fill("frontend");
  await page.getByRole("button", { name: t("labels.make-label"), exact: true }).click();
  await expect(page.getByText(t("labels.label-made", { name: "frontend" }))).toBeVisible();
  await expect(
    listNamed(page, t("labels.labels-search")).getByRole("button", { name: "frontend" }),
  ).toBeVisible();

  await page.goto(issueAddress("WEB-1"));
  await showDetails(page);
  await page.getByLabel(t("labels.add-label"), { exact: true }).selectOption({ label: "frontend" });
  await page.getByRole("button", { name: t("labels.add-label-button"), exact: true }).click();
  await expect(
    listNamed(page, t("labels.labels-heading")).getByText("frontend", { exact: true }),
  ).toBeVisible();
  // Kept: the page is read again from the server.
  await page.reload();
  await showDetails(page);
  await expect(
    listNamed(page, t("labels.labels-heading")).getByText("frontend", { exact: true }),
  ).toBeVisible();
  await page
    .getByRole("button", { name: t("labels.take-off", { name: "frontend" }), exact: true })
    .click();
  await expect(page.getByText(t("labels.none-labels"))).toBeVisible();
});

test("tag_01 a tag is made by adding it to an issue and found on the page of tags", async ({
  page,
}) => {
  await signIn(page, ada);
  await page.goto(issueAddress("WEB-2"));
  await showDetails(page);
  await page.getByLabel(t("labels.add-tag"), { exact: true }).fill("urgent");
  await page.getByRole("button", { name: t("labels.add-tag-button"), exact: true }).click();
  await expect(
    listNamed(page, t("labels.tags-heading")).getByText("urgent", { exact: true }),
  ).toBeVisible();
  await page.goto(paths.labels);
  await expect(
    listNamed(page, t("labels.tags-search")).getByRole("button", { name: "urgent" }),
  ).toBeVisible();
});

test("tag_01 a status change right after a tag is added does not conflict", async ({ page }) => {
  await signIn(page, ada);
  await page.goto(issueAddress("WEB-3"));
  await showDetails(page);
  await page.getByLabel(t("labels.add-tag"), { exact: true }).fill("quick");
  await page.getByRole("button", { name: t("labels.add-tag-button"), exact: true }).click();
  await expect(
    listNamed(page, t("labels.tags-heading")).getByText("quick", { exact: true }),
  ).toBeVisible();
  // The tag counted the version of the issue up; the change of status is made on that version.
  await page.getByLabel(t("issue-field.status"), { exact: true }).selectOption({
    label: t("status.in-progress"),
  });
  await expect(page.getByText(/^Saved/u).first()).toBeVisible();
  await page.reload();
  await expect(page.getByLabel(t("issue-field.status"), { exact: true })).toHaveValue(
    "in-progress",
  );
});

test("tag_03 renaming a label shows which issues change before it is saved", async ({
  page,
  server,
}) => {
  const admin = await signedInAdmin(server);
  await admin.client.POST("/api/v1/issues/{issue}/labels", {
    params: { path: { issue: "WEB-1" } },
    body: { label: await labelIdOf(admin, "frontend") },
  });
  await admin.dispose();
  await signIn(page, ada);
  await page.goto(paths.labels);
  await listNamed(page, t("labels.labels-search"))
    .getByRole("button", { name: "frontend", exact: true })
    .click();
  const panel = page.getByRole("complementary", { name: "frontend" });
  await panel.getByLabel(t("labels.field-name"), { exact: true }).fill("front end");
  await panel.getByRole("button", { name: t("labels.show-changes"), exact: true }).click();
  await expect(panel.getByText(t("labels.preview-summary", { changed: 1 }))).toBeVisible();
  await expect(panel.getByText("WEB-1", { exact: true })).toBeVisible();
  await expectGoodInEveryLook(page);
  await panel.getByRole("button", { name: t("labels.save"), exact: true }).click();
  await expect(page.getByText(t("labels.saved"))).toBeVisible();
  await page.goto(paths.labels);
  await expect(
    listNamed(page, t("labels.labels-search")).getByRole("button", { name: "front end" }),
  ).toBeVisible();
});

test("tag_02 the page of labels and tags is good in every theme", async ({ page }) => {
  await signIn(page, ada);
  await page.goto(paths.labels);
  await expect(listNamed(page, t("labels.tags-search"))).toBeVisible();
  await expectGoodInEveryLook(page);
});

test("tag_02 a person who may not look after labels does not find the page", async ({ page }) => {
  await signIn(page, bob);
  await page.goto(paths.labels);
  await expect(
    page.getByRole("heading", { level: 1, name: t("page.not-found.title") }),
  ).toBeVisible();
});
