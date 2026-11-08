import { t } from "@bugaboo/messages";

import { addDays } from "../src/shared/format.ts";
import { weekdayName } from "../src/shared/clock.ts";
import { signedInAdmin } from "./support/access.ts";
import { bob, signInThroughApi } from "./support/accounts.ts";
import { expect, test } from "./support/fixtures.ts";
import { signIn } from "./support/flows.ts";
import { partIn } from "./support/parts.ts";
import { createIssue, prepareWeb } from "./support/issues.ts";
import {
  bookOn,
  cellOf,
  columnOf,
  openWeek,
  today,
  typeSaved,
  weekOf,
} from "./support/timesheet.ts";

// The windows of the timesheet that change what the cells hold in more detail (TIME-04, TIME-08):
// the breaks of a day, one line each, and the entries of a row, one by one, with the form every
// time entry has. They are on the grid of a computer; the phone has the same buttons in its day.

test.use({ preparation: { run: prepareWeb } });

test("time_08 the breaks of a day are listed one to a line and changed in their window", async ({
  page,
}) => {
  await signIn(page, bob);
  const column = columnOf(today());
  await openWeek(page, weekOf(today()));
  await typeSaved(page, cellOf(page, t("time.sheet.start"), column), "0:00");
  await typeSaved(page, cellOf(page, t("time.sheet.end"), column), "0:10");
  await typeSaved(page, cellOf(page, t("time.sheet.breaks"), column), "0:02-0:06");
  const opener = page.getByRole("button", {
    name: t("time.breaks.open-of", { day: weekdayName(today(), "long") }),
  });
  await opener.click();
  const window = page.getByRole("dialog");
  const lines = window.getByRole("textbox", { name: t("time.breaks.label") });
  // The window starts with the break there is, and the focus is in its box.
  await expect(lines).toHaveValue(/^12:02\sAM.12:06\sAM$/u);
  await expect(lines).toBeFocused();
  await lines.fill("0:01-0:02\n0:03-0:05");
  await window.getByRole("button", { name: t("time.breaks.save"), exact: true }).click();
  await expect(window).toHaveCount(0);
  await expect(cellOf(page, t("time.sheet.breaks"), column)).toHaveValue("0:03");
  await opener.click();
  await expect(window.getByRole("textbox", { name: t("time.breaks.label") })).toHaveValue(
    /^12:01\sAM.12:02\sAM\n12:03\sAM.12:05\sAM$/u,
  );
});

test("time_04 the details of a row list its entries and change one of them", async ({
  page,
  server,
}) => {
  const admin = await signedInAdmin(server);
  const issue = await createIssue(admin, "WEB", "Several entries on a day");
  await admin.dispose();
  const week = addDays(weekOf(today()), -21);
  const worker = await signInThroughApi(server, bob);
  await bookOn(worker, issue, week, 30);
  await bookOn(worker, issue, week, 20);
  await worker.dispose();
  await signIn(page, bob);
  await openWeek(page, week);
  // A cell of two entries shows their sum and is changed in the details.
  await expect(cellOf(page, issue.key, 0, false)).toHaveValue("0:50");
  await page
    .getByRole("button", { name: t("time.sheet.details-of", { issue: issue.key }) })
    .click();
  const window = page.getByRole("dialog");
  await expect(window.getByRole("button", { name: /^Change the time of/u })).toHaveCount(2);
  await window
    .getByRole("button", { name: /^Change the time of/u })
    .first()
    .click();
  // The same form as Add time: the length, the note and the fields of an entry.
  await window.getByRole("textbox", { name: t("time.entry.length") }).fill("1:00");
  await window.getByRole("textbox", { name: t("time.entry.note") }).fill("Longer than first noted");
  await window.getByRole("button", { name: t("time.details.save"), exact: true }).click();
  await expect(window.getByText(/Longer than first noted/u)).toBeVisible();
  // The window shows the list again once the change is saved; its footer says Close then, beside the
  // Close of the window's own corner.
  await expect(
    window.getByRole("button", { name: t("time.details.back"), exact: true }),
  ).toHaveCount(0);
  await partIn(window, "window.footer-actions")
    .getByRole("button", { name: t("common.close"), exact: true })
    .click();
  await page.reload();
  await expect(cellOf(page, issue.key, 0, false)).toHaveValue("1:20");
});

test("time_04 the focus goes into the form of an entry and back to its button", async ({
  page,
  server,
}) => {
  const admin = await signedInAdmin(server);
  const issue = await createIssue(admin, "WEB", "Focus in the details");
  await admin.dispose();
  const week = addDays(weekOf(today()), -35);
  const worker = await signInThroughApi(server, bob);
  await bookOn(worker, issue, week, 30);
  await worker.dispose();
  await signIn(page, bob);
  await openWeek(page, week);
  await page
    .getByRole("button", { name: t("time.sheet.details-of", { issue: issue.key }) })
    .click();
  const window = page.getByRole("dialog");
  // The window opens with the focus inside it, on the button of the one entry.
  const change = window.getByRole("button", { name: /^Change the time of/u });
  await change.click();
  await expect(window.getByRole("textbox", { name: t("time.entry.length") })).toBeFocused();
  await window.getByRole("button", { name: t("time.details.back"), exact: true }).click();
  await expect(change).toBeFocused();
});
