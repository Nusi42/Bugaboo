import { t } from "@bugaboo/messages";
import { type Page } from "@playwright/test";

import { newIssueDialog, openNewIssueDialog } from "./issues.ts";
import { box, list } from "./quick-search.ts";
import { expectHeading, expectRows } from "./readme-media.ts";
import { PACE } from "./readme-pace.ts";
import { readmeIssueTitle, readmeProject, readmeRepairs } from "./readme-organization.ts";
import { type Recording } from "./recording.ts";
import { titleBox } from "./windows.ts";
import { projectIssuesAddress } from "../../src/shared/paths.ts";

// What the recordings of the front page show, step by step, on the made-up workshop.

/** How long quick search takes to show the results of a typed letter. */
const RESULTS_MS = 400;

/** The title of the repair that the last recording reports. */
const NEW_REPAIR = "Fit new mudguards to a city bike";

/** From the list of the workshop to the cargo bike. */
export async function openAnIssue(page: Page, recording: Readonly<Recording>): Promise<void> {
  await page.goto(projectIssuesAddress(readmeProject));
  await expectRows(page);
  await recording.look(PACE.startMs);
  const row = page.locator("[data-part='list-row']").filter({ hasText: readmeIssueTitle });
  await recording.click(row.getByRole("link").first());
  await expectHeading(page);
  await recording.look(PACE.endMs);
}

/** Quick search finds the issues about brakes while the word is typed, and one is opened. */
export async function findAnIssue(page: Page, recording: Readonly<Recording>): Promise<void> {
  await page.goto(projectIssuesAddress(readmeProject));
  await expectRows(page);
  await recording.look(PACE.startMs);
  await page.keyboard.press("Control+k");
  await box(page).waitFor();
  await recording.look();
  await recording.type(box(page), "brake", RESULTS_MS);
  const found = list(page).getByRole("option").first();
  await found.waitFor();
  await page.waitForLoadState("networkidle");
  await recording.look(PACE.resultMs);
  await recording.click(found);
  await expectHeading(page);
  await recording.look(PACE.endMs);
}

/** A new repair is written and reported from the list of the repairs. */
export async function reportAnIssue(page: Page, recording: Readonly<Recording>): Promise<void> {
  await page.goto(projectIssuesAddress(readmeRepairs));
  await expectRows(page);
  await recording.look(PACE.startMs);
  await recording.pointAt(page.getByRole("button", { name: t("issues.new"), exact: true }).first());
  await openNewIssueDialog(page);
  await recording.look();
  await recording.type(titleBox(page), NEW_REPAIR);
  await recording.look(PACE.resultMs);
  await recording.click(
    newIssueDialog(page).getByRole("button", { name: t("issues.create"), exact: true }),
  );
  await newIssueDialog(page).waitFor({ state: "hidden" });
  // The new issue opens.
  await page.getByRole("heading", { level: 1, name: NEW_REPAIR }).waitFor();
  await page.waitForLoadState("networkidle");
  await recording.look(PACE.endMs);
}

/** Shows the workflow of the repairs as a drawing: its statuses and the arrows between them. */
export async function showDrawing(page: Page): Promise<void> {
  const view = page.getByRole("radio", { name: t("workflows.view-drawing"), exact: true });
  await view.check();
  await page
    .getByRole("button", { name: /Open its rules/u })
    .first()
    .waitFor();
  // The drawing is taller than what is above it: the window shows it from the choice of the view.
  await view.evaluate((choice) => {
    choice.scrollIntoView({ block: "start" });
  });
}
