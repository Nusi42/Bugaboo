import { ada } from "./support/accounts.ts";
import { expect, test } from "./support/fixtures.ts";
import { signIn } from "./support/flows.ts";
import { prepareWeb, pressNewIssue, waitForNewIssueForm } from "./support/issues.ts";
import { partIn } from "./support/parts.ts";
import {
  dragWindowEdge,
  expectWindowSeen,
  holdNewIssueLayout,
  pageSize,
  TALLER_THAN_THE_FORM_PX,
  windowSize,
} from "./support/windows.ts";
import { projectIssuesAddress } from "../src/shared/paths.ts";

// A window that had to show grey shapes stands at the top of the page and keeps that top (UX §6.1, §6.3), so it grows
// only downwards: its edges must still follow the pointer, and it has no top edge to drag.

test.use({ preparation: { run: prepareWeb } });

/** How far the bottom edge is dragged. */
const DRAG_PX = 80;

/** The places to drag at the top of a window. */
const TOP_HANDLES = '[data-variant="n"], [data-variant="ne"], [data-variant="nw"]';

test("ux_6_3 the bottom edge of a window at the top of the page follows the pointer, and the top has no handle", async ({
  page,
}) => {
  await page.setViewportSize({ ...pageSize(page), height: TALLER_THAN_THE_FORM_PX });
  await signIn(page, ada);
  await page.goto(projectIssuesAddress("WEB"));
  const release = await holdNewIssueLayout(page);
  await pressNewIssue(page);
  await expectWindowSeen(page);
  // While it waits it cannot be resized, so a size as tall as the page is never kept.
  await expect(partIn(page, "window.handle")).toHaveCount(0);
  release();
  await waitForNewIssueForm(page);
  await expect(partIn(page, "window.handle").and(page.locator(TOP_HANDLES))).toHaveCount(0);
  await expect(partIn(page, "window.handle", "s")).toHaveCount(1);
  const before = await windowSize(page);
  await dragWindowEdge(page, "s", { x: 0, y: DRAG_PX });
  expect((await windowSize(page)).height - before.height).toBe(DRAG_PX);
});
