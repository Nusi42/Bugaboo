import { t } from "@bugaboo/messages";

import { ada, createAdmin } from "./support/accounts.ts";
import { expect, test } from "./support/fixtures.ts";
import { signIn } from "./support/flows.ts";
import { layoutOf } from "./support/layout.ts";
import { paths } from "../src/shared/paths.ts";

// M5 exit, final pass, package A: the places of search in the shell. On a phone the Search tab of the bar is current on
// the Search page (F-A2).

test.use({
  preparation: {
    run: async (server) => {
      await (await createAdmin(server)).dispose();
    },
  },
});

test("cfg_08 on a phone the Search tab of the bar is current on the Search page", async ({
  page,
}) => {
  test.skip(layoutOf(page) !== "phone", "only a phone has the bar at the bottom");
  await signIn(page, ada);
  await page.goto(paths.search);
  const bar = page.getByRole("navigation", { name: t("shell.menu"), exact: true });
  const search = bar.getByRole("button", { name: t("shell.search"), exact: true });
  await expect(search).toHaveAttribute("aria-current", "page");
  const home = bar.getByRole("link", { name: t("nav.home"), exact: true });
  const fills = await Promise.all(
    [search, home].map((tab) => tab.evaluate((node) => getComputedStyle(node).backgroundColor)),
  );
  expect(fills[0], "the current tab is drawn like a current tab").not.toBe(fills[1]);
});
