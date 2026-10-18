import { t } from "@bugaboo/messages";
import { expect, type Page } from "@playwright/test";

import { cleo } from "./accounts.ts";
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
  await page
    .getByRole("button", { name: t("workflows.rules-done"), exact: true })
    .last()
    .click();
  await drawing(page);
  await page.getByRole("button", { name: t("workflows.publish"), exact: true }).click();
  await expect(page.getByText(t("save.saved"), { exact: true })).toBeVisible();
  const markAsDone = await tryMarkAsDone(page, "WEB-1", cleo);
  await expect(markAsDone).toContainText(t("error.workflow.not-for-you"));
  await inFourThemes(page, "13-try-it");
}
