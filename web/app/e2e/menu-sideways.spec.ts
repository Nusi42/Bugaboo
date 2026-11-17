import { ada } from "./support/accounts.ts";
import { test } from "./support/fixtures.ts";
import { signIn } from "./support/flows.ts";
import { prepareWeb } from "./support/issues.ts";
import { expectMenuNeverScrollsSideways, openWithMenu } from "./support/menu-sideways.ts";

// The menu never scrolls sideways (UX §6.5), wide or narrowed, in a window with room for all of it and in a low one.
// A spec for a computer.

test.use({ preparation: { run: prepareWeb } });

/** A window with room for the whole menu, and one as low as a laptop with the browser's bars and a docked tool. */
const TALL = { width: 1280, height: 900 } as const;
const LOW = { width: 1280, height: 400 } as const;

test.beforeEach(async ({ page }) => {
  await signIn(page, ada);
});

test("ux_06 the wide menu never scrolls sideways, in a tall and in a low window", async ({
  page,
}) => {
  await page.setViewportSize(TALL);
  await openWithMenu(page, "wide");
  await expectMenuNeverScrollsSideways(page);
  await page.setViewportSize(LOW);
  await expectMenuNeverScrollsSideways(page);
});

test("ux_06 the menu narrowed by the person never scrolls sideways, in a tall and in a low window", async ({
  page,
}) => {
  await page.setViewportSize(TALL);
  await openWithMenu(page, "narrowed");
  await expectMenuNeverScrollsSideways(page);
  await page.setViewportSize(LOW);
  await expectMenuNeverScrollsSideways(page);
});

test("ux_06 the menu narrowed by an open issue never scrolls sideways, in a tall and in a low window", async ({
  page,
}) => {
  await page.setViewportSize(TALL);
  await openWithMenu(page, "squeezed");
  await expectMenuNeverScrollsSideways(page);
  await page.setViewportSize(LOW);
  await expectMenuNeverScrollsSideways(page);
});
