import { dataOf, layoutEditPart, type Schema } from "@bugaboo/api-client";
import { t } from "@bugaboo/messages";
import { expect, type Locator, type Page } from "@playwright/test";

import { addressInProject, paths, projectIssuesAddress } from "../../src/shared/paths.ts";
import { roleId, roleName, setFieldLevel, signedInAdmin } from "./access.ts";
import { createOwnField } from "./fields.ts";
import { firstIssueType, newIssueDialog, openNewIssueDialog, prepareWeb } from "./issues.ts";
import { type ApiSession } from "./api.ts";
import { type Instance } from "./server.ts";

/** The name of the own field that the tests of layouts move around. */
export const dueDate = "Review deadline";

/**
 * Gets a server ready for the tests of layouts: the project WEB with a Team member (Bob), a Guest
 * (Lea) and a Viewer (Ana), and the own field *Review deadline* for issues, which the Guest may fill in
 * (so only the layout decides whether the form asks the Guest for it).
 */
export async function prepareLayouts(server: Instance): Promise<void> {
  await prepareWeb(server);
  const admin = await signedInAdmin(server);
  const made = await createOwnField(admin, dueDate, "date");
  await setFieldLevel(
    admin,
    { role: await roleId(admin, roleName.guest), field: made.id },
    "can-change",
  );
  await admin.dispose();
}

/**
 * The layout of the new-issue form of the project WEB for the issue type *Bug* (the type that the
 * new-issue dialog starts with, so the place that *Customize this form* makes a layout for), as
 * the editor reads it.
 */
async function webEditor(admin: ApiSession, bug: string): Promise<Schema<"LayoutEditorBody">> {
  return dataOf(
    await admin.client.GET("/api/v1/layouts/editor", {
      params: { query: { kind: "new-issue", project: "WEB", issue_type: bug } },
    }),
  );
}

/** What the editor says of a field, as the save takes it. */
function kept(field: Schema<"EditorFieldBody">) {
  return { field: field.field, roles: [...field.roles] };
}

/** Takes the layout of the form of *Bug* in the project WEB away: it uses the layout it inherits again. */
async function removeLayoutOfWeb(admin: ApiSession, editor: Schema<"LayoutEditorBody">) {
  if (editor.layout !== undefined && editor.layout !== null) {
    dataOf(
      await admin.client.DELETE("/api/v1/layouts/{layout}", {
        params: { path: { layout: editor.layout.id }, query: { version: editor.layout.version } },
      }),
    );
  }
}

/** A field to put into a layout, and the roles it is for. */
interface Placed {
  readonly field: Schema<"EditorFieldBody">["field"];
  readonly roles: readonly string[];
}

/** Saves the layout of a form with these fields at the end of its first section. */
async function putFieldsInFirstSection(
  admin: ApiSession,
  editor: Schema<"LayoutEditorBody">,
  slot: NonNullable<Schema<"LayoutEditorBody">["layout"]>,
  placed: readonly Placed[],
): Promise<void> {
  const what = { record: { kind: "layout", id: slot.id }, part: layoutEditPart } as const;
  const taken = dataOf(
    await admin.client.POST("/api/v1/edit-locks", {
      body: { target: what, editor_id: crypto.randomUUID() },
    }),
  );
  if (taken.kind !== "taken") {
    throw new TypeError("Somebody else is editing the layout.");
  }
  const sections = editor.sections.map((section) => ({
    tab: section.tab,
    title: section.title,
    fields: section.fields.map(kept),
  }));
  const first = sections[0] ?? { tab: "", title: "", fields: [] };
  sections.splice(0, 1, {
    ...first,
    fields: [
      ...first.fields,
      ...placed.map((each) => ({ field: each.field, roles: [...each.roles] })),
    ],
  });
  dataOf(
    await admin.client.PUT("/api/v1/layouts/{layout}", {
      params: { path: { layout: slot.id } },
      body: { version: slot.version, lock_token: taken.lock_token, sections },
    }),
  );
  await admin.client.POST("/api/v1/edit-locks/let-go", {
    body: { target: what, lock_token: taken.lock_token },
  });
}

/** Saves the layout of *Bug* in the project WEB with *Review deadline* (for every role) at the end of the first section. */
async function putDueDateInFirstSection(
  admin: ApiSession,
  editor: Schema<"LayoutEditorBody">,
  slot: NonNullable<Schema<"LayoutEditorBody">["layout"]>,
  due: Schema<"EditorFieldBody">,
): Promise<void> {
  const everyone = editor.roles.map((role) => role.id);
  await putFieldsInFirstSection(admin, editor, slot, [{ field: due.field, roles: everyone }]);
}

/**
 * Gives the new-issue form of *Bug* in the project WEB, for every role but the Guest, the fields
 * with these names (an own field by its name, another by its key) besides what it asks for already (the project makes a layout of its own when it
 * has none): the full form a Team member fills in, where a Guest gets the short one.
 */
export async function addToTheFormOfWeb(
  admin: ApiSession,
  names: readonly string[],
): Promise<void> {
  const bug = await firstIssueType(admin);
  let editor = await webEditor(admin, bug);
  if (editor.layout === undefined || editor.layout === null) {
    editor = dataOf(
      await admin.client.POST("/api/v1/layouts", {
        body: { kind: "new-issue", project: "WEB", issue_type: bug },
      }),
    );
  }
  const guest = await roleId(admin, roleName.guest);
  const others = editor.roles.map((role) => role.id).filter((role) => role !== guest);
  const placed: Placed[] = [];
  for (const name of names) {
    const found = editor.available.find((field) => field.name === name || field.field === name);
    if (found === undefined) {
      throw new TypeError(`The form cannot take a field called ${name}.`);
    }
    placed.push({ field: found.field, roles: others });
  }
  if (editor.layout === undefined || editor.layout === null) {
    throw new TypeError("The project has no layout of its own after one was made.");
  }
  await putFieldsInFirstSection(admin, editor, editor.layout, placed);
}

/**
 * Brings the new-issue form of the project WEB to the state a test starts from, through the API,
 * so that no test depends on what another one did: with `customized` the project has a layout of
 * its own for *Bug* with *Review deadline* in the first section (what the lead makes in the first test);
 * with `inherited` the form of *Bug* has no layout of its own and uses the one it inherits.
 */
export async function setLayoutOfWeb(
  server: Instance,
  state: "customized" | "inherited",
): Promise<void> {
  const admin = await signedInAdmin(server);
  const bug = await firstIssueType(admin);
  let editor = await webEditor(admin, bug);
  if (state === "inherited") {
    await removeLayoutOfWeb(admin, editor);
  } else {
    if (editor.layout === undefined || editor.layout === null) {
      editor = dataOf(
        await admin.client.POST("/api/v1/layouts", {
          body: { kind: "new-issue", project: "WEB", issue_type: bug },
        }),
      );
    }
    const due = editor.available.find((field) => field.name === dueDate);
    if (editor.layout !== undefined && editor.layout !== null && due !== undefined) {
      await putDueDateInFirstSection(admin, editor, editor.layout, due);
    }
  }
  await admin.dispose();
}

/**
 * Waits until the editor of a layout can be used: when another window of the same person still
 * holds the lock (the lease lasts a minute after a window closed), takes the editing over.
 */
export async function editWhenYouCan(page: Page, ready: Locator): Promise<void> {
  const takeOver = page.getByRole("button", { name: t("editing.edit-anyway"), exact: true });
  // The editor is on its way (its buttons are switched off) until the lock is decided.
  await expect
    .poll(async () => (await takeOver.isVisible()) || (await ready.isEnabled()))
    .toBe(true);
  if (await takeOver.isVisible()) {
    await takeOver.click();
  }
  await expect(ready).toBeEnabled();
}

/** Adds a section to the layout in the editor by pressing the button with the keyboard. */
export async function addSectionWithKeyboard(page: Page): Promise<void> {
  const add = page.getByRole("button", { name: t("layouts.add-section"), exact: true });
  await add.focus();
  await expect(add).toBeFocused();
  await add.press("Enter");
}

/**
 * Presses the keys one after the other on the list. The list may be gone before the last key (the
 * first letters can already pick the choice and the editor draws the list again): then there is
 * nothing left to type, and no key goes to the page.
 */
async function pressKeys(list: Locator, keys: string): Promise<void> {
  if (keys === "" || (await list.count()) === 0) {
    return;
  }
  await list.press(keys.slice(0, 1), { timeout: 2000 }).catch(() => null);
  await pressKeys(list, keys.slice(1));
}

/**
 * Picks a choice of a list in the editor with the keyboard alone: the list has the focus and the
 * choice is typed into it, one key at a time. Each key is pressed on the list, never on the page,
 * so a letter can not reach a shortcut of the page.
 */
export async function chooseByTyping(list: Locator, choice: string): Promise<void> {
  await list.focus();
  await expect(list).toBeFocused();
  await pressKeys(list, choice);
}

/**
 * The same person, signed in in another window (`other`), opens the layout of *Bug*, takes the editing over, adds a
 * section with this title and saves.
 */
export async function saveSectionInAnotherWindow(other: Page, title: string): Promise<void> {
  await openLayoutOfBugs(other);
  const add = other.getByRole("button", { name: t("layouts.add-section"), exact: true });
  await editWhenYouCan(other, add);
  await add.click();
  await other.getByLabel(t("layouts.section-title"), { exact: true }).last().fill(title);
  await other.getByRole("button", { name: t("layouts.save"), exact: true }).click();
  await expect(other.getByText(t("save.saved"), { exact: true })).toBeVisible();
}

/** The dialog that makes an issue in the project WEB, opened with the button of the bar. */
export async function openNewIssueOfWeb(page: Page) {
  await page.goto(projectIssuesAddress("WEB"));
  await openNewIssueDialog(page);
  return newIssueDialog(page);
}

/** Opens the editor of the new-issue form for the issue type *Bug*, whose layout the tests customize through `setLayoutOfWeb`. */
export async function openLayoutOfBugs(page: Page): Promise<void> {
  await page.goto(addressInProject(paths.projectLayouts, "WEB"));
  await page
    .getByLabel(t("issue-field.issue-type"), { exact: true })
    .selectOption({ label: "Bug" });
}
