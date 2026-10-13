import { t } from "@bugaboo/messages";
import { type Page } from "@playwright/test";

import { addressInProject, paths, projectIssuesAddress } from "../src/shared/paths.ts";
import { ada, bob } from "./support/accounts.ts";
import { lea, roleName } from "./support/access.ts";
import { expectGoodInEveryLook } from "./support/checks.ts";
import { expect, test } from "./support/fixtures.ts";
import { anotherBrowser, signIn } from "./support/flows.ts";
import { dueDate, editWhenYouCan, prepareLayouts } from "./support/layouts.ts";
import { onThisLayout } from "./support/layout.ts";

// Layouts and role-based forms (CFG-01, ISS-02, ISS-17, M3 WP-09): a project lead opens
// *Customize this form* from the new-issue dialog, moves *Due date* into a section (by dragging on a
// computer, with the list and the buttons everywhere), unticks the Guest, and the Guest's form is
// the short one while a Team member's is the full one.

test.use({ preparation: { run: prepareLayouts } });
test.describe.configure({ mode: "serial" });

const dialogName = t("issues.new-title");

/** The places where a dragged field can be dropped. */
const slot = '[data-part="layout-slot"]';

/** The dialog that makes an issue, opened with the button of the bar. */
async function openNewIssue(page: Page) {
  await page.goto(projectIssuesAddress("WEB"));
  await page
    .getByRole("button", { name: t("issues.new"), exact: true })
    .first()
    .click();
  const dialog = page.getByRole("dialog", { name: dialogName, exact: true });
  await expect(dialog).toBeVisible();
  return dialog;
}

/** The dialog of this person, once its form is there: the description is on every form. */
async function openedForm(page: Page) {
  const dialog = await openNewIssue(page);
  await expect(dialog.getByLabel(t("issues.title-required"), { exact: true })).toBeVisible();
  await expect(
    dialog.getByText(t("issue-field.description"), { exact: true }).first(),
  ).toBeVisible();
  return dialog;
}

/** Opens the editor of the new-issue form for the issue type *Bug*, which the dialog customized. */
async function openLayoutOfBugs(page: Page): Promise<void> {
  await page.goto(addressInProject(paths.projectLayouts, "WEB"));
  await page.getByLabel(t("layouts.type"), { exact: true }).selectOption({ label: "Bug" });
}

/** The row of a field in the editor, by its name. */
function rowOf(page: Page, name: string) {
  return page.getByRole("listitem").filter({ has: page.getByText(name, { exact: true }) });
}

/** Opens the editor from *Customize this form* in the dialog and makes the layout of the project. */
async function customizeFromDialog(page: Page): Promise<void> {
  const dialog = await openNewIssue(page);
  await dialog.getByRole("button", { name: t("layouts.customize-form"), exact: true }).click();
  await expect(
    page.getByRole("heading", {
      level: 1,
      name: t("layouts.project-title", { project: "Website" }),
    }),
  ).toBeVisible();
  await page
    .getByRole("button", { name: t("layouts.customize-project", { project: "Website" }) })
    .click();
  await expect(
    page.getByRole("heading", { name: t("layouts.section", { number: "1" }) }),
  ).toBeVisible();
}

/** Moves *Due date* into the first section: dragged on a computer, from its list elsewhere. */
async function moveDueDateIntoSection(page: Page): Promise<void> {
  const available = page.getByRole("list", { name: t("layouts.available"), exact: true });
  await expect(available.getByText(dueDate, { exact: true })).toBeVisible();
  const addTo = available.getByLabel(t("layouts.field-add-to", { name: dueDate }), {
    exact: true,
  });
  await onThisLayout(page, {
    computer: async () => {
      const section = page.getByRole("region", {
        name: t("layouts.section", { number: "1" }),
        exact: true,
      });
      await available
        .getByRole("button", { name: t("layouts.field-drag", { name: dueDate }) })
        .dragTo(section.locator(slot).last());
    },
    tablet: async () => {
      await addTo.selectOption({ index: 1 });
    },
    phone: async () => {
      await addTo.selectOption({ index: 1 });
    },
  });
  await expect(available.getByText(dueDate, { exact: true })).toHaveCount(0);
}

/** Unticks the Guest for *Due date* and saves the layout. */
async function untickGuestAndSave(page: Page): Promise<void> {
  const roles = rowOf(page, dueDate)
    .last()
    .getByRole("group", { name: t("layouts.roles-form", { name: dueDate }) });
  const guest = roles.getByRole("checkbox", { name: roleName.guest });
  await expect(guest).toBeChecked();
  await roles.getByText(roleName.guest, { exact: true }).click();
  await expect(guest).not.toBeChecked();
  await page.getByRole("button", { name: t("layouts.save"), exact: true }).click();
  await expect(page.getByText(t("layouts.saved"), { exact: true })).toBeVisible();
}

test("cfg_01 a lead customizes the new-issue form, and a Guest gets the short form and a Team member the full one", async ({
  page,
  browser,
  server,
}) => {
  await signIn(page, ada);
  await customizeFromDialog(page);
  await moveDueDateIntoSection(page);
  await expect(rowOf(page, dueDate).last()).toBeVisible();
  await expectGoodInEveryLook(page);
  await untickGuestAndSave(page);

  const guest = await anotherBrowser(browser, server);
  await signIn(guest, lea);
  // The Guest may fill in *Due date*; the layout does not ask for it. (The field is never asked
  // for, so there is nothing to wait for: the form is there with the description.)
  await expect((await openedForm(guest)).getByLabel(dueDate, { exact: true })).toHaveCount(0);
  const member = await anotherBrowser(browser, server);
  await signIn(member, bob);
  await expect((await openedForm(member)).getByLabel(dueDate, { exact: true })).toBeVisible();
});

test("cfg_01 every move of the editor is a button too, and a second editor is told who is editing", async ({
  page,
  browser,
  server,
}) => {
  await signIn(page, ada);
  await openLayoutOfBugs(page);
  const row = rowOf(page, dueDate).last();
  await expect(row).toBeVisible();
  // Up and down are buttons: the keyboard presses them with Enter.
  const down = row.getByRole("button", { name: t("layouts.field-down", { name: dueDate }) });
  const up = row.getByRole("button", { name: t("layouts.field-up", { name: dueDate }) });
  // The first test left its lock for a minute: the same person may take the editing over.
  await editWhenYouCan(page, up);
  await up.focus();
  await page.keyboard.press("Enter");
  await expect(down).toBeEnabled();
  await expect(page.getByText(t("layouts.unsaved"), { exact: true })).toBeVisible();
  // Taking it off the form puts it with the fields that are not on it.
  await row.getByRole("button", { name: t("layouts.field-remove", { name: dueDate }) }).click();
  await expect(
    page
      .getByRole("list", { name: t("layouts.available"), exact: true })
      .getByText(dueDate, { exact: true }),
  ).toBeVisible();
  await page.getByRole("button", { name: t("layouts.discard"), exact: true }).click();

  // Another lead opens the same layout: Ada holds the lock, so they are told.
  const other = await anotherBrowser(browser, server);
  await signIn(other, ada);
  await openLayoutOfBugs(other);
  await expect(other.getByText(/is editing this layout/u)).toBeVisible();
  await expect(other.getByRole("button", { name: t("layouts.save"), exact: true })).toBeDisabled();
});
