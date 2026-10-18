import { t } from "@bugaboo/messages";
import { expect, type Page } from "@playwright/test";

import { issueAddress } from "../../src/shared/paths.ts";
import { goodInFourThemes } from "./demo.ts";
import { BUG, DEMO_M4, made } from "./demo-m4-state.ts";
import { type SamplePerson } from "./sample.ts";

// What the steps of the exit demo of M4 do on the page of the bug (`m4-exit-demo.spec.ts`).

/** The pictures of a step go to the folder of the demo, one for each theme. */
export function good(page: Page, step: string): Promise<void> {
  return goodInFourThemes(page, step, DEMO_M4);
}

/** Opens the bug. */
export async function openTheBug(page: Page): Promise<void> {
  await page.goto(issueAddress(made.key));
  await expect(page.getByRole("heading", { name: BUG })).toBeVisible();
}

/** Picks a person in the picker that *Assign someone* opens in the window with this title. */
export async function assign(page: Page, window: string, person: SamplePerson): Promise<void> {
  await page
    .getByRole("dialog", { name: window })
    .getByRole("button", { name: t("issues.assign-someone") })
    .click();
  const picker = page.getByRole("dialog", { name: t("issues.assign-title") });
  await picker.getByLabel(t("issues.assign-search"), { exact: true }).fill(person.name);
  await picker
    .getByRole("button", { name: t("issues.assign-name", { name: person.name }), exact: true })
    .click();
  await picker.getByRole("button", { name: t("common.done"), exact: true }).click();
}
