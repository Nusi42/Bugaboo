import { dataOf, type Schema } from "@bugaboo/api-client";
import { t } from "@bugaboo/messages";
import { expect, type Locator, type Page } from "@playwright/test";

import { valuesBody } from "../../src/features/theme-editor/index.ts";
import { filled, paths, routeParam } from "../../src/shared/paths.ts";
import { part } from "../../../packages/ui/src/tokens.ts";
import {
  bob,
  cleo,
  createAdmin,
  createPerson,
  ownId,
  type Person,
  signInThroughApi,
} from "./accounts.ts";
import { giveRole, roleName } from "./access.ts";
import { type Instance } from "./server.ts";
import { inOrder } from "./sequence.ts";

// The theme editor in the browser (THM-03, THM-04, UX §5.6): who is there, the themes they start
// with, and the parts of the editor a test reads.

/**
 * What every file of the theme editor starts with: Ada, who set Bugaboo up; Bob, a team member; and
 * Cleo, a second administrator, who looks after the themes for everyone too.
 */
export async function prepareThemes(server: Instance): Promise<void> {
  const admin = await createAdmin(server);
  const made = await Promise.all([bob, cleo].map((person) => createPerson(server, admin, person)));
  const [, second] = made;
  if (second !== undefined) {
    await giveRole(admin, await ownId(second), roleName.administrator, null);
  }
  await Promise.all([admin, ...made].map((session) => session.dispose()));
}

/** A theme file as an upload takes it. */
export interface ThemeFile {
  readonly name: string;
  readonly extends: string;
  readonly values: Schema<"ThemeValuesBody">;
}

/** Makes `person` a saved theme of `kind` from a file (an upload is saved at once), through the API. */
export async function uploadedTheme(
  server: Instance,
  person: Person,
  kind: Schema<"ThemeKind">,
  file: ThemeFile,
): Promise<Schema<"ThemeBody">> {
  const session = await signInThroughApi(server, person);
  try {
    const detail = dataOf(
      await session.client.POST("/api/v1/themes/upload", {
        params: { query: { kind } },
        body: { name: file.name, extends: file.extends, values: valuesBody(file.values) },
      }),
    );
    return detail.theme;
  } finally {
    await session.dispose();
  }
}

/** The one own theme `person` has, as they see it; a test that expects one fails when there is none. */
export async function onlyThemeOf(
  server: Instance,
  person: Person,
): Promise<Schema<"ThemeDetailBody">> {
  const session = await signInThroughApi(server, person);
  try {
    const [theme] = dataOf(await session.client.GET("/api/v1/themes")).themes;
    if (theme === undefined) {
      throw new Error(`${person.name} has no theme`);
    }
    return dataOf(
      await session.client.GET("/api/v1/themes/{theme}", { params: { path: { theme: theme.id } } }),
    );
  } finally {
    await session.dispose();
  }
}

/** Takes away every own theme `person` may change, so a test starts with none. */
export async function deleteThemesOf(server: Instance, person: Person): Promise<void> {
  const session = await signInThroughApi(server, person);
  const { themes } = dataOf(await session.client.GET("/api/v1/themes"));
  await inOrder(
    themes
      .filter((theme) => theme.may_change)
      .map((theme) => async () => {
        // A theme somebody still edits (a lock a test left within its lease) stays.
        await session.client
          .DELETE("/api/v1/themes/{theme}", {
            params: { path: { theme: theme.id }, query: { version: theme.version } },
          })
          .catch(() => null);
      }),
  );
  await session.dispose();
}

/** The address of the editor of a theme. */
export function editorAddress(theme: Pick<Schema<"ThemeBody">, "id" | "kind">): string {
  return filled(
    theme.kind === "for-everyone" ? paths.theme : paths.myTheme,
    routeParam.theme,
    theme.id,
  );
}

/** The line on top of the editor that sums the readability up. */
export function readabilityLine(page: Page): Locator {
  return page.locator(`output[data-part='${part.themeReadability}']`);
}

/** Opens the editor of a theme and waits until the rating is there and the person may change it. */
export async function openEditor(
  page: Page,
  theme: Pick<Schema<"ThemeBody">, "id" | "kind">,
): Promise<void> {
  await page.goto(editorAddress(theme));
  await expect(readabilityLine(page)).not.toHaveText(t("theme-editor.readability-loading"));
}

/** A group of the editor, by its name. */
export function groupNamed(page: Page, name: string): Locator {
  return page
    .locator(`[data-part='${part.themeGroup}']`)
    .filter({ has: page.locator("summary").getByText(name, { exact: true }) });
}

/**
 * Puts a color of a group on screen with *Change a color*, opening the group first; a color the
 * theme changes is on screen already.
 */
export async function changeColor(page: Page, group: string, color: string): Promise<void> {
  if (await colorField(page, color).isVisible()) {
    return;
  }
  const box = groupNamed(page, group);
  if ((await box.getAttribute("open")) === null) {
    await box.locator("summary").click();
  }
  await box.getByLabel(t("theme-editor.change-color")).selectOption({ label: color });
  await expect(colorField(page, color)).toBeVisible();
}

/** The text field of a color, by its plain name. */
export function colorField(page: Page, color: string): Locator {
  return page.getByRole("textbox", { name: color, exact: true });
}

/** The preview of the editor: beside the groups on a computer, behind *Preview* on a phone. */
export async function preview(page: Page): Promise<Locator> {
  // A phone shows the button, a computer the preview itself.
  const opens = page.getByRole("button", { name: t("theme-editor.preview"), exact: true });
  if (await opens.isVisible()) {
    await opens.click();
  }
  const shown = page.locator(`[data-part='${part.themeEditorPreview}']`);
  await expect(shown).toBeVisible();
  return shown;
}

/** The value of a custom property on an element, as the browser works it out. */
export function variableOf(element: Locator, name: string): Promise<string> {
  return element.evaluate(
    (node, variable) => getComputedStyle(node).getPropertyValue(variable).trim(),
    name,
  );
}

/** The person's own name for a theme in the facts of the list: *Starts from Dusk, dark*. */
export function themeRow(page: Page, name: string): Locator {
  return page.locator(`[data-part='${part.themeListItem}']`).filter({ hasText: name });
}
