import { t } from "@bugaboo/messages";
import { expect, type Page } from "@playwright/test";

import { projectIssuesAddress } from "../../src/shared/paths.ts";
import { goodInFourThemes } from "./demo.ts";
import {
  assignedToName,
  COST_CENTER,
  costCenter,
  GUEST_BUG,
  made,
  MEMBER_BUG,
  theTwo,
} from "./demo-m3-state.ts";
import { signIn } from "./flows.ts";
import { newIssueDialog, openNewIssueDialog } from "./issues.ts";
import { sampleGuestIn, type SamplePerson } from "./sample.ts";

// The first steps of the exit demo of M3 (`m3-exit-demo.spec.ts`): the two bugs.

/** The key in the address of the page that is open: `/issues/WEB-142`. */
function keyOf(page: Page): string {
  return decodeURIComponent(new URL(page.url()).pathname.split("/").at(-1) ?? "");
}

/** Opens the new-issue form on the Website and writes the title. */
async function startAnIssue(page: Page, title: string): Promise<void> {
  await page.goto(projectIssuesAddress("WEB"));
  await openNewIssueDialog(page);
  await newIssueDialog(page).getByLabel(t("issues.title-required"), { exact: true }).fill(title);
}

/** Presses Create and waits for the page of the new issue. */
async function create(page: Page, title: string): Promise<void> {
  await newIssueDialog(page)
    .getByRole("button", { name: t("issues.create"), exact: true })
    .click();
  await expect(page.getByRole("heading", { name: new RegExp(title, "u") })).toBeVisible();
  await expect(page).toHaveURL(/\/issues\//u);
}

/** Writes the description in the form that is open. */
async function describe(page: Page, text: string): Promise<void> {
  await newIssueDialog(page)
    .getByRole("textbox", { name: t("issue-field.description"), exact: true })
    .fill(text);
}

/** Step 1: a Guest makes a bug with the short form. */
export async function aGuestMakesABug(page: Page): Promise<void> {
  await signIn(page, sampleGuestIn("WEB"));
  await startAnIssue(page, GUEST_BUG);
  await describe(page, "I type my coupon, press Pay, and it is gone.");
  // The short form: no priority, no person to assign, no cost center.
  const form = newIssueDialog(page);
  await expect(form.getByLabel(t("issue-field.priority"), { exact: true })).toHaveCount(0);
  await expect(form.getByText(assignedToName, { exact: true })).toHaveCount(0);
  await expect(form.getByText(costCenter, { exact: true })).toHaveCount(0);
  await goodInFourThemes(page, "01-guest-short-form");
  await create(page, GUEST_BUG);
  made.guestKey = keyOf(page);
  await goodInFourThemes(page, "02-guest-bug-made");
}

/** Picks a person in the picker that *Add Assigned to* opens. */
async function assign(page: Page, person: SamplePerson): Promise<void> {
  const form = newIssueDialog(page);
  await form
    .getByRole("button", { name: t("issues.add-field", { field: assignedToName }) })
    .click();
  const picker = page.getByRole("dialog", { name: t("issues.assign-title") });
  await picker.getByLabel(t("issues.assign-search"), { exact: true }).fill(person.name);
  await picker
    .getByRole("button", { name: t("issues.assign-name", { name: person.name }), exact: true })
    .click();
  await picker.getByRole("button", { name: t("common.done"), exact: true }).click();
  await expect(form.getByText(person.name)).toBeVisible();
}

/** Step 2: a Team member makes a bug with the full form, with Cost center and a person assigned. */
export async function aTeamMemberMakesABug(page: Page): Promise<void> {
  const { first: maker, second: colleague } = theTwo();
  await signIn(page, maker);
  await startAnIssue(page, MEMBER_BUG);
  await describe(page, "Seen on the shop page: 19.99 is shown as 19.98.");
  const form = newIssueDialog(page);
  await form
    .getByLabel(t("issue-field.priority"), { exact: true })
    .selectOption({ label: t("priority.high") });
  await form.getByLabel(costCenter, { exact: true }).fill(COST_CENTER);
  await assign(page, colleague);
  await goodInFourThemes(page, "03-member-full-form");
  await create(page, MEMBER_BUG);
  made.key = keyOf(page);
  await expect(page.getByText(COST_CENTER)).toBeVisible();
  await expect(page.getByText(colleague.name).first()).toBeVisible();
  await goodInFourThemes(page, "04-member-bug-made");
}
