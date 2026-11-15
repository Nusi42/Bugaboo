import { t } from "@bugaboo/messages";

import { paths } from "../src/shared/paths.ts";
import { ada } from "./support/accounts.ts";
import { expect, test } from "./support/fixtures.ts";
import { signIn } from "./support/flows.ts";
import {
  READABLE,
  createFromDusk,
  deleteThemesOf,
  onlyThemeOf,
  prepareThemes,
  readabilityLine,
  typeHint,
} from "./support/theme-editor.ts";

// The order of emphasis of the text colors (THM-03, UX §5.6): a hint that stands out more than the
// quiet text is told in a gentle line under the verdict that names both colors, and the theme can
// still be saved, also as a theme for everyone.

test.use({ preparation: { run: prepareThemes } });

test.beforeEach(async ({ server }) => {
  await deleteThemesOf(server, ada);
});

/** The gentle line that says the order of the hint and the quiet text is upside down. */
const upsideDown = () =>
  t("theme-editor.order-upside-down", {
    quieter: t("theme-color.fg-subtle"),
    louder: t("theme-color.fg-muted"),
  });

test("thm_03_a_hint_that_stands_out_more_than_the_quiet_text_is_told_and_still_saved", async ({
  page,
  server,
}) => {
  await signIn(page, ada);
  await page.goto(paths.themes);
  await createFromDusk(page, "Harbor");
  // Dusk's own hint keeps its place: no line.
  await expect(page.getByText(upsideDown(), { exact: true })).toHaveCount(0);
  // A hint brighter than the quiet text on the dark card is readable but upside down.
  await typeHint(page, READABLE);
  await expect(readabilityLine(page)).toHaveText(t("theme-editor.readability-fine"));
  await expect(page.getByText(upsideDown(), { exact: true })).toBeVisible();
  await page.getByRole("button", { name: t("theme-editor.save"), exact: true }).click();
  await expect.poll(async () => (await onlyThemeOf(server, ada)).theme.saved).toBe(true);
});
