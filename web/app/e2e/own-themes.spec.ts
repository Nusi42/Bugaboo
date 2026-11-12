import { ada, createAdmin, forgetChoices } from "./support/accounts.ts";
import { expectLook } from "./support/checks.ts";
import { expect, test } from "./support/fixtures.ts";
import { signIn } from "./support/flows.ts";
import {
  chooseOwnDarkTheme,
  frameVariable,
  refusedByThePolicy,
  watchThePolicy,
} from "./support/own-themes.ts";
import { pageAttributes } from "../../packages/ui/src/tokens.ts";

/** The color of the own theme: an accent that reads well on Dusk. */
const ACCENT = "#b4a8ff";

test.use({
  preparation: {
    run: async (server) => {
      await (await createAdmin(server)).dispose();
    },
  },
});

test.beforeEach(async ({ server }) => {
  await forgetChoices(server);
});

test("thm_04 the page uses an own theme without an inline style", async ({ page, server }) => {
  await watchThePolicy(page);
  const theme = await chooseOwnDarkTheme(server, ada, { accent: { value: ACCENT } });

  await signIn(page, ada);
  await expectLook(page, "dusk");
  await expect(page.locator("html")).toHaveAttribute(pageAttributes.ownTheme, theme.id);
  await expect(page.locator(`link[rel='stylesheet'][href='${theme.sheet}']`)).toHaveCount(1);
  await expect.poll(() => frameVariable(page, "--accent")).toBe(ACCENT);
  // Nothing on the page carries a style of its own, and the policy refused nothing.
  await expect(page.locator("[style]")).toHaveCount(0);
  expect(await refusedByThePolicy(page)).toEqual([]);
});
