import { expect, type Page } from "@playwright/test";

/** The things in the open dialog that take the focus, selected in the page. */
const takersSelector = 'input, select, textarea, button, a[href], [tabindex]:not([tabindex="-1"])';

/**
 * Every thing in the open dialog that takes the focus gets a focus ring it did not have before
 * (WCAG 2.4.7): an outline that has a colour, or a shadow that changes with the focus. A resting
 * shadow, an outline of no colour and `outline: none` are no ring.
 */
async function expectRingsDrawn(page: Page): Promise<void> {
  // A key press first: the browser then draws the ring of a focus that a script sets, as for the keyboard.
  await page.keyboard.press("Shift");
  const found = await page.evaluate((selector) => {
    const failures: string[] = [];
    const dialog = [...document.querySelectorAll('[role="dialog"], [role="alertdialog"]')].at(-1);
    if (dialog === undefined) {
      return ["There is no open dialog."];
    }
    for (const taker of dialog.querySelectorAll<HTMLElement>(selector)) {
      const ringed = taker.closest('[data-part="search-field.box"]') ?? taker;
      taker.blur();
      const shadowBefore = getComputedStyle(ringed).boxShadow;
      taker.focus();
      const style = getComputedStyle(ringed);
      const coloured =
        style.outlineColor !== "transparent" && !/^rgba\([^)]*,\s*0\)$/u.test(style.outlineColor);
      const outlined =
        style.outlineStyle !== "none" &&
        Number(style.outlineWidth.replace("px", "")) > 0 &&
        coloured;
      const shadowed = style.boxShadow !== "none" && style.boxShadow !== shadowBefore;
      const shown = taker.getClientRects().length > 0 && !taker.hasAttribute("disabled");
      if (shown && !outlined && !shadowed) {
        failures.push(
          `${taker.tagName} ${(taker.textContent ?? "").trim().slice(0, 24)}: no focus ring is drawn`,
        );
      }
    }
    return failures;
  }, takersSelector);
  expect(found).toEqual([]);
}

/**
 * A focus ring drawn outside the edge of what takes the focus is not cut off on any of its four
 * sides by a box that scrolls or hides on the way up to the dialog. A ring drawn inside the edge
 * (an offset below nothing) cannot be cut.
 */
async function expectRingsNotCut(page: Page): Promise<void> {
  const found = await page.evaluate((selector) => {
    const failures: string[] = [];
    const dialog = [...document.querySelectorAll('[role="dialog"], [role="alertdialog"]')].at(-1);
    for (const taker of dialog?.querySelectorAll<HTMLElement>(selector) ?? []) {
      const ringed = taker.closest('[data-part="search-field.box"]') ?? taker;
      taker.scrollIntoView({ block: "nearest" });
      taker.focus();
      const style = getComputedStyle(ringed);
      const offset = Number(style.outlineOffset.replace("px", ""));
      const reach = Number(style.outlineWidth.replace("px", "")) + offset;
      const ring = ringed.getBoundingClientRect();
      const shown = taker.getClientRects().length > 0 && !taker.hasAttribute("disabled");
      const above: Element[] = [];
      for (let up = ringed.parentElement; up !== null; up = up.parentElement) {
        above.push(up);
        if (up === dialog) {
          break;
        }
      }
      for (const up of shown && offset >= 0 ? above : []) {
        const { overflowX, overflowY } = getComputedStyle(up);
        const frame = up.getBoundingClientRect();
        const cut =
          ring.left - reach < frame.left - 0.5 ||
          ring.right + reach > frame.right + 0.5 ||
          ring.top - reach < frame.top - 0.5 ||
          ring.bottom + reach > frame.bottom + 0.5;
        if (cut && (overflowX !== "visible" || overflowY !== "visible")) {
          failures.push(
            `${taker.tagName} ${(taker.textContent ?? "").trim().slice(0, 24)}: the focus ring is cut by ${up.tagName}`,
          );
        }
      }
    }
    return failures;
  }, takersSelector);
  expect(found).toEqual([]);
}

/** The focus ring of everything in the open dialog is drawn and whole (WCAG 2.4.7, the review of M5, S19). */
export async function expectRingsWhole(page: Page): Promise<void> {
  await expectRingsDrawn(page);
  await expectRingsNotCut(page);
}
