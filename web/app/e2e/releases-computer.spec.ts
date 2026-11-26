import { t } from "@bugaboo/messages";

import { dayText } from "../src/shared/format.ts";
import { ana } from "./support/access.ts";
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
  tableOf,
} from "./support/releases.ts";

// The page *Releases* of a project on a computer (ISS-08, ADR-0049, UX §4.16): people with *Manage
// releases* make releases, mark them as released, retire and delete them; everybody else sees the page
// without changes.

test.use({ preparation: { run: prepareReleases } });

/** The words of the three states, as the page writes them. */
const state = {
  planned: t("release-state.planned"),
  released: t("release-state.released"),
  retired: t("release-state.retired"),
} as const;

test("iss_08_manage_releases_makes_marks_retires_and_deletes_on_the_releases_page", async ({
  page,
}) => {
  await signIn(page, ada);
  await openReleases(page);
  await expect(rowOf(page, state.planned, releases.named)).toBeVisible();
  // Make a release with its day.
  await page.getByRole("button", { name: t("releases.create") }).click();
  const window = page.getByRole("dialog", {
    name: t("releases.create-title", { project: "Website" }),
  });
  await window.getByLabel(t("releases.name")).fill("2.6");
  await window.getByLabel(t("releases.release-date")).fill(dayText(dayFromToday(30)));
  await window.getByRole("button", { name: t("releases.create-it") }).click();
  await expect(rowOf(page, state.planned, "2.6")).toBeVisible();
  // Mark it as released: it moves to the table of the released ones.
  await changeButton(page, "2.6").click();
  await page.getByRole("button", { name: t("releases.mark-released") }).click();
  await expect(rowOf(page, state.released, "2.6")).toBeVisible();
  await expect(rowOf(page, state.planned, "2.6")).toHaveCount(0);
  // Retire it: the retired ones are folded until they are asked for.
  await changeButton(page, "2.6").click();
  await page.getByRole("button", { name: t("releases.retire") }).click();
  await expect(tableOf(page, state.retired)).toHaveCount(0);
  await page.getByRole("button", { name: new RegExp(`^${state.retired}`, "u") }).click();
  await expect(rowOf(page, state.retired, "2.6")).toBeVisible();
  // Delete the release that no issue names: *Undo* brings it back.
  await changeButton(page, releases.open).click();
  await page.getByRole("button", { name: t("releases.delete") }).click();
  await expect(rowOf(page, state.planned, releases.open)).toHaveCount(0);
  await page.getByRole("button", { name: t("common.undo"), exact: true }).click();
  await expect(rowOf(page, state.planned, releases.open)).toBeVisible();
});

test("iss_08_a_refused_delete_offers_retire", async ({ page }) => {
  await signIn(page, ada);
  await openReleases(page);
  await changeButton(page, releases.named).click();
  await page.getByRole("button", { name: t("releases.delete") }).click();
  // The release is named by an issue: the window says to retire it, and names no issue.
  const window = page.getByRole("dialog");
  await expect(window.getByText(t("error.release.in-use"))).toBeVisible();
  await window.getByRole("button", { name: t("releases.retire") }).click();
  await expect(rowOf(page, state.planned, releases.named)).toHaveCount(0);
  await page.getByRole("button", { name: new RegExp(`^${state.retired}`, "u") }).click();
  await expect(rowOf(page, state.retired, releases.named)).toBeVisible();
});

test("iss_08_a_person_without_manage_releases_sees_the_page_without_changes", async ({ page }) => {
  // Ana is a Viewer of WEB: she sees its releases and may change none.
  await signIn(page, ana);
  await openReleases(page);
  await expect(rowOf(page, state.planned, releases.open)).toBeVisible();
  await expect(page.getByRole("button", { name: t("releases.create") })).toHaveCount(0);
  await expect(page.getByRole("button", { name: t("releases.change") })).toHaveCount(0);
});
