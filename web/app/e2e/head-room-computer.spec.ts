import type { Locator, Page } from "@playwright/test";

import { signedInAdmin } from "./support/access.ts";
import { ada } from "./support/accounts.ts";
import { expect, test } from "./support/fixtures.ts";
import { signIn } from "./support/flows.ts";
import { createIssue, prepareWeb } from "./support/issues.ts";
import { useLargerText } from "./support/list-ids.ts";
import { tickIssue } from "./support/list-look.ts";
import { openTimeline } from "./support/timeline.ts";
import { issueAddress, projectIssuesAddress } from "../src/shared/paths.ts";

// The room of the head of a pane (UX §6.5): the top bar and what stands above the boxes of a pane, the least height of
// the box and the bar of ticked issues over its foot fit a window of `size-low-window`, with the text at its own size
// and larger, and with a title of three lines. A window as low as that or lower lets the main column scroll.

/** A title long enough for three lines in the head of the issue. */
const LONG_TITLE =
  "The sign-in page forgets the address typed before the password was reset, so a person who " +
  "comes back from the mail has to type it again, and on a phone the field is hidden under the keys";

/** The issue with the long title, the fourth of the project. */
const LONG = "WEB-4";

test.use({
  preparation: {
    run: async (server) => {
      await prepareWeb(server);
      const admin = await signedInAdmin(server);
      await createIssue(admin, "WEB", LONG_TITLE);
      await admin.dispose();
    },
  },
});

/** A size of the theme in px, as the page has it. */
function tokenOf(page: Page, name: string): Promise<number> {
  return page.evaluate(
    (token) =>
      Number(
        getComputedStyle(document.documentElement)
          .getPropertyValue(`--${token}`)
          .trim()
          .replace("px", ""),
      ),
    name,
  );
}

/** How far down the window a box starts: the room of everything above it. */
function topOf(box: Locator): Promise<number> {
  return box.evaluate((node) => node.getBoundingClientRect().top);
}

/** Leaves the text at its own size. */
async function ownSize(): Promise<void> {
  // Nothing to change.
}

/**
 * Measures the head of the list, an issue with a title of three lines and the timeline, with the text made as large as
 * `scale` makes it, against the low window.
 */
async function headsFit(page: Page, scale: (page: Page) => Promise<void>): Promise<void> {
  await signIn(page, ada);
  await page.goto(projectIssuesAddress("WEB"));
  await scale(page);
  const rows = page.locator("[data-part~='list-scroll']");
  await expect(rows).toBeVisible();
  const list = await topOf(rows);
  await tickIssue(page, "WEB-1");
  const bar = page.locator("[data-part~='selected-bar'][data-state~='active']");
  await expect(bar).toBeVisible();
  const barLine = await bar.evaluate((node) => node.getBoundingClientRect().height);

  await page.goto(issueAddress(LONG));
  await scale(page);
  const sections = page.locator(
    "[data-part~='issue-body'][data-variant~='split'] [data-part~='issue-main']",
  );
  await expect(sections).toBeVisible();
  const title = page.getByRole("heading", { level: 1, name: LONG_TITLE });
  await expect(title).toBeVisible();
  const lines = await title.evaluate(
    (node) =>
      node.getBoundingClientRect().height /
      Number(getComputedStyle(node).lineHeight.replace("px", "")),
  );
  expect(Math.round(lines), "the lines of the long title").toBeGreaterThanOrEqual(3);
  const issue = await topOf(sections);

  await openTimeline(page);
  await scale(page);
  const timeline = await topOf(page.locator("[data-part~='timeline-scroll']"));

  const least = await tokenOf(page, "size-list-scroll-least");
  const low = await tokenOf(page, "size-low-window");
  for (const [pane, head] of Object.entries({ list, issue, timeline })) {
    expect
      .soft(head + least + barLine, `the head of ${pane}, the least box and the bar`)
      .toBeLessThanOrEqual(low);
  }
}

test("ux_06 the head of the list, an issue and the timeline fit the low window", async ({
  page,
}) => {
  await headsFit(page, ownSize);
});

test("ux_06 the head of the list, an issue and the timeline fit the low window with larger text", async ({
  page,
}) => {
  await headsFit(page, useLargerText);
});
