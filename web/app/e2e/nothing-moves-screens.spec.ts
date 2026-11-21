import { expect, test } from "./support/fixtures.ts";
import { screenValues } from "./support/every-screen.ts";
import { signIn } from "./support/flows.ts";
import { movementWhile, type Movement } from "./support/movement.ts";
import { everyPartInView, readEverythingAgain } from "./support/read-again.ts";
import { sampleAdministrator } from "./support/sample.ts";
import {
  EVERY_SCREEN_MS,
  everyGuestScreen,
  everyScreen,
  opened,
  type ScreenAddress,
} from "./support/screen-list.ts";
import { inOrder } from "./support/sequence.ts";

// A read in the background moves and flashes nothing (UX §6.4, M8 WP-13): on every screen of the
// route table, every answer is read again, and no part in the window moves, goes, is drawn afresh
// or makes way for grey shapes, and nothing scrolls.

test.use({ sample: true });

/** What a read in the background did on one screen, where it did anything. */
interface Found {
  readonly screen: string;
  readonly moved: readonly string[];
  readonly vanished: readonly string[];
  readonly greyShapes: boolean;
  readonly shifts: Movement["shifts"];
  readonly scrolled: Movement["scrolled"];
}

/** Each screen opened, read again in the background and measured. */
async function readAgainOnEach(
  page: Parameters<typeof opened>[0],
  screens: readonly ScreenAddress[],
): Promise<Found[]> {
  const found: Found[] = [];
  await inOrder(
    screens.map((screen) => async () => {
      await opened(page, screen.address);
      const parts = await everyPartInView(page);
      const movement = await movementWhile(page, parts, () => readEverythingAgain(page));
      const moved = movement.moved.map(
        (each) => `${each.name} ${JSON.stringify(each.from)} -> ${JSON.stringify(each.to)}`,
      );
      const shifts = movement.shifts.filter((shift) => shift.value > 0);
      if (
        moved.length > 0 ||
        shifts.length > 0 ||
        movement.vanished.length > 0 ||
        movement.greyShapes ||
        movement.scrolled.length > 0
      ) {
        found.push({
          screen: screen.path,
          moved,
          vanished: movement.vanished,
          shifts,
          greyShapes: movement.greyShapes,
          scrolled: movement.scrolled,
        });
      }
    }),
  );
  return found;
}

test("ux_01_a_read_in_the_background_moves_and_flashes_nothing_on_any_screen", async ({
  page,
  server,
}) => {
  test.setTimeout(EVERY_SCREEN_MS);
  const values = await screenValues(server);
  const guests = await readAgainOnEach(page, everyGuestScreen());
  await signIn(page, sampleAdministrator());
  const signedIn = await readAgainOnEach(page, everyScreen(values));
  expect([...guests, ...signedIn]).toEqual([]);
});
