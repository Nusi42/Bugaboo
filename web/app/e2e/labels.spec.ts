import { t } from "@bugaboo/messages";
import { type Page } from "@playwright/test";

import { ada, bob } from "./support/accounts.ts";
import { expectGoodInEveryLook } from "./support/checks.ts";
import { expect, test } from "./support/fixtures.ts";
import { signIn } from "./support/flows.ts";
import { itemArea } from "./support/layout.ts";
import { detailRow, expectStatus, issueNote, prepareWeb, startWorking } from "./support/issues.ts";
import { addTag, putLabelOn, showDetails } from "./support/labels.ts";
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
  await page.getByRole("button", { name: t("labels.create-label-group"), exact: true }).click();
  await expect(page.getByText(t("labels.group-created", { name: "Area" }))).toBeVisible();
  await page.getByLabel(t("labels.field-label-name"), { exact: true }).fill("frontend");
  await page.getByRole("button", { name: t("labels.create-label"), exact: true }).click();
  await expect(page.getByText(t("labels.label-created", { name: "frontend" }))).toBeVisible();
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
  await addTag(page, "urgent");
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
  await addTag(page, "quick");
  await expect(
    listNamed(page, t("labels.tags-heading")).getByText("quick", { exact: true }),
  ).toBeVisible();
  // The tag counted the version of the issue up; the change of status is made on that version. The
  // row of the tag says *Saved* already, so the note of the status is what is waited for.
  await startWorking(page);
  await expect(issueNote(page)).toBeVisible();
  await page.reload();
  await expectStatus(page, t("status.in-progress"));
});

test("ux_4_4 Tags and Labels are rows of the one list of the Details and change in place with Saved and Undo (R7)", async ({
  page,
}) => {
  await signIn(page, ada);
  await page.goto(issueAddress("WEB-3"));
  await showDetails(page);
  // One list for all the rows of the Details: the own fields, following, tags and labels too.
  const details = page.getByRole("complementary", { name: t("issues.details"), exact: true });
  await expect(details.locator("dl")).toHaveCount(1);
  const tags = detailRow(page, t("labels.tags-heading"));
  await expect(tags).toBeVisible();
  await addTag(tags, "fresh");
  await expect(tags.getByText("fresh", { exact: true })).toBeVisible();
  // Saved, and Undo, in the row where it happened; Undo takes the tag off again.
  await expect(tags.getByText(new RegExp(`^${t("save.saved")}`, "u"))).toBeVisible();
  await expect(detailRow(page, t("labels.labels-heading")).getByText(t("save.saved"))).toHaveCount(
    0,
  );
  await tags.getByRole("button", { name: t("common.undo"), exact: true }).click();
  await expect(tags.getByText("fresh", { exact: true })).toHaveCount(0);
});

test("tag_01 a read of the tags and labels that failed says why in their rows", async ({
  page,
}) => {
  await page.route("**/api/v1/issues/*/labels-and-tags", (route) =>
    route.fulfill({
      status: 403,
      contentType: "application/problem+json",
      body: JSON.stringify({
        type: "urn:bugaboo:error:access.forbidden",
        title: "access.forbidden",
        status: 403,
        code: "access.forbidden",
        message_key: "error.access.forbidden",
        params: {},
      }),
    }),
  );
  await signIn(page, ada);
  await page.goto(issueAddress("WEB-1"));
  await showDetails(page);
  const tags = detailRow(page, t("labels.tags-heading"));
  await expect(tags.getByText(t("error.access.forbidden"))).toBeVisible();
  await expect(
    detailRow(page, t("labels.labels-heading")).getByText(t("error.access.forbidden")),
  ).toBeVisible();
});

test("tag_03 renaming a label shows which issues change before it is saved", async ({
  page,
  server,
}) => {
  await putLabelOn(server, "WEB-1", "frontend");
  await signIn(page, ada);
  await page.goto(paths.labels);
  await listNamed(page, t("labels.labels-search"))
    .getByRole("button", { name: "frontend", exact: true })
    .click();
  const panel = itemArea(page, "frontend");
  await panel.getByLabel(t("labels.field-name"), { exact: true }).fill("front end");
  await panel.getByRole("button", { name: t("labels.show-changes"), exact: true }).click();
  await expect(panel.getByText(t("preview.summary", { changed: 1, skipped: 0 }))).toBeVisible();
  await expect(panel.getByText("WEB-1", { exact: true })).toBeVisible();
  await expectGoodInEveryLook(page);
  await panel.getByRole("button", { name: t("common.save"), exact: true }).click();
  await expect(page.getByText(t("save.saved"), { exact: true })).toBeVisible();
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
