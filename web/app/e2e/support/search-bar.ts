import { t } from "@bugaboo/messages";
import { type Locator, type Page } from "@playwright/test";

import { ada, type Person } from "./accounts.ts";
import { chooseTheme, themes } from "./checks.ts";
import { expect } from "./fixtures.ts";
import { signIn } from "./flows.ts";
import { layoutOf, onThisLayout } from "./layout.ts";
import { shotWholeAs } from "./screenshots.ts";
import { inOrder } from "./sequence.ts";

const MILESTONE = "M5/search";

/** Takes the open screen in the four themes, named for the screen and the window it is shown in. */
export async function inFourThemes(page: Page, name: string): Promise<void> {
  const layout = layoutOf(page);
  await inOrder(
    themes.map((theme) => async () => {
      await chooseTheme(page, theme);
      await shotWholeAs(page, `${name}-${layout}`, MILESTONE, theme.id);
    }),
  );
}

export async function openSearch(page: Page, text: string, person: Person = ada): Promise<void> {
  await signIn(page, person);
  await page.goto(`/search?s=${encodeURIComponent(text)}`);
  await expect(page.getByRole("heading", { level: 1, name: t("search.page.title") })).toBeVisible();
}

/** On a phone the bar is one button that opens a sheet. */
export async function openBar(page: Page): Promise<void> {
  if (layoutOf(page) === "phone") {
    await page.getByRole("button", { name: /^Filters/u }).click();
  }
}

/** The key `/` went to the bar: to its first control on a computer, into its sheet on a phone. */
export async function expectBarKeyWorked(page: Page): Promise<void> {
  const first = async (): Promise<void> => {
    await expect(page.getByRole("button", { name: t("search.bar.add-filter") })).toBeFocused();
  };
  await onThisLayout(page, {
    computer: first,
    tablet: first,
    phone: async () => {
      await expect(page.getByRole("dialog", { name: t("search.bar.filters") })).toBeVisible();
    },
  });
}

/** One mistake of a search as the route tells it. */
export interface RouteMistake {
  readonly key: string;
  readonly args: Readonly<Record<string, string>>;
  readonly start: number;
  readonly end: number;
}

export const noMistake: RouteMistake = { key: "", args: {}, start: -1, end: -1 };

/** The mistakes in the body of an answer, whatever the body is. */
export function mistakesOf(body: unknown): readonly RouteMistake[] {
  const found: unknown =
    typeof body === "object" && body !== null ? Reflect.get(body, "mistakes") : undefined;
  return Array.isArray(found) ? (found as readonly RouteMistake[]) : [];
}

/** The box of Advanced search. */
export function box(page: Page) {
  return page.getByRole("textbox", { name: t("search.bar.advanced-label") });
}

/** The text the address holds as the search. */
export function addressText(page: Page): string {
  return new URL(page.url()).searchParams.get("s") ?? "";
}

/** On a phone the sheet of the bar is closed with its button, so the page behind it can be pressed. */
export async function closeBar(page: Page): Promise<void> {
  if (layoutOf(page) === "phone") {
    await page
      .getByRole("dialog", { name: t("search.bar.filters") })
      .getByRole("button", { name: t("common.close") })
      .click();
  }
}

// The folder the pictures go to: the one flag of every picture spec, re-exported so a spec that already imports this file takes no other import.
export { screenshotFolder } from "./screenshots.ts";

/** The rows of a scrolling list that are only partly inside it (a row cut in half), as text; none when every row shows whole or not at all. */
export function rowsCutOff(list: Locator): Promise<readonly string[]> {
  return list.evaluate((element) => {
    const frame = element.getBoundingClientRect();
    const tolerance = 0.5;
    const cut: string[] = [];
    for (const child of element.children) {
      const { top, bottom } = child.getBoundingClientRect();
      const inside = bottom > frame.top && top < frame.bottom;
      const whole = top >= frame.top - tolerance && bottom <= frame.bottom + tolerance;
      if (inside && !whole) {
        cut.push(`${top}-${bottom} in ${frame.top}-${frame.bottom}`);
      }
    }
    return cut;
  });
}

/** Where the words of an element start (not where the box that holds them starts), in pixels from the left of the page. */
export function wordsStartOf(element: Locator): Promise<number> {
  return element.first().evaluate((node) => {
    const range = document.createRange();
    range.selectNodeContents(node);
    return range.getBoundingClientRect().left;
  });
}

/** In the window of an *Any of*: *Add a filter*, the field *Title*, the words, and the window's own *Add filter*. */
export async function addTitleFilter(page: Page, anyOf: Locator, words: string): Promise<void> {
  await anyOf.getByRole("button", { name: t("search.bar.any-of-add") }).click();
  await page.getByRole("button", { name: t("search.bar.filter-by", { field: "Title" }) }).click();
  await page.getByRole("textbox", { name: t("search.bar.value") }).fill(words);
  await page
    .getByRole("alertdialog", { name: "Title" })
    .getByRole("button", { name: t("search.bar.add-filter-confirm") })
    .click();
}

/** Where the box of an element starts, in pixels from the left of the page. */
export async function leftEdgeOf(element: Locator): Promise<number> {
  const found = await element.boundingBox();
  return found === null ? Number.NaN : found.x;
}
