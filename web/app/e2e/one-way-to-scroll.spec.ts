import { expect, test } from "./support/fixtures.ts";
import { screenValues } from "./support/every-screen.ts";
import { signIn } from "./support/flows.ts";
import { sampleAdministrator } from "./support/sample.ts";
import {
  everyGuestScreen,
  everyScreen,
  opened,
  type ScreenAddress,
} from "./support/screen-list.ts";
import { inOrder } from "./support/sequence.ts";
import { scrollingOf } from "./support/scrolling.ts";

// One way to scroll (UX §6.5, M8 WP-13): on every screen of the route table the page itself never
// scrolls, and no box that scrolls up and down stands inside another that does.

test.use({ sample: true });

/** Every screen, one after the other, takes longer than an ordinary test. */
const EVERY_SCREEN_MS = 900_000;

/** What breaks the rule on one screen. */
interface Found {
  readonly screen: string;
  readonly page: number;
  readonly nested: readonly string[];
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
      const seen = await scrollingOf(page);
      if (seen.page > 0 || seen.nested.length > 0) {
        found.push({ screen: screen.path, page: seen.page, nested: seen.nested });
      }
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
