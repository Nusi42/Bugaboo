import { t } from "@bugaboo/messages";
import { expect, type Page } from "@playwright/test";

import { addressInProject, paths, projectIssuesAddress } from "../../src/shared/paths.ts";
import { check } from "./access.ts";
import { ada, bob, cleo } from "./accounts.ts";
import { chooseTheme, type Theme } from "./checks.ts";
import { openFromMenu, signIn } from "./flows.ts";
import { newIssueDialog, openNewIssueDialog } from "./issues.ts";
import { tickIssues } from "./many-changes-pages.ts";
import { shotWholeAs } from "./screenshots.ts";

// The state and the pictures of the screens of M4 WP-11 for the maintainer's review (see
// `screenshots-offers.spec.ts`).

const MILESTONE = "M4";

/** Takes the open screen in the theme, once its content is there. */
async function take(page: Page, theme: Theme, name: string, project: string): Promise<void> {
  await chooseTheme(page, theme);
  await shotWholeAs(page, `${name}-${project}`, MILESTONE, theme.id);
}

export async function takeProjectTypes(page: Page, theme: Theme, project: string): Promise<void> {
  await signIn(page, cleo);
  await page.goto(addressInProject(paths.projectIssueTypes, "WEB"));
  await expect(page.getByRole("list", { name: t("issue-types.project-list") })).toBeVisible();
  await take(page, theme, "project-issue-types", project);
}

export async function takeProjectLabels(page: Page, theme: Theme, project: string): Promise<void> {
  await signIn(page, cleo);
  await page.goto(addressInProject(paths.projectLabels, "WEB"));
  await expect(page.getByRole("list", { name: t("labels.project-list") })).toBeVisible();
  await take(page, theme, "project-labels", project);
}

export async function takeOfferedIn(page: Page, theme: Theme, project: string): Promise<void> {
  await signIn(page, ada);
  await openFromMenu(page, t("nav.issue-types"));
  await page.getByRole("button", { name: "Bug", exact: true }).click();
  const panel = page.getByRole("complementary", { name: "Bug" });
  await expect(panel.getByRole("list", { name: t("projects.offered-in-list") })).toBeVisible();
  await take(page, theme, "offered-in", project);
}

export async function takeNewIssue(page: Page, theme: Theme, project: string): Promise<void> {
  await signIn(page, ada);
  await page.goto(projectIssuesAddress("WEB"));
  await openNewIssueDialog(page);
  await expect(newIssueDialog(page).getByLabel(t("issues.type"), { exact: true })).toBeVisible();
  await take(page, theme, "new-issue-types", project);
}

export async function takeMove(page: Page, theme: Theme, project: string): Promise<void> {
  await signIn(page, ada);
  await tickIssues(page, ["WEB-1"]);
  await page.getByRole("button", { name: t("many-changes.action-move"), exact: true }).click();
  const dialog = page.getByRole("alertdialog");
  await dialog.getByLabel(t("many-changes.project-search"), { exact: true }).fill("Lab");
  await dialog.getByRole("radio", { name: /Lab/u }).click();
  await dialog.getByRole("button", { name: t("many-changes.show-changes"), exact: true }).click();
  await expect(
    dialog.getByText(t("many-changes.note-type-not-used", { type: "Bug", project: "Lab" })),
  ).toBeVisible();
  await take(page, theme, "move-note", project);
}

export async function takeCheckAccess(page: Page, theme: Theme, project: string): Promise<void> {
  await signIn(page, ada);
  await openFromMenu(page, t("nav.check-access"));
  await check(page, "WEB", bob);
  await page
    .getByLabel(t("check-access.issue-type-question"), { exact: true })
    .selectOption({ label: "Bug" });
  await expect(page.getByText(t("check-access.for-issue-type", { type: "Bug" }))).toBeVisible();
  await take(page, theme, "check-access-type", project);
}
