import { t } from "@bugaboo/messages";

import { signedInAdmin } from "./support/access.ts";
import { ada } from "./support/accounts.ts";
import { expect, test } from "./support/fixtures.ts";
import { signIn } from "./support/flows.ts";
import { createIssue, prepareWeb } from "./support/issues.ts";
import { chipOf } from "./support/time-scene.ts";
import {
  choiceNamed,
  entriesBookedOn,
  setTimeDefault,
  startedMinutesAgo,
  starterField,
  startTimerOn,
} from "./support/time.ts";

// The timer on screen (TIME-01): one timer per person that every device shows, *Stop timer* with the
// fields of the entry filled in from the issue, and *Switch issue* that keeps one timer running.

test.use({ preparation: { run: prepareWeb } });

test("time_01 stop timer shows activity and cost center from the issue", async ({
  page,
  server,
}) => {
  const admin = await signedInAdmin(server);
  const issue = await createIssue(admin, "WEB", "Login fails after password reset");
  const activity = await starterField(admin, "activity");
  const costCenter = await starterField(admin, "cost-center");
  const testing = await choiceNamed(admin, activity, "Testing");
  const product = await choiceNamed(admin, costCenter, "Product development", "4100");
  await setTimeDefault(admin, issue, activity, testing);
  await setTimeDefault(admin, issue, costCenter, product);
  await startTimerOn(admin, issue);
  await startedMinutesAgo(admin, 42);
  await signIn(page, ada);
  await page.goto("/");
  await chipOf(page, issue.key).click();
  await page.getByRole("button", { name: t("time.timer.stop"), exact: true }).click();
  const window = page.getByRole("dialog", { name: t("time.stop.title", { issue: issue.key }) });
  await expect(window).toBeVisible();
  await expect(window).toContainText(
    t("time.stop.worked", { length: "42 minutes", title: "Login fails after password reset" }),
  );
  // The boxes start with the defaults of the issue, which the server worked out.
  await expect(window.getByLabel("Activity", { exact: true })).toHaveValue(testing);
  await expect(window.getByLabel("Cost center", { exact: true })).toHaveValue(product);
  await window.getByLabel(t("time.entry.note"), { exact: true }).fill("Found the cause");
  await window.getByRole("button", { name: t("time.stop.save"), exact: true }).click();
  await expect(window).toHaveCount(0);
  await expect(chipOf(page, issue.key)).toHaveCount(0);
  await expect(
    page.getByText(t("time.timer.saved", { length: "42m", issue: issue.key }), { exact: true }),
  ).toBeVisible();
  const [saved] = await entriesBookedOn(admin, issue);
  expect(saved?.minutes).toBe(42);
  expect(saved?.note).toBe("Found the cause");
  expect(saved?.fields[activity]).toBe(testing);
  await admin.dispose();
});
