import { t } from "@bugaboo/messages";

import { ada } from "./support/accounts.ts";
import { expect, test } from "./support/fixtures.ts";
import { openFromMenu, signIn } from "./support/flows.ts";
import { prepareWeb } from "./support/issues.ts";
import { onThisLayout } from "./support/layout.ts";
import { settingsBoxOf } from "./support/pane-head.ts";
import { boxOf } from "./support/geometry.ts";
import { paths } from "../src/shared/paths.ts";

// The box of a page scrolls, not the window (UX §6.5): the topic of the settings under its folding head on a computer,
// the main column elsewhere. A new page starts at its top, and Back brings it to where it was.

test.use({ preparation: { run: prepareWeb } });

test("ux_06 a new page starts at the top of its box", async ({ page }) => {
  await signIn(page, ada);
  await page.goto(paths.appearance);
  await expect(page.getByRole("heading", { level: 1 })).toBeVisible();
  const main = page.locator(settingsBoxOf(page));
  // The wheel over the box, as a person scrolls.
  const box = await boxOf(main);
  await page.mouse.move(box.x + 200, box.y + 200);
  await page.mouse.wheel(0, 300);
  await expect.poll(() => main.evaluate((column) => column.scrollTop)).toBeGreaterThan(0);
  await page.evaluate((to) => {
    history.pushState(null, "", to);
    dispatchEvent(new PopStateEvent("popstate"));
  }, paths.region);
  await expect(page).toHaveURL(new RegExp(`${paths.region}$`, "u"));
  await expect.poll(() => main.evaluate((column) => column.scrollTop)).toBe(0);
});

test("ux_06 Back brings the box of the page to where it was", async ({ page }) => {
  await signIn(page, ada);
  await page.goto(paths.appearance);
  await expect(page.getByRole("heading", { level: 1 })).toBeVisible();
  const main = page.locator(settingsBoxOf(page));
  const box = await boxOf(main);
  await page.mouse.move(box.x + box.width / 2, box.y + box.height / 2);
  await page.mouse.wheel(0, 300);
  await expect.poll(() => main.evaluate((column) => column.scrollTop)).toBeGreaterThan(0);
  const was = await main.evaluate((column) => column.scrollTop);
  // Another page in one step: the topic in the frame of the settings, which a phone shows through
  // the menu only.
  const topic = t("nav.access-keys");
  const frame = page.getByRole("navigation", { name: t("nav.my-settings"), exact: true });
  await onThisLayout(page, {
    computer: () => frame.getByRole("link", { name: topic, exact: true }).click(),
    tablet: () => frame.getByRole("link", { name: topic, exact: true }).click(),
    phone: () => openFromMenu(page, topic),
  });
  await expect(page).toHaveURL(new RegExp(`${paths.accessKeys}$`, "u"));
  await expect.poll(() => main.evaluate((column) => column.scrollTop)).toBe(0);
  // Back through the pages the way went through (on a phone, the list of the settings first).
  await onThisLayout(page, {
    computer: async () => {
      await page.goBack();
    },
    tablet: async () => {
      await page.goBack();
    },
    phone: async () => {
      await page.goBack();
      await page.goBack();
    },
  });
  await expect(page).toHaveURL(new RegExp(`${paths.appearance}$`, "u"));
  await expect.poll(() => main.evaluate((column) => column.scrollTop)).toBe(was);
});
