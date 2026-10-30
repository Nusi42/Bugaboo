import { t } from "@bugaboo/messages";
import { expect } from "@playwright/test";

import { test } from "./support/fixtures.ts";
import { signIn } from "./support/flows.ts";
import { layoutOf } from "./support/layout.ts";
import { sampleAdministrator } from "./support/sample.ts";
import { shadowsOf } from "./support/windows.ts";
import { projectIssuesAddress } from "../src/shared/paths.ts";

// M5 exit, package O: the sheet More of a phone opens at its top, with its title, and casts a shadow where more follows.

test.use({ sample: true });

test("ux_8 the sheet More opens at its top and has a shadow at its end", async ({ page }) => {
  test.skip(layoutOf(page) !== "phone", "only a phone has the sheet More");
  await signIn(page, sampleAdministrator());
  await page.goto(projectIssuesAddress("APP"));
  await page.getByRole("button", { name: t("shell.more"), exact: true }).click();
  const sheet = page.getByRole("dialog", { name: t("shell.more") });
  await expect(sheet.getByRole("heading", { name: t("shell.more"), level: 2 })).toBeInViewport();
  await expect.poll(() => sheet.evaluate((node) => node.scrollTop)).toBe(0);
  await expect.poll(() => shadowsOf(sheet)).toEqual({ start: 0, end: 1 });
});
