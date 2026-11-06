import { t } from "@bugaboo/messages";

import { weekdayName } from "../src/shared/clock.ts";
import { signedInAdmin } from "./support/access.ts";
import { bob, signInThroughApi } from "./support/accounts.ts";
import { expect, test } from "./support/fixtures.ts";
import { signIn } from "./support/flows.ts";
import { createIssue, prepareWeb } from "./support/issues.ts";
import { bookOn, columnOf, openWeek, today, typeSaved, weekOf } from "./support/timesheet.ts";
import { phoneCell, showDay } from "./support/timesheet-phone.ts";

// The windows of the timesheet on a phone: the breaks of the day that is shown, and the entries of
// a row of its time on issues (TIME-04, TIME-08).

test.use({ preparation: { run: prepareWeb } });

test("time_08 on a phone the breaks of the day are changed in their window", async ({ page }) => {
  await signIn(page, bob);
  const first = weekOf(today());
  const column = columnOf(today());
  await openWeek(page, first);
  await showDay(page, column);
  await typeSaved(page, phoneCell(page, first, t("time.sheet.start"), column), "0:00");
  await typeSaved(page, phoneCell(page, first, t("time.sheet.end"), column), "0:10");
  await page
    .getByRole("button", { name: t("time.breaks.open-of", { day: weekdayName(today(), "long") }) })
    .click();
  const window = page.getByRole("dialog");
  await expect(window.getByRole("textbox", { name: t("time.breaks.label") })).toBeFocused();
  await window.getByRole("textbox", { name: t("time.breaks.label") }).fill("0:02-0:06");
  await window.getByRole("button", { name: t("time.breaks.save"), exact: true }).click();
  await expect(window).toHaveCount(0);
  await expect(phoneCell(page, first, t("time.sheet.breaks"), column)).toHaveValue("0:04");
});

test("time_04 on a phone the details of a row open with its entries", async ({ page, server }) => {
  const admin = await signedInAdmin(server);
  const issue = await createIssue(admin, "WEB", "Details on a phone");
  await admin.dispose();
  const worker = await signInThroughApi(server, bob);
  await bookOn(worker, issue, today(), 30);
  await worker.dispose();
  await signIn(page, bob);
  const first = weekOf(today());
  await openWeek(page, first);
  await showDay(page, columnOf(today()));
  await page
    .getByRole("button", { name: t("time.sheet.details-of", { issue: issue.key }) })
    .click();
  await expect(
    page.getByRole("dialog").getByRole("button", { name: /^Change the time of/u }),
  ).toHaveCount(1);
});
