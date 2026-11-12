import { ada } from "./support/accounts.ts";
import { expect, test } from "./support/fixtures.ts";
import { signIn } from "./support/flows.ts";
import { chooseSpacing, listFrame, prepareLook } from "./support/list-look.ts";
import { partIn } from "./support/parts.ts";
import { inOrder } from "./support/sequence.ts";
import { touchTarget } from "./support/touch-targets.ts";
import { boxOf } from "./support/windows.ts";
import { projectIssuesAddress } from "../src/shared/paths.ts";
import { viewports } from "../../tooling/viewports.ts";

// Touch everywhere (UX-02, UX §7.4): *Compact* changes spacing, never a target of a finger (M5-F158): every box to
// tick of the list keeps 44 px, on the cards of a phone and in the grid of a tablet.

test.use({ preparation: { run: prepareLook } });

test("ux_02_compact_tick_boxes_keep_44_px_on_touch", async ({ page, server }) => {
  await chooseSpacing(server, "compact");
  await signIn(page, ada);
  const least = await touchTarget(page);
  await inOrder(
    [viewports.phone, viewports.tablet].map((size) => async () => {
      await page.setViewportSize(size);
      await page.goto(projectIssuesAddress("WEB"));
      await expect(listFrame(page)).toHaveAttribute("data-variant", /compact/u);
      await expect(partIn(listFrame(page), "checkbox").first()).toBeVisible();
      const boxes = await Promise.all(
        (await partIn(listFrame(page), "checkbox").all()).map((box) => boxOf(box)),
      );
      expect(boxes.length, `${String(size.width)} wide`).toBeGreaterThan(0);
      for (const box of boxes) {
        expect(box.width).toBeGreaterThanOrEqual(least);
        expect(box.height).toBeGreaterThanOrEqual(least);
      }
    }),
  );
  await chooseSpacing(server, "comfortable");
});
