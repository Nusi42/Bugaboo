import { t } from "@bugaboo/messages";
import { expect, type Page } from "@playwright/test";

import { ada } from "./accounts.ts";
import { signIn } from "./flows.ts";
import { paths } from "../../src/shared/paths.ts";

/** Signs in and opens *My settings › Appearance*. */
export async function openAppearance(page: Page): Promise<void> {
  await signIn(page, ada);
  await page.goto(paths.appearance);
  await expect(page.getByRole("heading", { level: 1, name: t("appearance.title") })).toBeVisible();
}

/** Turns a switch on or off by pressing its words, as a person does. */
export async function setSwitch(page: Page, name: string, on: boolean): Promise<void> {
  const field = page.getByRole("switch", { name, exact: true });
  if ((await field.isChecked()) !== on) {
    await page.getByText(name, { exact: true }).click();
  }
  await (on ? expect(field).toBeChecked() : expect(field).not.toBeChecked());
}
