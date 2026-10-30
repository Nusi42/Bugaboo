import { expect } from "@playwright/test";

import { paths } from "../src/shared/paths.ts";
import { test } from "./support/fixtures.ts";
import { signIn } from "./support/flows.ts";
import { partIn } from "./support/parts.ts";
import { sampleAdministrator } from "./support/sample.ts";
import { inOrder } from "./support/sequence.ts";

// M5 exit, the final pass (L3): a settings page whose sections are cards of their own (Sign-in and security, Labels and
// tags) has its heading and introduction on the page, above the sections, not in a card of their own that holds
// nothing else.

test.use({ sample: true });

/** The pages whose sections are cards, each with the heading and the introduction of the page above them. */
const pages = [paths.security, paths.labels] as const;

test("ux_5 the heading and introduction of a page of sections stand on the page, not in a card of their own", async ({
  page,
}) => {
  await signIn(page, sampleAdministrator());
  await inOrder(
    pages.map((address) => async () => {
      await page.goto(address);
      const intro = partIn(page, "card.intro").first();
      await expect(intro, address).toBeVisible();
      const head = partIn(page, "card")
        .filter({ has: partIn(page, "card.intro") })
        .first();
      await expect(head, address).toHaveCSS("border-top-width", "0px");
      await expect(head, address).toHaveCSS("padding-top", "0px");
    }),
  );
});
