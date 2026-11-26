import { t } from "@bugaboo/messages";

import { dayText, formatCalendarDay } from "../src/shared/format.ts";
import { ada } from "./support/accounts.ts";
import { expect, test } from "./support/fixtures.ts";
import { signIn } from "./support/flows.ts";
import { dayFromToday } from "./support/person-day.ts";
import {
  changeButton,
  openReleases,
  prepareReleases,
  releases,
  rowOf,
} from "./support/releases.ts";

// More of the page *Releases* (ISS-08, ADR-0049): the count and its search, changing the name and the day, and the
// way back to *Planned*, which the server decides (a reason only after the release was out).

test.use({ preparation: { run: prepareReleases } });

const planned = t("release-state.planned");
const released = t("release-state.released");

test("iss_08_the_count_of_a_release_opens_its_issues_as_a_search", async ({ page }) => {
  await signIn(page, ada);
  await openReleases(page);
  const row = rowOf(page, planned, releases.named);
  await expect(
    row.getByRole("link", { name: t("releases.issues-link", { count: 1 }) }),
  ).toBeVisible();
  await expect(rowOf(page, planned, releases.open).getByText("0", { exact: true })).toBeVisible();
  await row.getByRole("link", { name: t("releases.issues-link", { count: 1 }) }).click();
  await expect(page).toHaveURL(/planned/u);
  await expect(page.getByText("WEB-1", { exact: true }).first()).toBeVisible();
});

test("iss_08_change_saves_the_name_and_the_release_date", async ({ page }) => {
  const day = dayFromToday(40);
  await signIn(page, ada);
  await openReleases(page);
  await changeButton(page, releases.open).click();
  const window = page.getByRole("dialog");
  await window.getByLabel(t("releases.name")).fill("2.5.1");
  await window.getByLabel(t("releases.release-date")).fill(dayText(day));
  await window.getByRole("button", { name: t("releases.save") }).click();
  await expect(rowOf(page, planned, "2.5.1")).toContainText(formatCalendarDay(day));
});

test("iss_08_back_to_planned_needs_a_reason_only_after_the_release_was_out", async ({ page }) => {
  await signIn(page, ada);
  await openReleases(page);
  // Retired before it was ever out: no reason.
  await page
    .getByRole("button", { name: new RegExp(`^${t("release-state.retired")}`, "u") })
    .click();
  await changeButton(page, releases.retired).click();
  await page.getByRole("button", { name: t("releases.back-to-planned") }).click();
  await expect(rowOf(page, planned, releases.retired)).toBeVisible();
  // Out and then back: the server asks why, the window says so, and the reason lets it through.
  await changeButton(page, releases.named).click();
  await page.getByRole("button", { name: t("releases.mark-released") }).click();
  await expect(rowOf(page, released, releases.named)).toBeVisible();
  await changeButton(page, releases.named).click();
  const window = page.getByRole("dialog");
  await window.getByRole("button", { name: t("releases.back-to-planned") }).click();
  await expect(window.getByText(t("error.release.reason-needed"))).toBeVisible();
  await window.getByLabel(t("releases.reason")).fill("The date slipped");
  await window.getByRole("button", { name: t("releases.back-to-planned") }).click();
  await expect(rowOf(page, planned, releases.named)).toBeVisible();
});
