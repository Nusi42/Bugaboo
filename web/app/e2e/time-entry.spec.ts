import { dataOf } from "@bugaboo/api-client";
import { t } from "@bugaboo/messages";

import { signedInAdmin } from "./support/access.ts";
import { ada } from "./support/accounts.ts";
import { expectGoodInEveryLook } from "./support/checks.ts";
import { expect, test } from "./support/fixtures.ts";
import { openIssueWithDetails, signIn } from "./support/flows.ts";
import { createIssue, prepareWeb } from "./support/issues.ts";

// Booking time by hand (TIME-02): the window of *Add time* on the issue page takes a date, a length
// and a note, the fields of time entries start with the defaults of the issue, and what is saved is
// in the time of the issue and in its history.

test.use({ preparation: { run: prepareWeb } });

test("time_02 add time on the issue page books a length on a date", async ({ page, server }) => {
  const admin = await signedInAdmin(server);
  const issue = await createIssue(admin, "WEB", "Printer jams");
  await signIn(page, ada);
  await openIssueWithDetails(page, issue.key);
  await page.getByRole("button", { name: t("time.add.button"), exact: true }).click();
  const window = page.getByRole("dialog", { name: t("time.add.title", { issue: issue.key }) });
  await expect(window).toBeVisible();
  await expectGoodInEveryLook(page);
  await window.getByLabel(t("time.entry.date"), { exact: true }).fill("2027-01-14");
  await window.getByLabel(t("time.entry.length"), { exact: true }).fill("1:30");
  await window.getByLabel(t("time.entry.note"), { exact: true }).fill("Checked the rollers");
  await window.getByRole("button", { name: t("time.add.save"), exact: true }).click();
  await expect(window).toHaveCount(0);
  // The time of the issue and its history show it without a reload.
  await expect(page.getByRole("definition").filter({ hasText: "1h 30m" })).toBeVisible();
  await expect(page.getByText("Ada Admin booked 1h 30m", { exact: true })).toBeVisible();
  const mine = dataOf(
    await admin.client.GET("/api/v1/me/time-entries", {
      params: { query: { from: "2027-01-14", to: "2027-01-14" } },
    }),
  );
  // This server books nothing else on that date.
  const [booked] = mine.items;
  expect(booked?.readable_id).toBe(issue.key);
  expect(booked?.minutes).toBe(90);
  expect(booked?.note).toBe("Checked the rollers");
  await admin.dispose();
});

test("time_02 add time refuses a length that is none and says why", async ({ page, server }) => {
  const admin = await signedInAdmin(server);
  const issue = await createIssue(admin, "WEB", "Paper runs out");
  await admin.dispose();
  await signIn(page, ada);
  await openIssueWithDetails(page, issue.key);
  await page.getByRole("button", { name: t("time.add.button"), exact: true }).click();
  const window = page.getByRole("dialog");
  // Nothing can be saved before there is a length.
  await expect(
    window.getByRole("button", { name: t("time.add.save"), exact: true }),
  ).toBeDisabled();
  await window.getByLabel(t("time.entry.length"), { exact: true }).fill("soon");
  await expect(
    window.getByText(t("time.length.not-a-length", { decimal: "1.5" }), { exact: false }),
  ).toBeVisible();
  await expect(
    window.getByRole("button", { name: t("time.add.save"), exact: true }),
  ).toBeDisabled();
});
