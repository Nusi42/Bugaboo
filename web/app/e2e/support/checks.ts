import { AxeBuilder } from "@axe-core/playwright";
import { t } from "@bugaboo/messages";
import { expect, type Page } from "@playwright/test";

import { pageAttributes } from "../../../packages/ui/src/tokens.ts";
import { inOrder } from "./sequence.ts";
import { type SettingChoice, settingChoices } from "@bugaboo/api-client";

/** One of the built-in themes: the id on the page and the name people read. */
export interface Theme {
  readonly id: SettingChoice<"appearanceLightTheme" | "appearanceDarkTheme">;
  readonly label: string;
}

/** The four built-in themes, by the id on the page and the name people read. */
export const themes: readonly Theme[] = [
  ...settingChoices.appearanceLightTheme,
  ...settingChoices.appearanceDarkTheme,
].map((id) => ({ id, label: t(`theme.${id}`) }));

/** The page shows this theme: the `look` attribute of the page frame (`pageAttributes`). */
export async function expectLook(page: Page, look: string): Promise<void> {
  await expect(page.locator("html")).toHaveAttribute(pageAttributes.look, look);
}

/**
 * Picks a theme with the choice at the bottom of the pages before sign-in, as a person would. The
 * signed-in app keeps the choice in *My settings*, so there the theme is put on the page directly.
 */
export async function chooseTheme(page: Page, theme: Theme): Promise<void> {
  const choice = page.getByRole("radio", { name: theme.label, exact: true });
  if ((await choice.count()) > 0) {
    await choice.check();
  } else {
    await putOnTheme(page, theme);
  }
  await expectLook(page, theme.id);
}

/**
 * Shows a theme the way a person does in *My settings › Appearance*: *Always light* or *Always
 * dark*, and the theme among the samples of the light or the dark themes. The choice is kept on
 * the server for the person, and the page shows the theme once it is saved.
 */
export async function pickTheme(page: Page, theme: Theme): Promise<void> {
  const dark = settingChoices.appearanceDarkTheme.some((id) => id === theme.id);
  await page
    .getByRole("radio", {
      name: t(dark ? "appearance.mode.dark" : "appearance.mode.light"),
      exact: true,
    })
    .check();
  const group = page.getByRole("group", {
    name: t(dark ? "appearance.dark" : "appearance.light"),
    exact: true,
  });
  await group.getByRole("radio", { name: theme.label, exact: true }).check();
}

/**
 * Puts a theme on the page without the choice at the bottom, which a question that covers the
 * page makes unreachable. The colors follow `data-look` at once.
 */
async function putOnTheme(page: Page, theme: Theme): Promise<void> {
  await page.evaluate((look) => {
    document.documentElement.dataset["look"] = look;
  }, theme.id);
}

/** The screen has no accessibility violation that the checks of axe can find (WCAG 2.2 AA). */
export async function expectNoViolations(page: Page): Promise<void> {
  const results = await new AxeBuilder({ page })
    .withTags(["wcag2a", "wcag2aa", "wcag21a", "wcag21aa", "wcag22aa"])
    .analyze();
  expect(results.violations).toEqual([]);
}

/** The page is not wider than the window, so nobody has to scroll sideways. */
export async function expectNoSidewaysScroll(page: Page): Promise<void> {
  const overflow = await page.evaluate(
    () => document.documentElement.scrollWidth - document.documentElement.clientWidth,
  );
  expect(overflow).toBeLessThanOrEqual(0);
}

/** Places where the items of a list, a group of buttons or a group of choices stand side by side. */
const ROWS_OF_ITEMS =
  "ul, ol, [role='radiogroup'], [role='group'], [role='tablist'], [data-part='list-row']";

/** Where an item draws: its own box, and its text, which can reach out of a box that is too narrow. */
interface Drawn {
  readonly text: string;
  readonly left: number;
  readonly right: number;
  readonly top: number;
  readonly bottom: number;
}

/**
 * Nothing on the screen is drawn over its neighbour: what the items of every list and group draw
 * (box and text) is apart. A grid whose columns are too narrow draws codes on top of each other
 * while their boxes stay side by side, so the text counts too.
 */
export async function expectNoOverlap(page: Page): Promise<void> {
  const overlaps = await page.evaluate((selector) => {
    const found: string[] = [];
    for (const row of document.querySelectorAll(selector)) {
      const drawn: Drawn[] = [];
      for (const item of row.children) {
        const box = item.getBoundingClientRect();
        const range = document.createRange();
        range.selectNodeContents(item);
        // A cell that clips its text (with an ellipsis) draws no more than its box.
        const text =
          getComputedStyle(item).overflowX === "visible" ? range.getBoundingClientRect() : box;
        if (box.width > 0 && box.height > 0) {
          drawn.push({
            text: (item.textContent ?? "").trim(),
            left: Math.min(box.left, text.width > 0 ? text.left : box.left),
            right: Math.max(box.right, text.width > 0 ? text.right : box.right),
            top: Math.min(box.top, text.height > 0 ? text.top : box.top),
            bottom: Math.max(box.bottom, text.height > 0 ? text.bottom : box.bottom),
          });
        }
      }
      for (const [index, first] of drawn.entries()) {
        for (const second of drawn.slice(index + 1)) {
          const across = Math.min(first.right, second.right) - Math.max(first.left, second.left);
          const down = Math.min(first.bottom, second.bottom) - Math.max(first.top, second.top);
          if (across > 1 && down > 1) {
            found.push(`"${first.text}" and "${second.text}"`);
          }
        }
      }
    }
    return found;
  }, ROWS_OF_ITEMS);
  expect(overlaps).toEqual([]);
}

/** Things a person presses, which have their own name besides the text they show. */
const NAMED_CONTROLS = "button[aria-label], a[aria-label], [role='button'][aria-label]";

/**
 * What a button or link says in words is part of the name it has for a screen reader (WCAG 2.5.3,
 * label in name): a person who says "Sign out" to voice control must hit the button that shows
 * "Sign out". Every button or link with its own `aria-label` has its visible text in that label.
 */
export async function expectVisibleTextInName(page: Page): Promise<void> {
  const mismatches = await page.evaluate((selector) => {
    const found: string[] = [];
    for (const element of document.querySelectorAll(selector)) {
      const visible = (element.textContent ?? "").trim().toLowerCase();
      const name = (element.getAttribute("aria-label") ?? "").toLowerCase();
      if (visible !== "" && !name.includes(visible)) {
        found.push(`"${visible}" is not in "${name}"`);
      }
    }
    return found;
  }, NAMED_CONTROLS);
  expect(mismatches).toEqual([]);
}

/** The open screen is accessible and does not scroll sideways, in every theme, picked as a person does. */
export function expectGoodInEveryTheme(page: Page): Promise<void> {
  return inOrder(
    themes.map((theme) => async () => {
      await chooseTheme(page, theme);
      await expectNoViolations(page);
      await expectNoSidewaysScroll(page);
      await expectNoOverlap(page);
      await expectVisibleTextInName(page);
    }),
  );
}

/** The same without the choice of theme: for screens with a question open over the page. */
export function expectGoodInEveryLook(page: Page): Promise<void> {
  return inOrder(
    themes.map((theme) => async () => {
      await putOnTheme(page, theme);
      await expectNoViolations(page);
      await expectNoSidewaysScroll(page);
      await expectNoOverlap(page);
      await expectVisibleTextInName(page);
    }),
  );
}
