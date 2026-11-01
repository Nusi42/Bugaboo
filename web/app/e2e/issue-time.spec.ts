import { t } from "@bugaboo/messages";

import { projectIssuesAddress } from "../src/shared/paths.ts";
import { signedInAdmin } from "./support/access.ts";
import { ada } from "./support/accounts.ts";
import { expectGoodInEveryLook } from "./support/checks.ts";
import { expect, test } from "./support/fixtures.ts";
import { openIssueWithDetails, signIn } from "./support/flows.ts";
import { goalWithTask } from "./support/issue-time.ts";
import { prepareWeb, savedNote } from "./support/issues.ts";

// The lengths of time of an issue in its Details and the totals of a project (TIME-03, ADR-0035
// decision 4): Estimated time and Time left change in place, a Time left nobody typed is worked out
// and said quietly, Time spent is read, and the totals with the sub-issues and of the project are
// in the words of the short form.

test.use({ preparation: { run: prepareWeb } });

test("time_03 the Details show the estimate, the worked-out time left and the time spent", async ({
  page,
  server,
}) => {
  const admin = await signedInAdmin(server);
  const goal = await goalWithTask(admin);
  await admin.dispose();
  await signIn(page, ada);
  await openIssueWithDetails(page, goal.key);
  const estimated = page.getByLabel(t("issue-field.estimated-time"), { exact: true });
  await expect(estimated).toHaveValue("6h");
  await expect(
    page.getByText(t("issues.time-left-worked-out", { length: "4h 40m" }), { exact: true }),
  ).toBeVisible();
  await expect(page.getByRole("definition").filter({ hasText: "1h 20m" })).toBeVisible();
  await expect(
    page.getByText(
      `${t("issues.total-estimated", { length: "10h" })} · ${t("issues.total-left", { length: "6h 40m" })} · ${t("issues.total-spent", { length: "3h 20m" })}`,
      { exact: true },
    ),
  ).toBeVisible();
  // The history says the time booked in words, and no empty sentence (WP-05 sends it).
  await expect(page.getByText("Ada Admin booked 1h 20m", { exact: true })).toBeVisible();
  await expect(page.getByText("Ada Admin changed", { exact: true })).toHaveCount(0);
  // A typed time left counts until it is cleared: the quiet worked-out value goes.
  const left = page.getByLabel(t("issue-field.time-left"), { exact: true });
  await left.fill("1:30");
  await left.press("Enter");
  await expect(savedNote(page)).toBeVisible();
  await expect(page.getByText(t("issues.time-left-worked-out", { length: "4h 40m" }))).toHaveCount(
    0,
  );
  await expectGoodInEveryLook(page);
});

test("time_03 the list of a project says the time in the project", async ({ page, server }) => {
  const admin = await signedInAdmin(server);
  await goalWithTask(admin);
  await admin.dispose();
  await signIn(page, ada);
  await page.goto(projectIssuesAddress("WEB"));
  // Other tests of this server book time in WEB too: the line says the totals, whatever they are.
  const prefix = t("issues.project-time");
  const line = page.getByText(prefix, { exact: false });
  await expect(line).toBeVisible();
  await expect(line).toContainText(t("issues.total-spent", { length: "" }).trim());
});
