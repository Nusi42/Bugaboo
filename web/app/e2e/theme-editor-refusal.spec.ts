import { t } from "@bugaboo/messages";
import type { Locator, Page } from "@playwright/test";

import { paths } from "../src/shared/paths.ts";
import { ada } from "./support/accounts.ts";
import { expect, test } from "./support/fixtures.ts";
import { signIn } from "./support/flows.ts";
import {
  READABLE,
  UNREADABLE,
  createFromDusk,
  deleteThemesOf,
  hardColors,
  prepareThemes,
  readabilityLine,
  typeHint,
} from "./support/theme-editor.ts";
import { part } from "../../packages/ui/src/tokens.ts";

// A refused save of a theme for everyone, and a save that went through (THM-03, UX §5.6, §6.4), on
// a computer and a phone: the readability and the colors it names stay in sight, *Save theme* is
// off until they read well, the verdict counts pairs (glossary *Readability*), and a save says
// *Saved* where it happened.

test.use({ preparation: { run: prepareThemes } });

test.beforeEach(async ({ server }) => {
  await deleteThemesOf(server, ada);
});

/** *Save theme*. */
function saveButton(page: Page) {
  return page.getByRole("button", { name: t("theme-editor.save"), exact: true });
}

test("thm_03_a_refused_save_keeps_what_to_fix_in_sight_and_save_off_until_it_reads_well", async ({
  page,
}) => {
  await signIn(page, ada);
  await page.goto(paths.themes);
  await createFromDusk(page, "Harbor");
  await typeHint(page, UNREADABLE);
  // The verdict counts the pairs: as many as the backgrounds the hint is hard to read on.
  await hardColors(page)
    .getByRole("button", {
      name: t("theme-editor.show-named", { color: t("theme-color.fg-subtle") }),
    })
    .click();
  const pairs = await page.locator(`[data-part='${part.themeHardWith}'] li`).count();
  expect(pairs).toBeGreaterThan(1);
  await expect(readabilityLine(page)).toContainText(
    t("theme-editor.readability-hard", { count: pairs }),
  );
  await saveButton(page).click();
  // The refusal and the colors it names are in sight, and *Save theme* is off until they read well.
  await expect(readabilityLine(page)).toHaveAttribute("data-state", "refused");
  await expect(readabilityLine(page)).toBeInViewport();
  await expect(hardColors(page)).toBeInViewport();
  await expect(saveButton(page)).toBeDisabled();
  await typeHint(page, READABLE);
  await expect(saveButton(page)).toBeEnabled();
  await saveButton(page).click();
  // *Saved* where the save happened, beside *Save theme*.
  await expect(
    page.locator(`[data-part='${part.themeEditorBarEnd}']`).getByText(t("save.saved"), {
      exact: true,
    }),
  ).toBeVisible();
});

/** Scrolls the main column on and says whether the band of the verdict covers its top, and how wide the verdict is. */
function verdictWhenScrolled(verdict: Locator) {
  return verdict.evaluate((node, band) => {
    const main = node.closest("main");
    main?.scrollBy({ top: 200 });
    const top = main?.getBoundingClientRect().top ?? 0;
    const box = node.getBoundingClientRect();
    const above = document.elementFromPoint(box.left + box.width / 2, top + 2);
    return {
      covered: above?.closest(`[data-part='${band}']`)?.contains(node) === true,
      share: box.width / window.innerWidth,
    };
  }, part.themeEditorVerdict);
}

test("thm_03_on_a_phone_the_readability_is_a_whole_line_at_the_very_top", async ({ page }) => {
  test.skip(test.info().project.name !== "phone", "the line of a phone");
  await signIn(page, ada);
  await page.goto(paths.themes);
  await createFromDusk(page, "Harbor");
  await typeHint(page, READABLE);
  // Scrolled, the groups pass under the verdict: nothing of them shows above it, and it is about as
  // wide as the screen, so its words have the room of a line.
  const found = await verdictWhenScrolled(readabilityLine(page));
  expect(found.covered).toBe(true);
  expect(found.share).toBeGreaterThan(0.8);
});
