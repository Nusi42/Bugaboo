import { t } from "@bugaboo/messages";

import { expect, test } from "./support/fixtures.ts";
import { screenValues } from "./support/every-screen.ts";
import { addressWith, sendPassword, signIn } from "./support/flows.ts";
import { sampleAdministrator, sampleWithCodeApp } from "./support/sample.ts";
import {
  everyGuestScreen,
  everyScreen,
  opened,
  type ScreenAddress,
} from "./support/screen-list.ts";
import { inOrder } from "./support/sequence.ts";
import { lowWindow, scrollingOf } from "./support/scrolling.ts";
import { paths } from "../src/shared/paths.ts";

// One way to scroll (UX §6.5, M8 WP-13): on every screen of the route table the page itself never
// scrolls, and no box that scrolls up and down stands inside another that does. A box counts by its look, whatever it
// holds now.

test.use({ sample: true });

/** Every screen, one after the other, takes longer than an ordinary test. */
const EVERY_SCREEN_MS = 900_000;

/** What breaks the rule on one screen. */
interface Found {
  readonly screen: string;
  readonly page: number;
  readonly nested: readonly string[];
}

/** What breaks the rule on the screen that is open, named `screen`. */
async function foundOn(page: Parameters<typeof opened>[0], screen: string): Promise<Found[]> {
  const seen = await scrollingOf(page);
  return seen.page > 0 || seen.nested.length > 0
    ? [{ screen, page: seen.page, nested: seen.nested }]
    : [];
}

/** Each screen opened and looked at. */
async function scrollingOnEach(
  page: Parameters<typeof opened>[0],
  screens: readonly ScreenAddress[],
): Promise<Found[]> {
  const found: Found[] = [];
  await inOrder(
    screens.map((screen) => async () => {
      await opened(page, screen.address);
      found.push(...(await foundOn(page, screen.path)));
    }),
  );
  return found;
}

test("ux_06_the_page_never_scrolls_and_no_scroll_box_holds_another", async ({ page, server }) => {
  test.setTimeout(EVERY_SCREEN_MS);
  const values = await screenValues(server);
  const guests = await scrollingOnEach(page, everyGuestScreen());
  await signIn(page, sampleAdministrator());
  const signedIn = await scrollingOnEach(page, everyScreen(values));
  expect([...guests, ...signedIn]).toEqual([]);
});

test("ux_06_a_low_window_scrolls_the_card_before_sign_in_and_never_the_page", async ({ page }) => {
  await lowWindow(page);
  const signInPage = await scrollingOnEach(page, [{ path: paths.signIn, address: paths.signIn }]);
  // The button of the card is reached by the one box of the page.
  const send = page.getByRole("button", { name: t("sign-in.submit"), exact: true });
  await send.scrollIntoViewIfNeeded();
  await expect(send).toBeInViewport({ ratio: 1 });
  // The second step of sign-in, as a person with a code app reaches it.
  await sendPassword(page, sampleWithCodeApp().person);
  await expect(page).toHaveURL(addressWith(paths.secondStep));
  await expect(page.getByRole("heading", { level: 1, name: t("second-step.title") })).toBeVisible();
  expect([...signInPage, ...(await foundOn(page, paths.secondStep))]).toEqual([]);
});
