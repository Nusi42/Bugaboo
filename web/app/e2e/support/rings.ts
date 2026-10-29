import { expect, type Page } from "@playwright/test";

/** The things in the open dialog that take the focus. */
const takersSelector = 'input, select, textarea, button, a[href], [tabindex]:not([tabindex="-1"])';

/** The least alpha of an outline colour that can be seen as a ring. */
const leastAlpha = 0.1;

/** A box on the page, in pixels. */
interface Edges {
  readonly left: number;
  readonly right: number;
  readonly top: number;
  readonly bottom: number;
}

/** What the page says about one thing that takes the focus, once it has the focus. */
interface RingFacts {
  /** The tag and the first words of the text, for the message. */
  readonly name: string;
  readonly outlineStyle: string;
  readonly outlineWidth: number;
  readonly outlineColor: string;
  readonly outlineOffset: number;
  /** The box that wears the ring: the box of a search field, or the thing itself. */
  readonly box: Edges;
  /** The boxes that scroll or hide what is outside them, on the way up to the dialog. */
  readonly clips: readonly (Edges & { readonly tag: string })[];
}

/** The alpha of a computed colour: `rgb(0 0 0 / 50%)`, `rgba(0, 0, 0, 0.5)`, `oklch(… / 0.5)`; 1 when it names none. */
function alphaOf(colour: string): number {
  const slashed = /\/\s*([\d.]+)(%?)\s*\)$/u.exec(colour);
  if (slashed !== null) {
    return Number(slashed[1]) / (slashed[2] === "%" ? 100 : 1);
  }
  const commas = /^rgba\([^,]+,[^,]+,[^,]+,\s*([\d.]+)\s*\)$/u.exec(colour);
  return commas === null ? 1 : Number(commas[1]);
}

/**
 * Focuses every thing in the open dialog that takes the focus, one after the other, and gives the
 * facts of its ring. Shown things only: a hidden or disabled one has no ring to see.
 */
async function ringFacts(page: Page): Promise<readonly RingFacts[]> {
  // A key press first: the browser then draws the ring of a focus that a script sets, as for the keyboard.
  await page.keyboard.press("Shift");
  return page.evaluate((selector) => {
    const dialog = [...document.querySelectorAll('[role="dialog"], [role="alertdialog"]')].at(-1);
    const facts: RingFacts[] = [];
    for (const taker of dialog?.querySelectorAll<HTMLElement>(selector) ?? []) {
      if (taker.getClientRects().length === 0 || taker.hasAttribute("disabled")) {
        continue;
      }
      taker.scrollIntoView({ block: "nearest" });
      taker.focus();
      const ringed = taker.closest('[data-part="search-field.box"]') ?? taker;
      const style = getComputedStyle(ringed);
      const own = ringed.getBoundingClientRect();
      const clips: (Edges & { tag: string })[] = [];
      for (let up = ringed.parentElement; up !== null; up = up.parentElement) {
        const { overflowX, overflowY } = getComputedStyle(up);
        if (overflowX !== "visible" || overflowY !== "visible") {
          const { left, right, top, bottom } = up.getBoundingClientRect();
          clips.push({ left, right, top, bottom, tag: up.tagName });
        }
        if (up === dialog) {
          break;
        }
      }
      facts.push({
        name: `${taker.tagName} ${(taker.textContent ?? "").trim().slice(0, 24)}`,
        outlineStyle: style.outlineStyle,
        outlineWidth: Number(style.outlineWidth.replace("px", "")),
        outlineColor: style.outlineColor,
        outlineOffset: Number(style.outlineOffset.replace("px", "")),
        box: { left: own.left, right: own.right, top: own.top, bottom: own.bottom },
        clips,
      });
    }
    return facts;
  }, takersSelector);
}

/** The ring of a thing that takes the focus is there: an outline with a style, a width and a colour that can be seen. */
function undrawn(facts: readonly RingFacts[]): readonly string[] {
  return facts
    .filter(
      (one) =>
        one.outlineStyle === "none" ||
        !(one.outlineWidth > 0) ||
        alphaOf(one.outlineColor) < leastAlpha,
    )
    .map((one) => `${one.name}: no focus ring is drawn`);
}

/** The ring, which reaches its width plus its offset beyond the edge, is cut by a box that clips: none for a ring drawn inside the edge. */
function cut(facts: readonly RingFacts[]): readonly string[] {
  const slack = 0.5;
  return facts.flatMap((one) => {
    const reach = one.outlineWidth + one.outlineOffset;
    return one.outlineOffset < 0
      ? []
      : one.clips
          .filter(
            (clip) =>
              one.box.left - reach < clip.left - slack ||
              one.box.right + reach > clip.right + slack ||
              one.box.top - reach < clip.top - slack ||
              one.box.bottom + reach > clip.bottom + slack,
          )
          .map((clip) => `${one.name}: the focus ring is cut by ${clip.tag}`);
  });
}

/**
 * The focus ring of everything in the open dialog is drawn and whole (WCAG 2.4.7, the review of M5,
 * S19). The rings of this app are outlines, so a ring is drawn when its outline has a style, a width
 * and a colour of at least 0.1 alpha (`outline: none`, a transparent outline and a shadow that is
 * there before the focus are no ring), and it is whole when no box that scrolls or hides on the way
 * up to the dialog cuts what reaches beyond the edge (the width plus the offset) on any side.
 */
export async function expectRingsWhole(page: Page): Promise<void> {
  const facts = await ringFacts(page);
  expect(facts.length > 0 ? [] : ["There is nothing to focus in the open dialog."]).toEqual([]);
  expect([...undrawn(facts), ...cut(facts)]).toEqual([]);
}
