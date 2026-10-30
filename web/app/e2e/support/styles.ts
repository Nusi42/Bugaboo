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

/**
 * The ink of one line of text in the font an element is drawn in, against the same line in the font of the page, both
 * in the regular weight: about 1 for a face as light as the text font, about 1.3 for one whose regular weight reads bold.
 */
export function inkAgainstText(element: Locator): Promise<number> {
  return element.evaluate((node) => {
    const line = "priority is WEB-4QR2G";
    const width = 400;
    const height = 40;
    const inkOf = (family: string): number => {
      const canvas = document.createElement("canvas");
      canvas.width = width;
      canvas.height = height;
      const draw = canvas.getContext("2d");
      if (draw === null) {
        return Number.NaN;
      }
      draw.fillStyle = "#fff";
      draw.fillRect(0, 0, width, height);
      draw.fillStyle = "#000";
      draw.font = `400 14px ${family}`;
      draw.fillText(line, 2, 28);
      const pixels = draw.getImageData(0, 0, width, height).data;
      let ink = 0;
      for (let at = 0; at < pixels.length; at += 4) {
        ink += 255 - (pixels[at] ?? 255);
      }
      return ink;
    };
    return (
      inkOf(getComputedStyle(node).fontFamily) / inkOf(getComputedStyle(document.body).fontFamily)
    );
  });
}
