import { t } from "@bugaboo/messages";

import { issueAddress } from "../src/shared/paths.ts";
import { ada } from "./support/accounts.ts";
import { expect, test } from "./support/fixtures.ts";
import { signIn } from "./support/flows.ts";
import { prepareWeb } from "./support/issues.ts";
import { longScene } from "./support/time-scene.ts";

// The warning of a timer that runs longer than a work day is also in the bar of the issue it runs
// on (UX §3.3, §4.9): a warning mark in the place of the timer, and its words in the name. A spec for
// a phone.

test.use({ preparation: { run: prepareWeb } });
test.setTimeout(180_000);

test("time_01 the bar of the issue marks a timer that runs longer than a work day", async ({
  page,
  server,
}) => {
  const { issue, dispose } = await longScene(server);
  await signIn(page, ada);
  await page.goto(issueAddress(issue.key));
  const place = page.getByRole("button", {
    name: new RegExp(`^${t("time.timer.stop-on", { issue: issue.key })}`, "u"),
  });
  await expect(place).toBeVisible();
  await expect(place).toHaveAccessibleName(new RegExp(t("time.timer.runs-long"), "u"));
  await expect(place.locator("svg")).toBeVisible();
  await dispose();
});
