import { t } from "@bugaboo/messages";
import { expect, type Page } from "@playwright/test";

import { bob, cleo } from "./accounts.ts";
import { inFourThemes } from "./four-themes.ts";
import { layoutOf } from "./layout.ts";
import {
  addSendToTesting,
  openSimpleToChange,
  requireFoundInVersion,
  tryMarkAsDone,
} from "./workflows-pages.ts";

/** The drawing of the workflow, on a computer; a phone has the list only. */
async function drawing(page: Page): Promise<void> {
  if (layoutOf(page) !== "computer") {
    return;
  }
  await page.getByRole("radio", { name: t("workflows.view-drawing"), exact: true }).check();
  await expect(page.getByRole("button", { name: /Open its rules/u }).first()).toBeVisible();
  await inFourThemes(page, "12-drawing");
}

/** The editor with a new status and status change, its rules, the drawing, and *Try it*. */
export async function shootEditor(page: Page): Promise<void> {
  await openSimpleToChange(page);
  await addSendToTesting(page);
  await inFourThemes(page, "10-editor-list");
  await requireFoundInVersion(page);
  await page
    .getByRole("button", { name: t("workflows.rules-open", { button: "Send to testing" }) })
    .click();
  await inFourThemes(page, "11-rules");
  await page.getByRole("button", { name: t("workflows.rules-done"), exact: true }).click();
  await drawing(page);
  await page.getByRole("button", { name: t("workflows.publish"), exact: true }).click();
  await expect(page.getByText(t("save.saved"), { exact: true })).toBeVisible();
  const markAsDone = await tryMarkAsDone(page, "WEB-1", cleo);
  await expect(markAsDone).toContainText(t("workflows.try-not-for-you"));
  await inFourThemes(page, "13-try-it");
  await shootWhoCanDoIt(page);
  await shootNamesTaken(page);
}

/** *Mark as done* is for Administrators only: *Try it* for Bob says that *Who can do it* does not name him. */
async function shootWhoCanDoIt(page: Page): Promise<void> {
  await page
    .getByRole("button", { name: t("workflows.rules-open", { button: "Mark as done" }) })
    .first()
    .click();
  await page.getByRole("checkbox", { name: "Administrator", exact: true }).check({ force: true });
  await page.getByRole("button", { name: t("workflows.rules-done"), exact: true }).click();
  await page.getByRole("button", { name: t("workflows.publish"), exact: true }).click();
  await expect(page.getByText(t("save.saved"), { exact: true })).toBeVisible();
  const markAsDone = await tryMarkAsDone(page, "WEB-1", bob);
  await expect(markAsDone).toContainText(t("workflows.try-not-named"));
  await inFourThemes(page, "14-try-it-not-named");
}

/** A status name another status has: said at the field; and a status renamed to one, which Publish names. */
async function shootNamesTaken(page: Page): Promise<void> {
  const added = page.getByLabel(t("workflows.new-status"), { exact: true });
  await added.fill("in   PROGRESS");
  await expect(page.getByText(t("workflows.status-name-taken"))).toBeVisible();
  await inFourThemes(page, "15-name-taken");
  await added.fill("");
  await page.getByLabel(t("workflows.status-name"), { exact: true }).first().fill("done");
  await expect(page.getByText(t("workflows.publish-name-twice", { name: "done" }))).toBeVisible();
  await inFourThemes(page, "16-publish-blocked");
}
