import { t } from "@bugaboo/messages";

import { projectIssuesAddress } from "../src/shared/paths.ts";
import { signedInAdmin } from "./support/access.ts";
import { ada } from "./support/accounts.ts";
import { expectGoodInEveryLook } from "./support/checks.ts";
import { expect, test } from "./support/fixtures.ts";
import { openIssueWithDetails, signIn } from "./support/flows.ts";
import { estimate, goalWithTask, totalsText } from "./support/issue-time.ts";
import { createIssue, prepareWeb, savedNote } from "./support/issues.ts";
import { addSubIssue } from "./support/relate.ts";

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
    page.getByText(totalsText("10h", "6h 40m", "3h 20m"), { exact: true }),
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

test("time_03 a sub-issue added on the page counts in the totals at once", async ({
  page,
  server,
}) => {
  const admin = await signedInAdmin(server);
  const goal = await goalWithTask(admin);
  const extra = await createIssue(admin, "WEB", "Test the e-mail text");
  await estimate(admin, extra, 120);
  await admin.dispose();
  await signIn(page, ada);
  await openIssueWithDetails(page, goal.key);
  await expect(
    page.getByText(totalsText("10h", "6h 40m", "3h 20m"), { exact: true }),
  ).toBeVisible();
  await addSubIssue(page, extra.key);
  // No reload: the totals with sub-issues are read again when the issue gets a sub-issue.
  await expect(
    page.getByText(totalsText("12h", "8h 40m", "3h 20m"), { exact: true }),
  ).toBeVisible();
});

test("time_03 a length is in the light weight of the code face, as heavy as the words around it", async ({
  page,
  server,
}) => {
  const admin = await signedInAdmin(server);
  const goal = await goalWithTask(admin);
  await admin.dispose();
  await signIn(page, ada);
  await openIssueWithDetails(page, goal.key);
  await expect(
    page.getByText(totalsText("10h", "6h 40m", "3h 20m"), { exact: true }),
  ).toBeVisible();
  // Cascadia Mono at the regular weight puts half as much ink again on the page as the text face:
  // a length set among words reads as bold unless it has the code weight of the theme.
  const looks = await page.evaluate(() => {
    const root = getComputedStyle(document.documentElement);
    const weights: string[] = [];
    for (const length of document.querySelectorAll('[data-part="length"]')) {
      weights.push(getComputedStyle(length).fontWeight);
    }
    return {
      code: root.getPropertyValue("--weight-code").trim(),
      regular: root.getPropertyValue("--weight-regular").trim(),
      lengths: weights,
    };
  });
  expect(Number(looks.code)).toBeLessThan(Number(looks.regular));
  expect(looks.lengths.length).toBeGreaterThan(0);
  expect(new Set(looks.lengths)).toEqual(new Set([looks.code]));
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

test("time_03 a list grouped by a length lists the issues of each range", async ({
  page,
  server,
}) => {
  const admin = await signedInAdmin(server);
  const goal = await goalWithTask(admin);
  await admin.dispose();
  await signIn(page, ada);
  const query = new URLSearchParams({ group: "estimated time" }).toString();
  await page.goto(`${projectIssuesAddress("WEB")}?${query}`);
  // The range of 4 to 8 hours holds the goal (6 hours) and lists it, like any other group.
  const range = t("issues.length-range", { from: "4h", to: "8h" });
  await expect(page.getByRole("row", { name: new RegExp(range, "u") })).toBeVisible();
  await expect(page.getByText(t("issues.group-unlisted"), { exact: true })).toHaveCount(0);
  await expect(page.getByRole("row", { name: new RegExp(goal.key, "u") }).first()).toBeVisible();
  // The issues without an estimate are a group with the name of what they lack. The range of the
  // goal is folded first: the other tests of this server give it more issues than a page holds.
  await page.getByRole("row", { name: new RegExp(`^${range},`, "u") }).click();
  await expect(
    page.getByRole("row", {
      name: new RegExp(t("issues.length-not-set", { field: t("issue-field.estimated-time") }), "u"),
    }),
  ).toBeVisible();
});
