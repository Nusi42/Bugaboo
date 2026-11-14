import type { Locator, Page } from "@playwright/test";

import { ada } from "./support/accounts.ts";
import { expect, test } from "./support/fixtures.ts";
import { signIn } from "./support/flows.ts";
import { prepareWeb } from "./support/issues.ts";
import { inOrder } from "./support/sequence.ts";
import { expectStill, landmarkOf, movementWhile, partLandmark } from "./support/movement.ts";
import { issueAddress } from "../src/shared/paths.ts";

// The head of a pane folds to one line as its boxes scroll and opens again at their top (UX §6.5): it follows the
// scroll, never moves the scroll by itself, and nothing in the box moves but by the scroll.

test.use({ preparation: { run: prepareWeb } });

/** A low window, so the boxes of the pane are higher than their room. */
const LOW_WINDOW = { width: 1280, height: 520 } as const;

/** How far a person scrolls in one step: less than the head can fold. */
const STEP = 30;

/** The head of the pane on the screen. */
function headOf(page: Page): Locator {
  return page.locator("[data-part='pane-head']");
}

/** How high an element is drawn now. */
function heightOf(element: Locator): Promise<number> {
  return element.evaluate((node) => node.getBoundingClientRect().height);
}

/**
 * Scrolls a box as a person does, waits for the next frame, and says where the box then stands and how far its first
 * element moved on the screen.
 */
function scrollBy(box: Locator, by: number): Promise<{ scrolled: number; moved: number }> {
  return box.evaluate(async (node, step) => {
    const first = node.firstElementChild ?? node;
    const before = first.getBoundingClientRect().top;
    const start = node.scrollTop;
    node.scrollTop = start + step;
    await new Promise<void>((done) => {
      requestAnimationFrame(() => {
        requestAnimationFrame(() => {
          done();
        });
      });
    });
    return {
      scrolled: node.scrollTop - start,
      moved: before - first.getBoundingClientRect().top,
    };
  }, by);
}

/** The head folds as `box` scrolls, the box moves only by the scroll, and the head opens again at the top. */
async function foldsWith(page: Page, box: Locator): Promise<void> {
  const head = headOf(page);
  await expect(head).toBeVisible();
  const whole = await heightOf(head);
  const step = await scrollBy(box, STEP);
  expect(step).toEqual({ scrolled: STEP, moved: STEP });
  const folding = await heightOf(head);
  expect(folding).toBeLessThan(whole);
  // Far down, the head is one line; the scroll stays where the person left it.
  await box.evaluate((node) => {
    node.scrollTop = node.scrollHeight;
  });
  const end = await box.evaluate((node) => node.scrollTop);
  await expect.poll(() => heightOf(head)).toBeLessThan(folding);
  expect(await box.evaluate((node) => node.scrollTop)).toBe(end);
  // Back at the top the head is whole again.
  await box.evaluate((node) => {
    node.scrollTop = 0;
  });
  await expect.poll(() => heightOf(head)).toBeCloseTo(whole, 0);
  expect(await box.evaluate((node) => node.scrollTop)).toBe(0);
}

test("ux_06 the head of an issue folds as its sections or its details scroll, and opens at the top", async ({
  page,
}) => {
  await page.setViewportSize(LOW_WINDOW);
  await signIn(page, ada);
  await page.goto(issueAddress("WEB-1"));
  const split = "[data-part='issue-body'][data-variant~='split']";
  await foldsWith(page, page.locator(`${split} [data-part='issue-main']`));
  await foldsWith(page, page.locator(`${split} [data-part='issue-details']`));
});

/** How many turns of the wheel a person makes, each less than the head can fold. */
const TURNS = 8;

/** Turns the wheel over `box` a few small steps down, then as many up, as a person does. */
async function wheelOver(page: Page, box: Locator): Promise<void> {
  const place = await box.boundingBox();
  await page.mouse.move((place?.x ?? 0) + 8, (place?.y ?? 0) + (place?.height ?? 0) - 8);
  const turns = (way: number) =>
    Array.from({ length: TURNS }, () => () => page.mouse.wheel(0, way));
  await inOrder(turns(STEP));
  await expect.poll(() => box.evaluate((node) => node.scrollTop)).toBe(TURNS * STEP);
  await inOrder(turns(-STEP));
  await expect.poll(() => box.evaluate((node) => node.scrollTop)).toBe(0);
}

test("ux_01 while one box of an issue scrolls and the head folds, nothing else moves in any frame", async ({
  page,
}) => {
  await page.setViewportSize(LOW_WINDOW);
  await signIn(page, ada);
  await page.goto(issueAddress("WEB-1"));
  const split = "[data-part='issue-body'][data-variant~='split']";
  const sections = page.locator(`${split} [data-part='issue-main']`);
  const details = page.locator(`${split} [data-part='issue-details']`);
  await expect(details).toBeVisible();
  await page.waitForLoadState("networkidle");
  const title = await landmarkOf(page.getByRole("heading", { level: 1 }), "the title");
  /** Scrolls one box and measures every frame: the other box keeps the room of the whole head and does not move. */
  const scrollOne = async (scrolled: Locator, other: Locator, part: string): Promise<void> => {
    const still = await landmarkOf(
      other.locator("> *").first(),
      `the start of ${part}'s neighbour`,
    );
    const movement = await movementWhile(
      page,
      [
        title,
        still,
        partLandmark("the sections", "issue-main"),
        partLandmark("the details", "issue-details"),
        partLandmark("the slim list", "focus-view.list"),
      ],
      () => wheelOver(page, scrolled),
    );
    // The head folds and opens again with the scroll (the test above); its title stays where it is.
    expectStill(movement, { scrolls: [part] });
  };
  await scrollOne(sections, details, "issue-main");
  await scrollOne(details, sections, "issue-details");
});
