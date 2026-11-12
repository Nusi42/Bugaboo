import { expect, test } from "./support/fixtures.ts";
import { smallTargetsOfEveryScreen } from "./support/touch-targets.ts";
import { viewports } from "../../tooling/viewports.ts";

// Touch everywhere (UX-02, UX §7.4 and §8): with a finger every control is at least 44 px high and wide, on every
// screen of the route table, on a phone and on a tablet. A link in a sentence, a link in a row that is pressed as a
// whole and what only a screen reader reads are no targets of their own (WCAG 2.5.8).

test.use({ sample: true });

/** Every screen, one after the other, takes longer than an ordinary test. */
const EVERY_SCREEN_MS = 600_000;

test("ux_02_every_control_has_a_touch_target_of_at_least_44_px_on_a_phone", async ({
  page,
  server,
}) => {
  test.setTimeout(EVERY_SCREEN_MS);
  const found = await smallTargetsOfEveryScreen(page, server);
  expect(found.filter((each) => each.targets.length > 0)).toEqual([]);
});

test.describe("on a tablet", () => {
  test.use({ viewport: viewports.tablet, hasTouch: true });

  test("ux_02 every control has a touch target of at least 44 px on a tablet with a finger", async ({
    page,
    server,
  }) => {
    test.setTimeout(EVERY_SCREEN_MS);
    const found = await smallTargetsOfEveryScreen(page, server);
    expect(found.filter((each) => each.targets.length > 0)).toEqual([]);
  });
});
