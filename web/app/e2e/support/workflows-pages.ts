import { t } from "@bugaboo/messages";
import { expect, type Locator, type Page } from "@playwright/test";

import { paths } from "../../src/shared/paths.ts";
import { check } from "./access.ts";
import { type Person } from "./accounts.ts";
import { editWhenYouCan } from "./layouts.ts";
import { serverWord } from "./server-words.ts";
import { foundInVersion } from "./workflows.ts";

/** The block of a status in the plain list, found by the name of the status. */
export function blockOf(page: Page, name: string): Locator {
  return page
    .getByRole("listitem")
    .filter({ has: page.getByRole("heading", { level: 3, name, exact: true }) });
}

/** Opens the editor of the workflow of the whole organization for all issue types (*Simple*). */
export async function openSimple(page: Page): Promise<void> {
  await page.goto(paths.workflows);
  await expect(
    page.getByRole("heading", { level: 1, name: t("workflows.organization-title") }),
  ).toBeVisible();
  await expect(page.getByLabel(t("workflows.name"), { exact: true })).toBeVisible();
}

/** Opens the editor of *Simple* and waits until the lock is the person own, so that it can be changed. */
export async function openSimpleToChange(page: Page): Promise<void> {
  await openSimple(page);
  await editWhenYouCan(page, page.getByLabel(t("workflows.name"), { exact: true }));
}

/** Adds the status *Testing* and the status change *Send to testing* from *In progress* to the draft. */
export async function addSendToTesting(page: Page): Promise<void> {
  await page.getByLabel(t("workflows.new-status"), { exact: true }).fill("Testing");
  await page.getByRole("button", { name: t("workflows.add-status"), exact: true }).click();
  await expect(blockOf(page, "Testing")).toBeVisible();
  const progress = page.getByRole("group", {
    name: t("workflows.add-change", { from: "In progress" }),
  });
  await progress.getByLabel(t("workflows.add-change-to"), { exact: true }).selectOption("Testing");
  await progress
    .getByLabel(t("workflows.add-change-button"), { exact: true })
    .fill("Send to testing");
  await progress.getByRole("button", { name: t("workflows.add-change-do"), exact: true }).click();
}

/** Opens the rules of *Send to testing*, asks for *Found in version* to be filled in, and closes them. */
export async function requireFoundInVersion(page: Page): Promise<void> {
  await page
    .getByRole("button", { name: t("workflows.rules-open", { button: "Send to testing" }) })
    .click();
  const rules = page.getByRole("dialog", {
    name: t("workflows.rules-title", { button: "Send to testing" }),
  });
  await rules
    .getByRole("group", { name: t("workflows.must-fill") })
    .getByText(foundInVersion, { exact: true })
    .click();
  await rules.getByRole("button", { name: t("workflows.rules-done"), exact: true }).click();
  await expect(rules).toHaveCount(0);
}

/** Takes a status out of the draft with its button, which also takes the status changes from and to it. */
export async function removeStatus(page: Page, name: string): Promise<void> {
  await page
    .getByRole("button", { name: t("workflows.status-remove", { name }), exact: true })
    .click();
}

/** Gives the workflow in the editor another name. */
export async function renameWorkflowTo(page: Page, name: string): Promise<void> {
  await page.getByLabel(t("workflows.name"), { exact: true }).fill(name);
}

/** *Try it* for an issue and a person: the answer for the status change *Mark as done*. */
export async function tryMarkAsDone(page: Page, issue: string, person: Person): Promise<Locator> {
  const tryIt = page.getByRole("region", { name: t("workflows.try-title") });
  await tryIt.getByLabel(t("common.issue-id"), { exact: true }).fill(issue);
  await check(page, null, person);
  await tryIt.getByRole("button", { name: t("workflows.try-go"), exact: true }).click();
  return tryIt
    .getByRole("listitem")
    .filter({ hasText: serverWord("workflow-change.mark-as-done") });
}

/** The button of the arrow from *Open* to *In progress* in the drawing, which opens its rules. */
export function startWorkingArrow(page: Page): Locator {
  const label = t("workflows.arrow-label", {
    from: "Open",
    to: "In progress",
    button: serverWord("workflow-change.start-working"),
  });
  return page.getByRole("button", { name: label, exact: true });
}
