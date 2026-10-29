import { t } from "@bugaboo/messages";
import { type Locator, type Page } from "@playwright/test";

import { expect, test } from "./support/fixtures.ts";

import { layoutOf } from "./support/layout.ts";
import { itemArea, signIn } from "./support/flows.ts";
import { sampleAdministrator } from "./support/sample.ts";
import { resolved } from "./support/styles.ts";
import { paths } from "../src/shared/paths.ts";

// M5 exit, package T (second picture review): every button is as high as a control (36 with a mouse, 44 for
// a finger), the buttons of a row of notifications and of a group included, and a button whose words are
// long grows with them and never wraps inside a fixed height.

test.use({ sample: true });

/** How high the control of the theme is, as the browser works it out. */
async function controlHeight(page: Page): Promise<number> {
  return Number((await resolved(page, "height", "var(--size-control)")).replace("px", ""));
}

async function heightsOf(buttons: Locator): Promise<readonly number[]> {
  const count = await buttons.count();
  return Promise.all(
    Array.from(
      { length: count },
      async (_, at) => (await buttons.nth(at).boundingBox())?.height ?? 0,
    ),
  );
}

test("ux_7_4 the buttons of a notification are as high as a control", async ({ page }) => {
  await signIn(page, sampleAdministrator());
  await page.goto(paths.home);
  const actions = page.locator("[data-part='notification.actions']").first();
  await expect(actions).toBeVisible();
  const least = await controlHeight(page);
  const heights = await heightsOf(actions.getByRole("button"));
  expect(heights.length).toBeGreaterThan(0);
  heights.forEach((height) => {
    expect(height).toBeGreaterThanOrEqual(least - 0.5);
  });
  expect(new Set(heights.map((height) => Math.round(height))).size).toBe(1);
});

test("ux_7_4 Put in and Take out are as high as a control", async ({ page }) => {
  test.skip(layoutOf(page) === "phone", "the panel of a group is for a computer");
  await signIn(page, sampleAdministrator());
  await page.goto(paths.groups);
  await page.getByRole("button", { name: "Backend", exact: true }).click();
  const least = await controlHeight(page);
  const panel = itemArea(page, "Backend");
  await expect(panel).toBeVisible();
  const takeOut = panel.getByRole("button", { name: t("groups.remove"), exact: false });
  const putIn = panel.getByRole("button", { name: t("groups.add"), exact: false });
  // The members and the people come in after the panel opens: wait for both lists.
  await expect(takeOut.first()).toBeVisible();
  await expect(putIn.first()).toBeVisible();
  const [takeOutHeights, putInHeights] = [await heightsOf(takeOut), await heightsOf(putIn)];
  const all = [...takeOutHeights, ...putInHeights];
  expect(takeOutHeights.length).toBeGreaterThan(0);
  expect(putInHeights.length).toBeGreaterThan(0);
  all.forEach((height) => {
    expect(height).toBeGreaterThanOrEqual(least - 0.5);
  });
  // Put in and Take out are one height, to the pixel.
  expect(new Set(all.map((height) => Math.round(height))).size).toBe(1);
});

test("ux_7_4 a button with long words grows and never has a fixed height", async ({ page }) => {
  await signIn(page, sampleAdministrator());
  await page.goto(paths.layouts);
  const button = page.getByRole("button", { name: t("layouts.customize-organization") });
  await expect(button).toBeVisible();
  const least = await controlHeight(page);
  expect((await heightsOf(button))[0]).toBeGreaterThanOrEqual(least - 0.5);
  await expect(button).toHaveCSS(
    "height",
    await button.evaluate((n) => `${n.getBoundingClientRect().height}px`),
  );
  const fixed = await button.evaluate((node) => {
    const style = getComputedStyle(node);
    return { min: style.minHeight, max: style.maxHeight };
  });
  expect(fixed.max).toBe("none");
  expect(Number(fixed.min.replace("px", ""))).toBeGreaterThanOrEqual(least - 0.5);
});

test("ux_7_3 the intro of Layouts is body text like the intro of Fields, not a hint", async ({
  page,
}) => {
  await signIn(page, sampleAdministrator());
  await page.goto(paths.layouts);
  const intro = page.getByText(t("layouts.intro"), { exact: true });
  await expect(intro).toHaveCSS("font-size", await resolved(page, "font-size", "var(--fs-md)"));
  await expect(intro).toHaveCSS("color", await resolved(page, "color", "var(--fg)"));
});
