import { t } from "@bugaboo/messages";
import { type Page } from "@playwright/test";

import { ada, bob } from "./support/accounts.ts";
import { lea, roleName } from "./support/access.ts";
import { expectGoodInEveryLook } from "./support/checks.ts";
import { expect, test } from "./support/fixtures.ts";
import { anotherBrowser, signIn } from "./support/flows.ts";
import {
  addSectionWithKeyboard,
  chooseByTyping,
  dueDate,
  editWhenYouCan,
  openLayoutOfBugs,
  openNewIssueOfWeb,
  prepareLayouts,
  setLayoutOfWeb,
  saveSectionInAnotherWindow,
} from "./support/layouts.ts";
import { titleLabel } from "./support/issues.ts";
import { handleIn, onThisLayout, slotsIn } from "./support/layout.ts";

// Layouts and role-based forms (CFG-01, ISS-02, ISS-17, M3 WP-09): a project lead opens
// *Customize this form* from the new-issue dialog, moves *Review deadline* into a section (by dragging on a
// computer, with the list and the buttons everywhere), unticks the Guest, and the Guest's form is
// the short one while a Team member's is the full one.

/** A lock that is let go at once is free within this, not after the lease of a minute. */
const ONE_SECOND_MS = 1000;

test.use({ preparation: { run: prepareLayouts } });

/** The dialog of this person, once its form is there: the description is on every form. */
async function openedForm(page: Page) {
  const dialog = await openNewIssueOfWeb(page);
  await expect(dialog.getByLabel(titleLabel(), { exact: true })).toBeVisible();
  await expect(
    dialog.getByText(t("issue-field.description"), { exact: true }).first(),
  ).toBeVisible();
  return dialog;
}

/** The row of a field in the editor, by its name. */
function rowOf(page: Page, name: string) {
  return page.getByRole("listitem").filter({ has: page.getByText(name, { exact: true }) });
}

/** Opens the editor from *Customize this form* in the dialog and makes the layout of the project. */
async function customizeFromDialog(page: Page): Promise<void> {
  const dialog = await openNewIssueOfWeb(page);
  await dialog.getByRole("button", { name: t("layouts.customize-form"), exact: true }).click();
  await expect(
    page.getByRole("heading", {
      level: 1,
      name: t("layouts.project-title", { project: "Website" }),
    }),
  ).toBeVisible();
  await page
    .getByRole("button", {
      name: t("layouts.customize-project", { project: "Website" }),
    })
    .click();
  await expect(
    page.getByRole("heading", { name: t("layouts.section", { number: "1" }) }),
  ).toBeVisible();
}

/** Moves *Review deadline* into the first section: dragged on a computer, from its list elsewhere. */
async function moveDueDateIntoSection(page: Page): Promise<void> {
  const available = page.getByRole("list", {
    name: t("layouts.available"),
    exact: true,
  });
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
      await handleIn(available, t("layouts.field-drag", { name: dueDate })).dragTo(
        slotsIn(section).last(),
      );
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

/** Unticks the Guest for *Review deadline* and saves the layout. */
async function untickGuestAndSave(page: Page): Promise<void> {
  const roles = rowOf(page, dueDate)
    .last()
    .getByRole("group", { name: t("layouts.roles-form", { name: dueDate }) });
  const guest = roles.getByRole("checkbox", { name: roleName.guest });
  await expect(guest).toBeChecked();
  await roles.getByText(roleName.guest, { exact: true }).click();
  await expect(guest).not.toBeChecked();
  await page.getByRole("button", { name: t("layouts.save"), exact: true }).click();
  await expect(page.getByText(t("save.saved"), { exact: true })).toBeVisible();
}

test("cfg_01 a lead customizes the new-issue form, and a Guest gets the short form and a Team member the full one", async ({
  page,
  browser,
  server,
}) => {
  await setLayoutOfWeb(server, "inherited");
  await signIn(page, ada);
  await customizeFromDialog(page);
  await moveDueDateIntoSection(page);
  await expect(rowOf(page, dueDate).last()).toBeVisible();
  await expectGoodInEveryLook(page);
  await untickGuestAndSave(page);

  const guest = await anotherBrowser(browser, server);
  await signIn(guest, lea);
  // The Guest may fill in *Review deadline*; the layout does not ask for it. (The field is never asked
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
  await setLayoutOfWeb(server, "customized");
  await signIn(page, ada);
  await openLayoutOfBugs(page);
  const row = rowOf(page, dueDate).last();
  await expect(row).toBeVisible();
  // Up and down are buttons: the keyboard presses them with Enter.
  const down = row.getByRole("button", {
    name: t("layouts.field-down", { name: dueDate }),
  });
  const up = row.getByRole("button", {
    name: t("layouts.field-up", { name: dueDate }),
  });
  // An earlier window of the same person may still hold the lock for a minute: the same person may take the editing over.
  await editWhenYouCan(page, up);
  await up.focus();
  await page.keyboard.press("Enter");
  await expect(down).toBeEnabled();
  await expect(page.getByText(t("save.not-saved"), { exact: true })).toBeVisible();
  // Taking it off the form puts it with the fields that are not on it.
  await row.getByRole("button", { name: t("layouts.field-remove", { name: dueDate }) }).click();
  await expect(
    page
      .getByRole("list", { name: t("layouts.available"), exact: true })
      .getByText(dueDate, { exact: true }),
  ).toBeVisible();
  await page.getByRole("button", { name: t("editing.changes-drop"), exact: true }).click();

  // Another lead opens the same layout: Ada holds the lock, so they are told.
  const other = await anotherBrowser(browser, server);
  await signIn(other, ada);
  await openLayoutOfBugs(other);
  // The second window is the same person's, so the note says that, and *Save* waits.
  await expect(
    other.getByText(t("editing.elsewhere", { thing: t("editing.this-layout") })),
  ).toBeVisible();
  await expect(other.getByRole("button", { name: t("layouts.save"), exact: true })).toBeDisabled();
});

/** The button that moves *Review deadline* up in the editor of a layout. */
function upOf(page: Page) {
  return rowOf(page, dueDate)
    .last()
    .getByRole("button", { name: t("layouts.field-up", { name: dueDate }) });
}

test("cfg_01 a window of the layout editor that goes away lets its lock go at once (ADR-0019)", async ({
  browser,
  server,
}) => {
  await setLayoutOfWeb(server, "customized");
  const first = await anotherBrowser(browser, server);
  await signIn(first, ada);
  await openLayoutOfBugs(first);
  await editWhenYouCan(first, upOf(first));
  // The window goes away without leaving the editor: the lock is let go as the page is hidden.
  await first.goto("about:blank");
  const second = await anotherBrowser(browser, server);
  await signIn(second, ada);
  await openLayoutOfBugs(second);
  await expect(upOf(second)).toBeEnabled({ timeout: ONE_SECOND_MS });
  await expect(
    second.getByRole("button", { name: t("editing.edit-anyway"), exact: true }),
  ).toHaveCount(0);
});

test("cfg_01 a field is moved and added with the keyboard alone, and its drag handle is not announced", async ({
  page,
  server,
}) => {
  await setLayoutOfWeb(server, "customized");
  await signIn(page, ada);
  await openLayoutOfBugs(page);
  const up = rowOf(page, dueDate)
    .last()
    .getByRole("button", { name: t("layouts.field-up", { name: dueDate }) });
  await editWhenYouCan(page, up);
  // The handle is for the mouse and the finger: a screen reader finds the buttons and the lists.
  await expect(
    page.getByRole("button", { name: t("layouts.field-drag", { name: dueDate }) }),
  ).toHaveCount(0);

  const sectionList = (number: string) =>
    page.getByRole("list", { name: t("layouts.fields-of", { number }) });
  await addSectionWithKeyboard(page);
  await expect(sectionList("2")).toBeAttached();
  // *Move to* the second section, then off the form, then *Add to* the first section.
  const moveTo = page.getByLabel(t("layouts.field-move-to", { name: dueDate }), { exact: true });
  await chooseByTyping(moveTo, t("layouts.section", { number: "2" }));
  await expect(sectionList("2").getByText(dueDate, { exact: true })).toBeVisible();
  const remove = sectionList("2")
    .getByRole("listitem")
    .filter({ hasText: dueDate })
    .getByRole("button", { name: t("layouts.field-remove", { name: dueDate }) });
  await remove.focus();
  await expect(remove).toBeFocused();
  await remove.press("Enter");
  const available = page.getByRole("list", { name: t("layouts.available"), exact: true });
  await expect(available.getByText(dueDate, { exact: true })).toBeVisible();
  const addTo = available.getByLabel(t("layouts.field-add-to", { name: dueDate }), {
    exact: true,
  });
  await chooseByTyping(addTo, t("layouts.section", { number: "1" }));
  await expect(sectionList("1").getByText(dueDate, { exact: true })).toBeVisible();
  await page.getByRole("button", { name: t("editing.changes-drop"), exact: true }).click();
});

test("cfg_01 a layout that changed while it was edited shows the new one and keeps the changes as a draft", async ({
  page,
  browser,
  server,
}) => {
  await setLayoutOfWeb(server, "customized");
  await signIn(page, ada);
  await openLayoutOfBugs(page);
  const add = page.getByRole("button", { name: t("layouts.add-section"), exact: true });
  await editWhenYouCan(page, add);
  const titles = page.getByLabel(t("layouts.section-title"), { exact: true });
  const before = await titles.count();
  await add.click();
  await titles.last().fill("Mine");

  // The same person in another window takes the editing over and saves another change.
  const other = await anotherBrowser(browser, server);
  await signIn(other, ada);
  await saveSectionInAnotherWindow(other, "Theirs");

  // The first window learns at its next heartbeat that it lost the lock, asks for it again and
  // saves on the version it started from.
  await expect(page.getByText(t("layouts.lost"), { exact: true })).toBeVisible({
    timeout: 45_000,
  });
  await page.getByRole("button", { name: t("common.try-again"), exact: true }).click();
  await editWhenYouCan(page, add);
  await page.getByRole("button", { name: t("layouts.save"), exact: true }).click();

  // It says plainly that the layout changed, shows the layout as it is now, and keeps the changes.
  const changed = page.getByText(/This layout changed while you were editing/u);
  await expect(changed).toBeVisible();
  await expect(titles).toHaveCount(before + 1);
  await expect(titles.last()).toHaveValue("Theirs");
  await page.getByRole("button", { name: t("layouts.draft-restore"), exact: true }).click();
  // Putting the changes back replaces the layout on screen with the draft; saving makes it the layout.
  await expect(titles).toHaveCount(before + 1);
  await expect(titles.last()).toHaveValue("Mine");
  await page.getByRole("button", { name: t("layouts.save"), exact: true }).click();
  await expect(page.getByText(t("save.saved"), { exact: true })).toBeVisible();
  await expect(changed).toHaveCount(0);
});
