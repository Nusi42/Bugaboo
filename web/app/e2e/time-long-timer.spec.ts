import { t } from "@bugaboo/messages";

import { ada } from "./support/accounts.ts";
import { expect, test } from "./support/fixtures.ts";
import { signIn } from "./support/flows.ts";
import { prepareWeb } from "./support/issues.ts";
import { chipOf, longScene } from "./support/time-scene.ts";

// A timer that runs longer than a work day shows its warning on the timer itself (UX §4.9): a mark in
// the button, and the words of the warning in its name, in the sidebar of a computer and at the top of
// the start page of a phone. The server marks it after a while, so the test waits for its job.

test.use({ preparation: { run: prepareWeb } });
test.setTimeout(180_000);

test("time_01 a timer that runs longer than a work day is marked on the timer", async ({
  page,
  server,
}) => {
  const { issue, dispose } = await longScene(server);
  await signIn(page, ada);
  await page.goto("/");
  const chip = chipOf(page, issue.key);
  await expect(chip).toBeVisible();
  // The mark is in the button, and the warning is in its name.
  await expect(chip.locator("svg")).toBeVisible();
  await expect(chip).toHaveAccessibleName(new RegExp(t("time.timer.runs-long"), "u"));
  // The sentence is shown too, beside the timer or over the page.
  await expect(
    page.getByRole("status").filter({ hasText: t("time.timer.runs-long") }),
  ).toBeVisible();
  await dispose();
});
