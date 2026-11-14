import type { Page } from "@playwright/test";

import { ada } from "./support/accounts.ts";
import { expect, test } from "./support/fixtures.ts";
import { signIn } from "./support/flows.ts";
import { prepareWeb } from "./support/issues.ts";
import { inOrder } from "./support/sequence.ts";
import { openTimeline } from "./support/timeline.ts";
import { issueAddress, projectIssuesAddress } from "../src/shared/paths.ts";

// On paper nothing scrolls (UX §6.5): the boxes that scroll on the screen are as long as what they hold, so nothing is
// cut at the height of the window.

test.use({ preparation: { run: prepareWeb } });

/** The boxes that scroll on a screen. */
const BOXES = [
  "app-layout.main",
  "list-scroll",
  "timeline-scroll",
  "timeline-tray",
  "issue-main",
  "issue-details",
  "side-panel.body",
  "focus-view.list",
];

/** The boxes that clip what they hold, by their parts, as the page is printed. */
function cutOnPaper(page: Page): Promise<string[]> {
  return page.evaluate((parts: readonly string[]) => {
    const cut: string[] = [];
    for (const name of parts) {
      for (const box of document.querySelectorAll(`[data-part~='${name}']`)) {
        // What a box shows past its edge is printed; only a box that clips cuts.
        const clips = getComputedStyle(box).overflowY !== "visible";
        if (clips && box.scrollHeight > box.clientHeight + 1) {
          cut.push(name);
        }
      }
    }
    return cut;
  }, BOXES);
}

test("ux_06 on paper the list, an issue and the timeline are whole", async ({ page }) => {
  await page.setViewportSize({ width: 1280, height: 400 });
  await signIn(page, ada);
  await page.emulateMedia({ media: "print" });
  const cut: Record<string, string[]> = {};
  await inOrder(
    [
      async () => {
        await page.goto(projectIssuesAddress("WEB"));
        await expect(page.locator("[data-part~='list-scroll']")).toBeVisible();
      },
      async () => {
        await page.goto(issueAddress("WEB-1"));
        await expect(page.locator("[data-part~='issue-main']")).toBeVisible();
      },
      () => openTimeline(page),
    ].map((open, at) => async () => {
      await open();
      cut[String(at)] = await cutOnPaper(page);
    }),
  );
  expect(cut).toEqual({ "0": [], "1": [], "2": [] });
});
