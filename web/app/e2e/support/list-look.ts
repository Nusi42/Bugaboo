import { settingKeys, type SettingChoice } from "@bugaboo/api-client";
import { t } from "@bugaboo/messages";
import { expect, type Locator, type Page } from "@playwright/test";

import { projectIssuesAddress } from "../../src/shared/paths.ts";
import { signedInAdmin } from "./access.ts";
import { ada, signInThroughApi, type Person } from "./accounts.ts";
import { signIn } from "./flows.ts";
import { issueOf, prepareWeb, setStatusAndPriority, versionOf, webIssues } from "./issues.ts";
import { type Instance } from "./server.ts";
import { boxOf } from "./windows.ts";
export { inViewChoices } from "./search-bar.ts";

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

/**
 * Sets a person's *Spacing* (*My settings › Appearance*) through the API. Every list follows it: a
 * list and a saved search have no spacing of their own (ADR-0034, amendment).
 */
export async function chooseSpacing(
  server: Instance,
  spacing: SettingChoice<"appearanceSpacing">,
  person: Person = ada,
): Promise<void> {
  const session = await signInThroughApi(server, person);
  const answer = await session.client.PUT("/api/v1/me/settings/{key}", {
    params: { path: { key: settingKeys.appearanceSpacing } },
    body: { value: spacing },
  });
  await session.dispose();
  expect(answer.response.ok).toBe(true);
}

/** The frame of the list, whose variant says whether the rows are *Compact*. */
export function listFrame(page: Page): Locator {
  return page.locator("[data-part='list-frame']");
}

/** The list of the issues of the project, as a grid whose rows are drawn only where they are on screen. */
export function grid(page: Page): Locator {
  return page.getByRole("grid", { name: t("issues.list"), exact: true });
}

/** The button with the name of a column in the head of the list: pressing it sorts by the column. */
export function columnButton(page: Page, column: string): Locator {
  // The name for a screen reader changes with the order, so the button is found by the name it shows.
  return page.locator("[data-part='list-sort']").filter({ hasText: column });
}

/** The header of a group of rows, found by the name of the group. */
export function groupOf(page: Page, name: string): Locator {
  return grid(page).locator("[data-part='list-group']").filter({ hasText: name });
}

/** The titles of the issues in the rows, from the top. */
export function titlesInOrder(page: Page): Promise<string[]> {
  return grid(page).locator("[data-part='list-row.cell'][data-variant='title']").allInnerTexts();
}

/** The row of an issue in the list, found by its title. */
export function rowOf(page: Page, title: string): Locator {
  return grid(page).locator("[data-part='list-row']").filter({ hasText: title });
}

/** Signs in as the administrator and opens the list of the project WEB. */
export async function openList(page: Page): Promise<void> {
  await signIn(page, ada);
  await page.goto(projectIssuesAddress("WEB"));
  await expect(rowOf(page, webIssues.login)).toBeVisible();
}

/** Shows the Legend: it stands beside the toolbar on a computer, and opens from the Key button on a phone (D2). */
export async function showLegend(page: Page): Promise<Locator> {
  const legend = page.getByRole("button", { name: t("issues.legend-button"), exact: true });
  if ((await legend.count()) > 0) {
    await legend.click();
  }
  return page.getByRole("list", { name: t("issues.legend"), exact: true });
}

/** The place of a text among the rows and the group headers of the grid, from the top. */
export async function placeInGrid(page: Page, text: string): Promise<number> {
  const rows = await grid(page)
    .locator("[data-part='list-row'], [data-part='list-group']")
    .allInnerTexts();
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

/** The names of the columns in the head that are cut (a word that ends in an ellipsis), by their words. */
export function cutHeadNames(page: Page): Promise<(string | null)[]> {
  return page.evaluate(() => {
    const found: (string | null)[] = [];
    for (const label of document.querySelectorAll('[data-part="list-sort.label"]')) {
      if (label.scrollWidth > label.clientWidth) {
        found.push(label.textContent);
      }
    }
    return found;
  });
}
