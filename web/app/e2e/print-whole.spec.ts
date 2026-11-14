import type { Page } from "@playwright/test";

import { ada } from "./support/accounts.ts";
import { expect, test } from "./support/fixtures.ts";
import { signIn } from "./support/flows.ts";
import { prepareWeb } from "./support/issues.ts";
import { inOrder } from "./support/sequence.ts";
import { openTimeline } from "./support/timeline.ts";
import { panelAddress } from "./support/addresses.ts";
import { issueAddress, projectIssuesAddress } from "../src/shared/paths.ts";

// On paper nothing scrolls (UX §6.5): the boxes that scroll on the screen are as long as what they hold, so nothing is
// cut at the height of the window.

test.use({ preparation: { run: prepareWeb } });

/** The boxes inside the app that clip what they hold, by their parts, as the page is printed. */
function cutOnPaper(page: Page): Promise<string[]> {
  return page.evaluate(() => {
    const cut: string[] = [];
    for (const box of document.querySelectorAll("[data-part~='app-layout'] *")) {
      // What a box shows past its edge is printed; only a box that clips cuts. A box of one pixel is hidden on purpose
      // (the words for a screen reader, the probe of a width): there is nothing to cut.
      const clips = getComputedStyle(box).overflowY !== "visible" && box.clientHeight > 1;
      if (clips && box.scrollHeight > box.clientHeight + 1) {
        cut.push(box instanceof HTMLElement ? (box.dataset["part"] ?? box.tagName) : box.tagName);
      }
    }
    return cut;
  });
}

test("ux_06 on paper the list, an issue, the timeline and the side panel are whole", async ({
  page,
}) => {
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
      async () => {
        await page.goto(panelAddress("WEB", "WEB-1"));
        await expect(page.locator("[data-part~='side-panel.body']")).toBeVisible();
      },
    ].map((open, at) => async () => {
      await open();
      cut[String(at)] = await cutOnPaper(page);
    }),
  );
  expect(cut).toEqual({ "0": [], "1": [], "2": [], "3": [] });
});
