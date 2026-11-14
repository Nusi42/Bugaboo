import type { Locator } from "@playwright/test";

import { ada } from "./support/accounts.ts";
import { expect, test } from "./support/fixtures.ts";
import { signIn } from "./support/flows.ts";
import { prepareWeb } from "./support/issues.ts";
import { expectStill, landmarkOf, movementWhile, partLandmark } from "./support/movement.ts";
import { foldsWith, LOW_WINDOW, wheelOver } from "./support/pane-head.ts";
import { issueAddress } from "../src/shared/paths.ts";

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
