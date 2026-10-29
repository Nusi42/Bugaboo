import { dataOf } from "@bugaboo/api-client";

import { paths, savedSearchAddress } from "../src/shared/paths.ts";
import { signedInAdmin } from "./support/access.ts";
import { ada } from "./support/accounts.ts";
import { expect, test } from "./support/fixtures.ts";
import { signIn } from "./support/flows.ts";
import { prepareManyWeb } from "./support/issues.ts";
import { layoutOf } from "./support/layout.ts";
import { scrollersOf } from "./support/phone-list.ts";
import { type Instance } from "./support/server.ts";

// One scroller on a phone (UX §4.2; M5 exit): on the Search page and on a saved search, as on the
// list of a project, the list is the only thing that scrolls.

let savedId = "";

async function prepare(server: Instance): Promise<void> {
  await prepareManyWeb(server, 40);
  const admin = await signedInAdmin(server);
  savedId = dataOf(
    await admin.client.POST("/api/v1/saved-searches", {
      body: { name: "Everything open", search_text: "statusGroup is not Done" },
    }),
  ).id;
  await admin.dispose();
}

test.use({ preparation: { run: prepare } });

for (const [name, address] of [
  ["the Search page", () => paths.search],
  ["a saved search", () => savedSearchAddress(savedId)],
] as const) {
  test(`ux_4_2 on a phone the list of ${name} is the one scroller`, async ({ page }) => {
    test.skip(layoutOf(page) !== "phone", "only a phone has one scroller");
    await signIn(page, ada);
    await page.goto(address());
    await expect(
      page.locator("[data-part='list-row'][data-variant~='phone']").first(),
    ).toBeVisible();
    expect(await scrollersOf(page)).toEqual(["list-scroll"]);
  });
}
