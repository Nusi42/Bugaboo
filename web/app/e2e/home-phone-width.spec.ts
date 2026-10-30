import { expect } from "@playwright/test";

import { test } from "./support/fixtures.ts";
import { signIn } from "./support/flows.ts";
import { layoutOf } from "./support/layout.ts";
import { partIn } from "./support/parts.ts";
import { sampleAdministrator } from "./support/sample.ts";
import { boxOf } from "./support/windows.ts";
import { paths } from "../src/shared/paths.ts";

// M5 exit, package O: on a phone the words of a notification on the start page have most of the width of the screen: the
// card, the group of an issue and the notification were three boxes inside each other.

test.use({ sample: true });

/** The least share of the width of the screen that the lines of a notification have (the three boxes left 0.6 of it). */
const LEAST_SHARE = 0.65;

test("ux_8 the words of a notification have most of the width of a phone", async ({ page }) => {
  test.skip(layoutOf(page) !== "phone", "a computer has the room");
  await signIn(page, sampleAdministrator());
  await page.goto(paths.home);
  const words = partIn(page, "notification.sentences").first();
  await expect(words).toBeVisible();
  const screen = await page.evaluate(() => window.innerWidth);
  expect((await boxOf(words)).width / screen).toBeGreaterThanOrEqual(LEAST_SHARE);
});
