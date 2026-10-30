import { expect } from "@playwright/test";

import { paths } from "../src/shared/paths.ts";
import { ada, createAdmin } from "./support/accounts.ts";
import { check } from "./support/access.ts";
import { test } from "./support/fixtures.ts";
import { signIn } from "./support/flows.ts";
import { layoutOf } from "./support/layout.ts";
import { partIn } from "./support/parts.ts";
import { boxOf } from "./support/windows.ts";

// M5 exit, the final pass: on a phone the answers of *Check access* have most of the width of the screen. The page,
// the card of the answer and each answer were three boxes inside each other and left the words about 0.6 of it; the
// card of the answer has no frame of its own on a phone, as on Home.

test.use({
  preparation: {
    run: async (server) => {
      const admin = await createAdmin(server);
      await admin.dispose();
    },
  },
});

/** The least share of the width of the screen that an answer has (three boxes left about 0.6 of it). */
const LEAST_SHARE = 0.75;

test("sec_03 the answers of Check access have most of the width of a phone", async ({ page }) => {
  test.skip(layoutOf(page) !== "phone", "a computer has the room");
  await signIn(page, ada);
  await page.goto(paths.checkAccess);
  await check(page, null, ada);
  const answer = partIn(page, "access.answer").first();
  await expect(answer).toBeVisible();
  const screen = await page.evaluate(() => window.innerWidth);
  expect((await boxOf(answer)).width / screen).toBeGreaterThanOrEqual(LEAST_SHARE);
});
