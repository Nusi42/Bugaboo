import { type Locator, type Page } from "@playwright/test";

import { pageAttributes, type PartName, themeIds } from "../../../packages/ui/src/tokens.ts";
import { inOrder } from "./sequence.ts";

/** The elements of one part of the theme file (`data-part`), on the page or inside an element. */
export function partOn(scope: Page | Locator, name: PartName): Locator {
  return scope.locator(`[data-part="${name}"]`);
}

/**
 * What the browser makes of a value on this page: a color, a length or a function written the way
 * the theme file writes it (`var(--bg-hover)`, `Highlight`). It is how a test says what the theme
 * file promises without writing a value of its own. The probe has a line of every kind, so that
 * widths and thicknesses are worked out too.
 */
export function resolved(page: Page, property: string, value: string): Promise<string> {
  return page.evaluate(
    ([name, text]) => {
      const probe = document.createElement("div");
      probe.style.setProperty("outline-style", "solid");
      probe.style.setProperty("text-decoration-line", "underline");
      probe.style.setProperty(name, text);
      document.body.append(probe);
      const result = getComputedStyle(probe).getPropertyValue(name);
      probe.remove();
      return result;
    },
    [property, value] as const,
  );
}

/** Shows one of the themes on the page, without a trip to the settings. */
export function lookAs(page: Page, look: string): Promise<void> {
  return page.evaluate(
    ([attribute, value]) => {
      document.documentElement.setAttribute(attribute, value);
    },
    [pageAttributes.look, look] as const,
  );
}

/**
 * Puts the focus on an element the way the keyboard does: a key was pressed, so the browser and
 * React Aria both show the ring.
 */
export async function focusByKeyboard(page: Page, element: Locator): Promise<void> {
  await page.keyboard.press("Escape");
  await element.focus();
}

/** Runs a step in each of the themes, one after the other, with the theme on the page. */
export function inEveryLook(
  page: Page,
  step: (look: (typeof themeIds)[number]) => Promise<void>,
): Promise<void> {
  return inOrder(
    themeIds.map((look) => async () => {
      await lookAs(page, look);
      await step(look);
    }),
  );
}
