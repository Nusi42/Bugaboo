import { t } from "@bugaboo/messages";
import { expect, type Page } from "@playwright/test";

import { openNewIssueDialog } from "./issues.ts";
import { expectPage } from "./showcase-opens.ts";
import { ISSUE, PROJECT } from "./showcase-sample.ts";
import { type Needs } from "./showcase.ts";
import { titleBox, windowButton, windowDock } from "./windows.ts";
import { issueAddress, projectIssuesAddress } from "../../src/shared/paths.ts";

/** Opens the new-issue form over the list of issues of the project. */
export async function openNewIssue({ page }: Pick<Needs, "page">): Promise<void> {
  await page.goto(projectIssuesAddress(PROJECT));
  await expectPage(page);
  await openNewIssueDialog(page);
  await throwOldDraftAway(page);
}

/** A draft that an earlier theme left is thrown away, so the form of every theme starts empty (M5 exit, O11). */
async function throwOldDraftAway(page: Page): Promise<void> {
  const drop = page.getByRole("button", { name: t("editing.draft-drop"), exact: true });
  if ((await drop.count()) > 0) {
    await drop.click();
    await expect(drop).toHaveCount(0);
  }
}

/** Opens the new-issue window with a title typed and makes it as wide as the page: the fields move to the side. */
export async function openNewIssueWide({ page }: Needs): Promise<void> {
  await openNewIssue({ page });
  await titleBox(page).fill("Login fails after the password was reset");
  await windowButton(page, t("window.maximize")).click();
  await expect(windowButton(page, t("window.restore-down"))).toBeVisible();
}

/** Opens the new-issue window with a title typed and parks it as the bar at the bottom of the page. */
export async function openNewIssueMinimized({ page }: Needs): Promise<void> {
  await openNewIssue({ page });
  await titleBox(page).fill("Login fails after the password was reset");
  await windowButton(page, t("window.minimize")).click();
  await expect(windowDock(page)).toBeVisible();
}

/** Opens the details of an issue under its status, as a phone shows them. */
export async function openIssueDetails({ page }: Pick<Needs, "page">): Promise<void> {
  await page.goto(issueAddress(ISSUE));
  await expectPage(page);
  await page.getByRole("button", { name: t("issues.all-details"), exact: true }).click();
}
