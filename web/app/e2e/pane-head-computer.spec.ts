import type { Locator } from "@playwright/test";

import { t } from "@bugaboo/messages";

import { ada } from "./support/accounts.ts";
import { expect, test } from "./support/fixtures.ts";
import { signIn } from "./support/flows.ts";
import { prepareWeb } from "./support/issues.ts";
import { inOrder } from "./support/sequence.ts";
import { expectStill, landmarkOf, movementWhile, partLandmark } from "./support/movement.ts";
import {
  coveredSteps,
  focusInHead,
  foldsWith,
  keyThrough,
  LOW_WINDOW,
  wheelOver,
} from "./support/pane-head.ts";
import { issueAddress } from "../src/shared/paths.ts";

/** What a person had typed before the window changed. */
const HALF_A_COMMENT = "half a comment";

// The head of a pane folds to one line as its boxes scroll and opens again at their top (UX §6.5): it follows the
// scroll, never moves the scroll by itself, and nothing in the box moves but by the scroll.

test.use({ preparation: { run: prepareWeb } });

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

/** The boxes of the open issue on a computer. */
const SPLIT = "[data-part='issue-body'][data-variant~='split']";

/** The head of the pane. */
const HEAD = "[data-part='pane-head']";

test("ux_06 the keyboard brings a control of the sections out from under the folded head", async ({
  page,
}) => {
  await page.setViewportSize(LOW_WINDOW);
  await signIn(page, ada);
  await page.goto(issueAddress("WEB-1"));
  const sections = page.locator(`${SPLIT} [data-part='issue-main']`);
  await expect(page.locator(`${SPLIT} [data-part='issue-details']`)).toBeVisible();
  await page.waitForLoadState("networkidle");
  // From the last control of the sections, with the box at its end and the head folded, back up through the box.
  await sections.locator("button, a[href], input, textarea, select").last().focus();
  await sections.evaluate((node) => {
    node.scrollTop = node.scrollHeight;
  });
  const steps = await keyThrough(page, "Shift+Tab", 10, "[data-part='issue-main']", HEAD);
  expect(steps.filter((step) => step.inside).length).toBeGreaterThan(3);
  expect(coveredSteps(steps), "the controls the head covered").toEqual([]);
});

test("ux_06 a control of the folded head reached with the keyboard shows whole, and the head never scrolls", async ({
  page,
}) => {
  await page.setViewportSize(LOW_WINDOW);
  await signIn(page, ada);
  await page.goto(issueAddress("WEB-1"));
  const sections = page.locator(`${SPLIT} [data-part='issue-main']`);
  await expect(sections).toBeVisible();
  await page.waitForLoadState("networkidle");
  await sections.evaluate((node) => {
    node.scrollTop = node.scrollHeight;
  });
  // From the first line of the folded head to the title and the status, its second and third.
  await page.locator(HEAD).getByRole("button").first().focus();
  await inOrder(Array.from({ length: 3 }, () => () => page.keyboard.press("Tab")));
  const title = page.getByRole("heading", { level: 1 }).getByRole("button");
  await expect(title).toBeFocused();
  const shown = () => focusInHead(page);
  await expect.poll(shown).toEqual({ inside: true, scrolled: 0 });
  await page.keyboard.press("Tab");
  await expect.poll(shown).toEqual({ inside: true, scrolled: 0 });
});

test("ux_06 a comment half written stays when the window changes from a computer to a tablet and back", async ({
  page,
}) => {
  await page.setViewportSize({ width: 1280, height: 800 });
  await signIn(page, ada);
  await page.goto(issueAddress("WEB-1"));
  await expect(page.locator(`${SPLIT} [data-part='issue-details']`)).toBeVisible();
  const comment = page.getByRole("textbox", { name: t("comments.write"), exact: true });
  await comment.fill(HALF_A_COMMENT);
  await page.setViewportSize({ width: 1000, height: 800 });
  await expect(page.locator(SPLIT)).toHaveCount(0);
  await page.setViewportSize({ width: 1280, height: 800 });
  await expect(page.locator(SPLIT)).toBeVisible();
  await expect(comment).toHaveValue(HALF_A_COMMENT);
});
