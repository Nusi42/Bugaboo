import { paths } from "../src/shared/paths.ts";
import { ada, createAdmin } from "./support/accounts.ts";
import { expect, test } from "./support/fixtures.ts";
import { signIn } from "./support/flows.ts";
import { layoutOf } from "./support/layout.ts";
import { inOrder } from "./support/sequence.ts";
import { partIn } from "./support/parts.ts";
import { boxOf } from "./support/windows.ts";

// One title for each screen of a phone (UX §7.3; M5 exit R3): the top bar is the title, and the
// heading of the one card that is the whole page is for screen readers only. Checked over the
// pages, not one by one.

test.use({
  preparation: {
    run: async (server) => {
      await (await createAdmin(server)).dispose();
    },
  },
});

const pages = [
  paths.projects,
  paths.appearance,
  paths.region,
  paths.startPage,
  paths.shortcuts,
  paths.security,
  paths.sessions,
  paths.themes,
  paths.notifications,
] as const;

test("ux_7_3 on a phone the heading of a page is not drawn a second time under the top bar (R3)", async ({
  page,
}) => {
  test.skip(layoutOf(page) !== "phone", "only a phone has the title in its top bar");
  await signIn(page, ada);
  await inOrder(
    pages.map((address) => async () => {
      await page.goto(address);
      const heading = page.getByRole("heading", { level: 1 }).first();
      await expect(heading).toBeAttached();
      // Not drawn: a heading for screen readers only is a point.
      expect((await boxOf(heading)).width, address).toBeLessThanOrEqual(1);
      await expect(partIn(page, "top-bar.title")).toBeVisible();
    }),
  );
});
