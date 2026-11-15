import { expect, type Locator } from "@playwright/test";

import { paths } from "../src/shared/paths.ts";
import { test } from "./support/fixtures.ts";
import { signIn } from "./support/flows.ts";
import { onThisLayout } from "./support/layout.ts";
import { partIn } from "./support/parts.ts";
import { sampleAdministrator } from "./support/sample.ts";
import { inOrder } from "./support/sequence.ts";

// M5 exit, the final pass (L3): a settings page whose sections are cards of their own (Sign-in and security, Labels and
// tags) has its heading and introduction on the page, above the sections, not in a card of their own that holds
// nothing else. On a computer and a tablet every page of the settings has them in the head of the page (UX §6.5).

test.use({ sample: true });

/** On a computer and a tablet: the introduction stands in the head of the page, above its cards (UX §6.5). */
async function inHeadOfPage(intro: Locator, address: string): Promise<void> {
  await expect(intro.locator("xpath=ancestor::*[@data-part='pane-head']"), address).toHaveCount(1);
  await expect(intro.locator("xpath=ancestor::*[@data-part='card']"), address).toHaveCount(0);
}

/** On a phone: in a card without a frame, the head of the page whose sections are cards. */
async function inCardWithoutFrame(intro: Locator, address: string): Promise<void> {
  const card = intro.locator("xpath=ancestor::*[@data-part='card'][1]");
  await expect(card, address).toHaveCSS("border-top-width", "0px");
  await expect(card, address).toHaveCSS("padding-top", "0px");
}

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
      await onThisLayout(page, {
        computer: () => inHeadOfPage(intro, address),
        tablet: () => inHeadOfPage(intro, address),
        phone: () => inCardWithoutFrame(intro, address),
      });
    }),
  );
});
