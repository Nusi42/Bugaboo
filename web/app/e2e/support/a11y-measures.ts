import { type Page } from "@playwright/test";

import { screenValues } from "./every-screen.ts";
import { signIn } from "./flows.ts";
import { sampleAdministrator } from "./sample.ts";
import { everyGuestScreen, everyScreen, opened, type ScreenAddress } from "./screen-list.ts";
import { readerOnly } from "./reader-only.ts";
import { inOrder } from "./sequence.ts";
import { type Instance } from "./server.ts";
import { viewports } from "../../../tooling/viewports.ts";

// The measures of WCAG 2.2 AA that a machine can take on a whole screen (COMP-15, EN 301 549 clause 9): what scrolls
// sideways at 320 px (1.4.10) and what text is cut off at 200 % or with wide text spacing (1.4.4, 1.4.12); the walk of the
// keyboard is in a11y-focus.ts. The four built-in themes set colors only (ADR-0026), so a measure of
// sizes and places is taken in one of them; colors are axe's business in every theme (a11y.spec.ts).

/** What a screen got wrong under one measure, by the route of the screen. */
export interface ScreenFindings {
  readonly screen: string;
  readonly found: readonly string[];
}

/**
 * The parts that may scroll sideways at 320 px: content that needs two dimensions to be read, which WCAG 1.4.10 names
 * as an exception (a data table, a chart over time). The issue lists are tables of columns, the matrix of access is a
 * table of roles and permissions, and the timeline is a chart of days.
 */
export const twoDimensional = [
  "[data-part='list-frame']",
  "[data-part='list-scroll']",
  "[data-part='access.scroll']",
  "[data-part^='timeline']",
  "table",
].join(", ");

/** How far a box may reach past its room before it counts: the browser rounds sizes to parts of a pixel. */
const ROUNDING_PX = 1;

/**
 * What scrolls sideways on the open screen: the page itself, and every box that scrolls sideways or hides what is
 * wider than it, unless it is content in two dimensions (`twoDimensional`). Each is named by its part.
 */
export function sidewaysScroll(page: Page): Promise<string[]> {
  return page.evaluate(
    ({ allowed, rounding }: Bounds) => {
      const found: string[] = [];
      const root = document.documentElement;
      if (root.scrollWidth > root.clientWidth + rounding) {
        found.push(`the page (${String(root.scrollWidth)} px in ${String(root.clientWidth)})`);
      }
      for (const box of document.querySelectorAll<HTMLElement>("body *")) {
        const style = getComputedStyle(box);
        const scrolls = style.overflowX === "auto" || style.overflowX === "scroll";
        if (
          scrolls &&
          box.clientWidth > 0 &&
          box.scrollWidth > box.clientWidth + rounding &&
          box.closest(allowed) === null
        ) {
          found.push(
            `${box.dataset["part"] ?? box.tagName.toLowerCase()} (${String(box.scrollWidth)} px in ${String(box.clientWidth)})`,
          );
        }
      }
      return found;
    },
    { allowed: twoDimensional, rounding: ROUNDING_PX },
  );
}

/** Every element of the page, in its order: what `cutText` looks at. */
const EVERY_ELEMENT = "body *";

/** What `cutText` is given in the page: whether only a screen reader reads each element, and the rounding. */
interface CutBounds {
  readonly hidden: readonly boolean[];
  readonly rounding: number;
}

/**
 * The text of the open screen that is cut off: a box with words of its own that hides what does not fit, without
 * saying so with an ellipsis or a clamp of lines (a name cut with "…" is shown whole where it is opened). A box that
 * only a screen reader reads is left out. Content in two dimensions is measured too: WCAG 1.4.4 has no exception for
 * it, only 1.4.10 lets it scroll sideways.
 */
export async function cutText(page: Page): Promise<string[]> {
  const hidden = await readerOnly(page, EVERY_ELEMENT);
  return page.evaluate(
    ({ hidden: onlyRead, rounding, every }: CutBounds & { readonly every: string }) => {
      const found: string[] = [];
      const hiding = new Set(["hidden", "clip"]);
      for (const [at, box] of [...document.querySelectorAll<HTMLElement>(every)].entries()) {
        let words = false;
        for (const node of box.childNodes) {
          words ||= node.nodeType === Node.TEXT_NODE && (node.textContent ?? "").trim() !== "";
        }
        if (!words || box.closest("[aria-hidden='true']")) {
          continue;
        }
        const style = getComputedStyle(box);
        const marked = style.textOverflow === "ellipsis" || style.webkitLineClamp !== "none";
        if (onlyRead[at] === true || marked || style.visibility !== "visible") {
          continue;
        }
        const sideways =
          hiding.has(style.overflowX) && box.scrollWidth > box.clientWidth + rounding;
        const down = hiding.has(style.overflowY) && box.scrollHeight > box.clientHeight + rounding;
        if (sideways || down) {
          found.push(
            `"${(box.textContent ?? "").trim().slice(0, 40)}" in ${box.dataset["part"] ?? box.tagName.toLowerCase()}`,
          );
        }
      }
      return found;
    },
    { hidden, rounding: ROUNDING_PX, every: EVERY_ELEMENT },
  );
}

/**
 * The text spacing of WCAG 1.4.12 that a person may set with their own style sheet: lines 1.5 times the size of the
 * text, letters 0.12 and words 0.16 of it apart, and paragraphs 2 times it apart.
 */
const WIDE_SPACING = `
*, *::before, *::after {
  line-height: 1.5 !important;
  letter-spacing: 0.12em !important;
  word-spacing: 0.16em !important;
}
p { margin-bottom: 2em !important; }
`;

/**
 * Puts the wide text spacing of WCAG 1.4.12 on the open page until it is loaded again, as a person's own style sheet
 * does. A style sheet made in the page, not a `<style>` element, which the security policy of Bugaboo refuses.
 */
export async function widenTextSpacing(page: Page): Promise<void> {
  await page.evaluate((rules) => {
    const sheet = new CSSStyleSheet();
    sheet.replaceSync(rules);
    document.adoptedStyleSheets = [...document.adoptedStyleSheets, sheet];
  }, WIDE_SPACING);
}

/** A computer's window at 200 % (WCAG 1.4.4): the page has half the room in each direction. */
const ZOOMED = {
  width: viewports.computer.width / 2,
  height: viewports.computer.height / 2,
} as const;

/** Gives the page the room it has at 200 %; a phone, narrower than that already, keeps its own width. */
export async function zoomTo200(page: Page): Promise<void> {
  const own = page.viewportSize() ?? ZOOMED;
  await page.setViewportSize({
    width: Math.min(own.width, ZOOMED.width),
    height: Math.min(own.height, ZOOMED.height),
  });
}

/** The values of one measure that the page is given: what may scroll or clip, and the rounding. */
interface Bounds {
  readonly allowed: string;
  readonly rounding: number;
}

/**
 * Opens every screen before sign-in as nobody, then every screen of the route table as the sample's administrator sees
 * it (the most controls), and takes `measure` on each; a screen with nothing wrong is left out of the answer.
 * `prepare` runs once the screen is open, before the measure.
 */
export async function measureEveryScreen(
  page: Page,
  server: Instance,
  measure: (page: Page) => Promise<readonly string[]>,
  prepare: (page: Page) => Promise<void> = () => Promise.resolve(),
): Promise<ScreenFindings[]> {
  const values = await screenValues(server);
  const found: ScreenFindings[] = [];
  const take = (screens: readonly ScreenAddress[]): Promise<void> =>
    inOrder(
      screens.map((screen) => async () => {
        await opened(page, screen.address);
        await prepare(page);
        const wrong = await measure(page);
        if (wrong.length > 0) {
          found.push({ screen: screen.path, found: wrong });
        }
      }),
    );
  await take(everyGuestScreen());
  await signIn(page, sampleAdministrator());
  await take(everyScreen(values));
  return found;
}
