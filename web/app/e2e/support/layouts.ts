import { dataOf, layoutEditPart, type Schema } from "@bugaboo/api-client";
import { t } from "@bugaboo/messages";
import { expect, type Browser, type Locator, type Page } from "@playwright/test";

import { addressInProject, paths } from "../../src/shared/paths.ts";
import { roleId, roleName, setFieldLevel, signedInAdmin } from "./access.ts";
import { ada } from "./accounts.ts";
import { anotherBrowser, signIn } from "./flows.ts";
import { prepareWeb } from "./issues.ts";
import { type ApiSession } from "./api.ts";
import { type Instance } from "./server.ts";

/** The name of the own field that the tests of layouts move around. */
export const dueDate = "Due date";

/**
 * Gets a server ready for the tests of layouts: the project WEB with a Team member (Bob), a Guest
 * (Lea) and a Viewer (Ana), and the own field *Due date* for issues, which the Guest may fill in
 * (so only the layout decides whether the form asks the Guest for it).
 */
export async function prepareLayouts(server: Instance): Promise<void> {
  await prepareWeb(server);
  const admin = await signedInAdmin(server);
  const made = dataOf(
    await admin.client.POST("/api/v1/fields", {
      body: { name: dueDate, kind: "date", applies_to: "issues" },
    }),
  );
  await setFieldLevel(
    admin,
    { role: await roleId(admin, roleName.guest), field: made.id },
    "can-change",
  );
  await admin.dispose();
}

/** The layout of the new-issue form of the project WEB, as the editor reads it. */
async function webEditor(admin: ApiSession): Promise<Schema<"LayoutEditorBody">> {
  return dataOf(
    await admin.client.GET("/api/v1/layouts/editor", {
      params: { query: { kind: "new-issue", project: "WEB" } },
    }),
  );
}

/** What the editor says of a field, as the save takes it. */
function kept(field: Schema<"EditorFieldBody">) {
  return { field: field.field, roles: [...field.roles] };
}

/** Takes the layout of the project WEB away: the project uses the layout it inherits again. */
async function removeLayoutOfWeb(admin: ApiSession, editor: Schema<"LayoutEditorBody">) {
  if (editor.layout !== undefined && editor.layout !== null) {
    dataOf(
      await admin.client.DELETE("/api/v1/layouts/{layout}", {
        params: { path: { layout: editor.layout.id }, query: { version: editor.layout.version } },
      }),
    );
  }
}

/** Saves the layout of the project WEB with *Due date* (for every role) at the end of the first section. */
async function putDueDateInFirstSection(
  admin: ApiSession,
  editor: Schema<"LayoutEditorBody">,
  slot: NonNullable<Schema<"LayoutEditorBody">["layout"]>,
  due: Schema<"EditorFieldBody">,
): Promise<void> {
  const what = { record: { kind: "layout", id: slot.id }, part: layoutEditPart };
  const taken = dataOf(
    await admin.client.POST("/api/v1/edit-locks", {
      body: { what, editor_id: crypto.randomUUID() },
    }),
  );
  if (taken.status !== "taken") {
    throw new TypeError("Somebody else is editing the layout.");
  }
  const sections = editor.sections.map((section) => ({
    tab: section.tab,
    title: section.title,
    fields: section.fields.map(kept),
  }));
  const first = sections[0] ?? { tab: "", title: "", fields: [] };
  const everyone = editor.roles.map((role) => role.id);
  sections.splice(0, 1, {
    ...first,
    fields: [...first.fields, { field: due.field, roles: everyone }],
  });
  dataOf(
    await admin.client.PUT("/api/v1/layouts/{layout}", {
      params: { path: { layout: slot.id } },
      body: { version: slot.version, lock_token: taken.lock_token, sections },
    }),
  );
  await admin.client.POST("/api/v1/edit-locks/release", {
    body: { what, lock_token: taken.lock_token },
  });
}

/**
 * Brings the new-issue form of the project WEB to the state a test starts from, through the API,
 * so that no test depends on what another one did: with `customized` the project has a layout of
 * its own with *Due date* in the first section (what the lead makes in the first test); with
 * `inherited` the project has no layout of its own and uses the one it inherits.
 */
export async function setLayoutOfWeb(
  server: Instance,
  state: "customized" | "inherited",
): Promise<void> {
  const admin = await signedInAdmin(server);
  let editor = await webEditor(admin);
  if (state === "inherited") {
    await removeLayoutOfWeb(admin, editor);
  } else {
    if (editor.layout === undefined || editor.layout === null) {
      editor = dataOf(
        await admin.client.POST("/api/v1/layouts", { body: { kind: "new-issue", project: "WEB" } }),
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
  const takeOver = page.getByRole("button", { name: t("layouts.edit-anyway"), exact: true });
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
  await page.getByRole("button", { name: t("layouts.add-section"), exact: true }).focus();
  await page.keyboard.press("Enter");
}

/** Picks a choice of a list in the editor with the keyboard alone: the list has the focus and the choice is typed. */
export async function chooseByTyping(page: Page, list: Locator, choice: string): Promise<void> {
  await list.focus();
  await page.keyboard.type(choice);
}

/**
 * The same person, in another window, opens the layout of *Bug*, takes the editing over, adds a
 * section with this title and saves.
 */
export async function saveSectionInAnotherWindow(
  browser: Browser,
  server: Instance,
  title: string,
): Promise<void> {
  const other = await anotherBrowser(browser, server);
  await signIn(other, ada);
  await openLayoutOfBugs(other);
  const add = other.getByRole("button", { name: t("layouts.add-section"), exact: true });
  await editWhenYouCan(other, add);
  await add.click();
  await other.getByLabel(t("layouts.section-title"), { exact: true }).last().fill(title);
  await other.getByRole("button", { name: t("layouts.save"), exact: true }).click();
  await expect(other.getByText(t("layouts.saved"), { exact: true })).toBeVisible();
}

/** Opens the editor of the new-issue form for the issue type *Bug*, whose layout the tests customize through `setLayoutOfWeb`. */
export async function openLayoutOfBugs(page: Page): Promise<void> {
  await page.goto(addressInProject(paths.projectLayouts, "WEB"));
  await page.getByLabel(t("layouts.type"), { exact: true }).selectOption({ label: "Bug" });
}
