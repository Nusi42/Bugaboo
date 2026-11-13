import { expect, type Page } from "@playwright/test";

import { everyGuestScreen, everyScreen, screenValues, type ScreenAddress } from "./every-screen.ts";
import { signIn } from "./flows.ts";
import { sampleAdministrator } from "./sample.ts";
import { inOrder } from "./sequence.ts";
import { type Instance } from "./server.ts";

import { pixelsOf } from "./tokens.ts";
import { paths } from "../../src/shared/paths.ts";

/**
 * The size of targets for a finger (UX §7.4, §8; UX-02): every control a finger presses is at least as high and as
 * wide as `size-control` says with touch. The measure takes the size from the theme file and only checks it against
 * the least that the UX concept writes down (`TOUCH_PX`).
 */
export function touchTarget(page: Page): Promise<number> {
  return pixelsOf(page, "--size-control");
}

/** A control that is smaller than a finger needs: the part the theme file would fix, its name and its box. */
export interface SmallTarget {
  readonly part: string;
  readonly within: string;
  readonly name: string;
  readonly width: number;
  readonly height: number;
}

/** A box on the page, in pixels. */
interface Size {
  readonly width: number;
  readonly height: number;
}

/** What the page says about one control a finger presses. */
interface TargetFacts {
  readonly part: string;
  /** The part it stands in: where to look for it on the screen. */
  readonly within: string;
  readonly name: string;
  /** Its own box. */
  readonly box: Size;
  /**
   * The box a finger hits: the label around a box to tick or a choice (the label is pressed), the box of the row a
   * link of a row covers (`::after`, `link[variant=row]`), the box with an edge that the control fills, else its own.
   */
  readonly hit: Size;
  /** Hidden, covered by a window (`inert`), or a box that only a screen reader reads. */
  readonly unseen: boolean;
  /** A link that flows in a line of words: WCAG 2.5.8 leaves it out, the sentence keeps the room. */
  readonly inSentence: boolean;
}

/** What a finger presses, by element and by role. */
const pressed = [
  "a[href]",
  "button",
  "input:not([type='hidden'])",
  "select",
  "textarea",
  "summary",
  "[role='button']",
  "[role='link']",
  "[role='checkbox']",
  "[role='radio']",
  "[role='switch']",
  "[role='tab']",
  "[role='menuitem']",
  "[role='option']",
  "[role='slider']",
].join(",");

/** How much less than the target a box may measure: the browser rounds sizes to parts of a pixel. */
const ROUNDING_PX = 0.5;

/** The smallest box that is drawn at all: what is smaller is a box only a screen reader reads. */
const DRAWN_PX = 2;

/** Where a control is and what it is; the facts of whether it is seen come from `seenOf`, in the same order. */
type PlaceFacts = Omit<TargetFacts, "unseen" | "inSentence">;

/** The boxes of every control of the page a finger presses, and the box a finger hits for each. */
function placesOf(page: Page): Promise<PlaceFacts[]> {
  return page.evaluate(
    (selector: string) =>
      [...document.querySelectorAll<HTMLElement>(selector)].map((target): PlaceFacts => {
        const box = target.getBoundingClientRect();
        const label = target instanceof HTMLInputElement ? target.closest("label") : null;
        const after = getComputedStyle(target, "::after").position === "absolute";
        // A control that fills the box around it is hit through that box's edge too: a field without an edge of its
        // own in a search field, a button that fills the chip of a filter.
        const frame = target.parentElement;
        const edged =
          frame !== null &&
          getComputedStyle(frame).borderTopStyle !== "none" &&
          box.height + 0.5 >= frame.clientHeight;
        const hit = (
          label ??
          (after ? target.offsetParent : null) ??
          (edged ? frame : null) ??
          target
        ).getBoundingClientRect();
        const around = frame?.closest<HTMLElement>("[data-part]")?.dataset;
        const named = (word: string | undefined, kind: string | undefined): string =>
          `${word ?? target.tagName.toLowerCase()}${kind === undefined ? "" : `[${kind}]`}`;
        return {
          part: named(target.dataset["part"], target.dataset["variant"]),
          within: named(around?.["part"], around?.["variant"]),
          name: target.getAttribute("aria-label") ?? (target.textContent ?? "").trim().slice(0, 40),
          box: { width: box.width, height: box.height },
          hit: { width: hit.width, height: hit.height },
        };
      }),
    pressed,
  );
}

/** Whether each control of the page is seen, and whether it is a link in a sentence, in the order of `placesOf`. */
function seenOf(page: Page): Promise<Pick<TargetFacts, "unseen" | "inSentence">[]> {
  return page.evaluate(
    ({ selector, drawn }: { readonly selector: string; readonly drawn: number }) =>
      [...document.querySelectorAll<HTMLElement>(selector)].map((target) => {
        const style = getComputedStyle(target);
        const box = target.getBoundingClientRect();
        // Only a screen reader reads a box that it, or a box around it, cuts away (`VisuallyHidden`, `hidden-text`).
        let clipped = false;
        for (let up: HTMLElement | null = target; up !== null; up = up.parentElement) {
          clipped ||= getComputedStyle(up).clipPath === "inset(50%)";
        }
        let words = false;
        for (const node of target.parentElement?.childNodes ?? []) {
          words ||= node.nodeType === Node.TEXT_NODE && (node.textContent ?? "").trim() !== "";
        }
        const labelled = target instanceof HTMLInputElement && target.closest("label") !== null;
        return {
          unseen:
            target.closest("[inert], [aria-hidden='true']") !== null ||
            style.visibility !== "visible" ||
            (!labelled && (box.width <= drawn || box.height <= drawn || clipped)),
          inSentence: target.tagName === "A" && style.display === "inline" && words,
        };
      }),
    { selector: pressed, drawn: DRAWN_PX },
  );
}

/** The facts of every control of the page a finger presses. */
async function factsOf(page: Page): Promise<TargetFacts[]> {
  const [places, seen] = [await placesOf(page), await seenOf(page)];
  return places.map((place, at): TargetFacts => ({
    part: place.part,
    within: place.within,
    name: place.name,
    box: place.box,
    hit: place.hit,
    unseen: seen[at]?.unseen ?? true,
    inSentence: seen[at]?.inSentence ?? false,
  }));
}

/**
 * The controls of the page that a finger presses and that are smaller than `least` in one direction. What is not a
 * target of its own is left out, as WCAG 2.5.8 leaves it out: a link inside a sentence, what is hidden or covered by a
 * window, and a box that only a screen reader reads. The link of a row that reaches over the whole row (R8) is measured
 * by the row it covers.
 */
export async function smallTargets(page: Page, least: number): Promise<SmallTarget[]> {
  const wanted = least - ROUNDING_PX;
  return (await factsOf(page))
    .filter((fact) => !fact.unseen && !fact.inSentence)
    .filter((fact) => fact.hit.width < wanted || fact.hit.height < wanted)
    .map(({ part, within, name, hit }) => ({
      part,
      within,
      name,
      width: Math.round(hit.width),
      height: Math.round(hit.height),
    }));
}

/** The small targets of one screen of the route table. */
export interface ScreenTargets {
  readonly screen: string;
  readonly targets: readonly SmallTarget[];
}

/** The least size of a target with touch that UX §7.4 and §8 write down: `size-control` with touch is at least it. */
const TOUCH_PX = 44;

/** Waits until a screen has drawn what it reads: its title, and no read on its way. */
async function opened(page: Page, address: string): Promise<void> {
  await page.goto(address);
  await expect(page.locator("h1, [data-part='top-bar.title']").first()).toBeVisible();
  await page.waitForLoadState("networkidle");
}

/** The small targets of each screen, opened one after the other. */
async function measured(
  page: Page,
  screens: readonly ScreenAddress[],
  least: number,
): Promise<ScreenTargets[]> {
  const found: ScreenTargets[] = [];
  await inOrder(
    screens.map((screen) => async () => {
      await opened(page, screen.address);
      found.push({ screen: screen.path, targets: await smallTargets(page, least) });
    }),
  );
  return found;
}

/**
 * Opens every screen before sign-in as nobody, then every screen of the route table as the sample's administrator
 * sees it (the most controls), in the window of the page, and gives the small targets of each. The size it measures
 * against is the touch value of the theme file, at least the one of the UX concept, or it measures nothing.
 */
export async function smallTargetsOfEveryScreen(
  page: Page,
  server: Instance,
): Promise<readonly ScreenTargets[]> {
  const values = await screenValues(server);
  await page.goto(paths.signIn);
  const least = await touchTarget(page);
  expect(least).toBeGreaterThanOrEqual(TOUCH_PX);
  const guests = await measured(page, everyGuestScreen(), least);
  await signIn(page, sampleAdministrator());
  return [...guests, ...(await measured(page, everyScreen(values), least))];
}
