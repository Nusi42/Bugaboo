import { paths } from "../src/shared/paths.ts";
import { ada, bob, createAdmin, createPerson } from "./support/accounts.ts";
import { emptyCardsOf } from "./support/cards.ts";
import { expect, test } from "./support/fixtures.ts";
import { signIn } from "./support/flows.ts";
import { inOrder } from "./support/sequence.ts";

// A card with nothing to show is not drawn (UX §6.1; M5 exit): the start page of somebody who may
// not look after the server has the greeting and the notifications, and no empty card above them.

test.use({
  preparation: {
    run: async (server) => {
      const admin = await createAdmin(server);
      await (await createPerson(server, admin, bob)).dispose();
      await admin.dispose();
    },
  },
});

const pages = [
  paths.home,
  paths.notifications,
  paths.appearance,
  paths.region,
  paths.shortcuts,
  paths.security,
  paths.sessions,
] as const;

test("ux_6_1 no page draws a card with nothing in it, for somebody who may not do much", async ({
  page,
}) => {
  await signIn(page, bob);
  await inOrder(
    pages.map((address) => async () => {
      await page.goto(address);
      await expect(page.getByRole("heading", { level: 1 })).toBeAttached();
      expect(await emptyCardsOf(page), address).toBe(0);
    }),
  );
});

test("ux_6_1 the same for the administrator", async ({ page }) => {
  await signIn(page, ada);
  await inOrder(
    [...pages, paths.projects, paths.themes].map((address) => async () => {
      await page.goto(address);
      await expect(page.getByRole("heading", { level: 1 })).toBeAttached();
      expect(await emptyCardsOf(page), address).toBe(0);
    }),
  );
});
