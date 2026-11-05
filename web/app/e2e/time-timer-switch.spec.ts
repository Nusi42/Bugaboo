import { t } from "@bugaboo/messages";

import { signedInAdmin } from "./support/access.ts";
import { ada } from "./support/accounts.ts";
import { expect, test } from "./support/fixtures.ts";
import { signIn } from "./support/flows.ts";
import { createIssue, prepareWeb } from "./support/issues.ts";
import { chipOf } from "./support/time-scene.ts";
import {
  clearTimer,
  entriesBookedOn,
  startedMinutesAgo,
  startTimerOn,
  timerRunsOn,
} from "./support/time.ts";

// *Switch issue* (TIME-01): the time so far is saved on the first issue and the timer goes on with the
// second, so one timer runs. A file of its own: an entry saved from a timer ends now, and the start of
// a later timer of the same person cannot be before it.

test.use({ preparation: { run: prepareWeb } });

test("time_01 switch issue from the timer keeps one running timer", async ({ page, server }) => {
  const admin = await signedInAdmin(server);
  const first = await createIssue(admin, "WEB", "Printer jams");
  const second = await createIssue(admin, "WEB", "Paper runs out");
  await startTimerOn(admin, first);
  await startedMinutesAgo(admin, 30);
  await signIn(page, ada);
  await page.goto("/");
  await chipOf(page, first.key).click();
  await page.getByRole("button", { name: t("time.timer.switch"), exact: true }).click();
  const window = page.getByRole("dialog", { name: t("time.switch.title") });
  await window.getByRole("searchbox").or(window.getByRole("textbox")).first().fill(second.key);
  await window
    .getByRole("button", { name: new RegExp(second.key, "u") })
    .first()
    .click();
  await expect(chipOf(page, second.key)).toBeVisible();
  await expect(chipOf(page, first.key)).toHaveCount(0);
  // The time so far went to the first issue, and exactly one timer runs.
  expect(await timerRunsOn(admin)).toBe(second.key);
  const [kept, ...others] = await entriesBookedOn(admin, first);
  expect(kept?.minutes).toBe(30);
  expect(others).toHaveLength(0);
  await clearTimer(admin);
  await admin.dispose();
});
