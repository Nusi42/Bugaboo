import { t } from "@bugaboo/messages";
import { type Page } from "@playwright/test";

import { issueAddress } from "../../src/shared/paths.ts";
import { inFourThemes } from "./four-themes.ts";
import { startWorking, statusButton } from "./issues.ts";
import { openSimpleToChange, removeStatus } from "./workflows-pages.ts";

/** The status button of an issue, *Ask for*, and the question of *Publish*. */
export async function shootStatusChanges(page: Page): Promise<void> {
  await page.goto(issueAddress("WEB-2"));
  await startWorking(page);
  await statusButton(page).click();
  await inFourThemes(page, "20-status-list");
  await page.getByRole("button", { name: "Send to testing", exact: true }).click();
  await inFourThemes(page, "21-ask-for");
  await openSimpleToChange(page);
  await removeStatus(page, "In progress");
  await page.getByRole("button", { name: t("workflows.publish"), exact: true }).click();
  await inFourThemes(page, "22-publish-question");
}
