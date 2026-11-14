import { ada } from "./support/accounts.ts";
import { expect, test } from "./support/fixtures.ts";
import { signIn } from "./support/flows.ts";
import { prepareWeb } from "./support/issues.ts";
import { boxOf } from "./support/windows.ts";
import { paths } from "../src/shared/paths.ts";

// The main column scrolls, not the window (UX §6.5): a new page starts at its top.

test.use({ preparation: { run: prepareWeb } });

test("ux_06 a new page starts at the top of the main column", async ({ page }) => {
  await signIn(page, ada);
  await page.goto(paths.appearance);
  await expect(page.getByRole("heading", { level: 1 })).toBeVisible();
  const main = page.locator("[data-part='app-layout.main']");
  // The wheel over the main column, as a person scrolls.
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
