import { signedInAdmin } from "./support/access.ts";
import { ada } from "./support/accounts.ts";
import { LABEL } from "./support/changes-in-place.ts";
import { wrongOfEveryChange } from "./support/changes-pass.ts";
import { expect, test } from "./support/fixtures.ts";
import { signIn } from "./support/flows.ts";
import { prepareWeb } from "./support/issues.ts";
import { createLabel } from "./support/labels.ts";

// Every change in place moves nothing else (UX §6.4, M8 WP-13): each row of the table of changes is
// made on its screen, on a computer and a phone, and only what it changes may grow, and only what
// stands under that may be pushed, by exactly that.

test.use({
  preparation: {
    run: async (server) => {
      await prepareWeb(server);
      const admin = await signedInAdmin(server);
      await createLabel(admin, LABEL);
      await admin.dispose();
    },
  },
});

/** Every row of the table takes longer than an ordinary test. */
const EVERY_CHANGE_MS = 600_000;

test("ux_01_every_change_in_place_moves_nothing_else", async ({ page }) => {
  test.setTimeout(EVERY_CHANGE_MS);
  await signIn(page, ada);
  expect(await wrongOfEveryChange(page)).toEqual([]);
});
