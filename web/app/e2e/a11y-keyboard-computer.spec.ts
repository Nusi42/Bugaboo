import { focusWalk } from "./support/a11y-focus.ts";
import { measureEveryScreen } from "./support/a11y-measures.ts";
import { expect, test } from "./support/fixtures.ts";
import { EVERY_SCREEN_MS } from "./support/screen-list.ts";
import { crowdedTargets, inWords } from "./support/touch-targets.ts";

// The keyboard and the mouse on every screen of the route table and every screen before sign-in (COMP-15, TEST-A11Y),
// on the sample as its administrator sees it, on a computer. With touch every target is 44 px (touch-targets-phone).

test.use({ sample: true });

/** The least size of a target that WCAG 2.5.8 asks for with a mouse. */
const POINTER_PX = 24;

test("comp_15 every target is at least 24 px", async ({ page, server }) => {
  test.setTimeout(EVERY_SCREEN_MS);
  // A target under 24 px is met when it has the room of 24 px around it (WCAG 2.5.8, spacing).
  const found = await measureEveryScreen(page, server, async (opened) =>
    inWords(await crowdedTargets(opened, POINTER_PX)),
  );
  expect(found).toEqual([]);
});

test("comp_15 a Tab walk of every screen keeps the order of the code, shows the focus and has no trap", async ({
  page,
  server,
}) => {
  test.setTimeout(EVERY_SCREEN_MS);
  expect(await measureEveryScreen(page, server, focusWalk)).toEqual([]);
});
