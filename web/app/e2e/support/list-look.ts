import { t } from "@bugaboo/messages";
import { expect, type Locator, type Page } from "@playwright/test";

import { projectIssuesAddress } from "../../src/shared/paths.ts";
import { signedInAdmin } from "./access.ts";
import { ada } from "./accounts.ts";
import { signIn } from "./flows.ts";
import { issueOf, prepareWeb, setStatusAndPriority, versionOf, webIssues } from "./issues.ts";
import { type Instance } from "./server.ts";
import { partIn } from "./parts.ts";
import { boxOf } from "./windows.ts";

/** The sample of the project WEB: one issue in progress and critical, one done and high, one plain. */
export async function prepareLook(server: Instance): Promise<void> {
  await prepareWeb(server);
  const admin = await signedInAdmin(server);
  const login = await issueOf(admin, "WEB-1");
  const passkey = await issueOf(admin, "WEB-2");
  await setStatusAndPriority(admin, login, await versionOf(admin, login), {
    status: "in-progress",
    priority: "critical",
  });
  await setStatusAndPriority(admin, passkey, await versionOf(admin, passkey), {
    status: "done",
    priority: "high",
  });
  await admin.dispose();
}

/** The list of the issues of the project, as a table. */
export function grid(page: Page): Locator {
  return page.getByRole("table", { name: t("issues.list"), exact: true });
}

/** The row of an issue in the list, found by its title. */
export function rowOf(page: Page, title: string): Locator {
  return partIn(grid(page), "list-row").filter({ hasText: title });
}

/** Signs in as the administrator and opens the list of the project WEB. */
export async function openList(page: Page): Promise<void> {
  await signIn(page, ada);
  await page.goto(projectIssuesAddress("WEB"));
  await expect(rowOf(page, webIssues.login)).toBeVisible();
}

/** The place of a text among the rows of the grid, from the top. */
export async function placeInGrid(page: Page, text: string): Promise<number> {
  const rows = await grid(page).getByRole("row").allInnerTexts();
  return rows.findIndex((row) => row.includes(text));
}

/**
 * Presses the middle of an element with the mouse, the way a person does. A row's link reaches
 * over the whole row (R8), so the browser test must not ask whether the element itself is hit.
 */
export async function pressMiddleOf(page: Page, element: Locator): Promise<void> {
  const box = await boxOf(element);
  await page.mouse.click(box.x + box.width / 2, box.y + box.height / 2);
}
