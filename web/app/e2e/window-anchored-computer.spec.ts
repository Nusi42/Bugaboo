import { ada } from "./support/accounts.ts";
import { expect, test } from "./support/fixtures.ts";
import { signIn } from "./support/flows.ts";
import { newIssueDialog, prepareWeb, pressNewIssue } from "./support/issues.ts";
import {
  dragWindowEdge,
  expectWindowSeen,
  holdNewIssueLayout,
  pageSize,
  windowSize,
} from "./support/windows.ts";
import { projectIssuesAddress } from "../src/shared/paths.ts";

// A window that had to show grey shapes stands at the top of the page and keeps that top (UX §6.1, §6.3), so it grows
// only downwards: its edges must still follow the pointer.

test.use({ preparation: { run: prepareWeb } });

/** A page taller than the filled new-issue form (about 1030 high), so the window can still grow. */
const TALLER_THAN_THE_FORM_PX = 1300;

/** How far the bottom edge is dragged. */
const DRAG_PX = 80;

test("ux_6_3 the bottom edge of a window at the top of the page follows the pointer", async ({
  page,
}) => {
  await page.setViewportSize({ ...pageSize(page), height: TALLER_THAN_THE_FORM_PX });
  await signIn(page, ada);
  await page.goto(projectIssuesAddress("WEB"));
  const release = await holdNewIssueLayout(page);
  await pressNewIssue(page);
  await expectWindowSeen(page);
  release();
  await expect(newIssueDialog(page).locator('[aria-busy="false"]')).toHaveCount(1);
  const before = await windowSize(page);
  await dragWindowEdge(page, "s", { x: 0, y: DRAG_PX });
  expect((await windowSize(page)).height - before.height).toBe(DRAG_PX);
});
