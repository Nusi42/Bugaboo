import { dataOf } from "@bugaboo/api-client";
import { t } from "@bugaboo/messages";
import { expect, type Page } from "@playwright/test";

import { pinsShown as SHOWN, pinsShownLow as LOW_SHOWN } from "../src/shared/pin-folds.ts";
import { projectAddress } from "../src/shared/paths.ts";
import { ada } from "./support/accounts.ts";
import { test } from "./support/fixtures.ts";
import { apiOf, signIn } from "./support/flows.ts";
import { prepareWeb } from "./support/issues.ts";
import { layoutOf } from "./support/layout.ts";

// Pinned navigation (CFG-05, M10 WP-09): the section *Pinned* of the sidebar, its place first in *More* on a phone, the
// fold after eight pins in a window 600 px high (M5-F150) and the keyboard's way to the order of the pins.

test.use({ preparation: { run: prepareWeb } });

/** The project the server of the tests has, as its page and its pins call it. */
const project = { shortName: "WEB", name: "Website" } as const;

/** The section of the sidebar. */
function pinned(page: Page) {
  return page.getByRole("region", { name: t("pins.heading") });
}

/**
 * Makes `count` projects inside one project (the tree of the sidebar shows only the top, so the sidebar stays as low as
 * it is) and pins them, through the API; their names in the order of the pins.
 */
async function pinProjects(
  page: Page,
  count: number,
  tag: string,
  server: Parameters<typeof apiOf>[1],
): Promise<readonly string[]> {
  const api = apiOf(page, server);
  // What an earlier test of this file left pinned would stand before the new pins.
  const left = dataOf(await api.client.GET("/api/v1/me/pins"));
  await Promise.all(
    left.items.map((pin) =>
      api.client.DELETE("/api/v1/me/pins", {
        params: { query: { kind: pin.kind, target: pin.id } },
      }),
    ),
  );
  const projects = dataOf(
    await api.client.GET("/api/v1/projects", { params: { query: { limit: 100 } } }),
  );
  const top = projects.items.find((each) => each.short_name === project.shortName)?.id ?? "";
  const names = Array.from({ length: count }, (_, number) => `${tag} part ${number + 1}`);
  const made = await Promise.all(
    names.map(async (name, number) =>
      dataOf(
        await api.client.POST("/api/v1/projects/{project}/sub-projects", {
          params: { path: { project: top } },
          body: { short_name: `${tag}${number + 1}`, name },
        }),
      ),
    ),
  );
  await Promise.all(
    made.map((part) =>
      api.client.POST("/api/v1/me/pins", { body: { kind: "project", target: part.id } }),
    ),
  );
  const pins = dataOf(await api.client.GET("/api/v1/me/pins"));
  const mine = new Set(made.map((each) => each.id));
  return pins.items.filter((pin) => mine.has(pin.id)).map((pin) => pin.name);
}

/** Pins the project from its page. */
async function pinFromThePage(page: Page): Promise<void> {
  await signIn(page, ada);
  await page.goto(projectAddress(project.shortName));
  await page.getByRole("button", { name: t("pins.pin"), exact: true }).click();
  await expect(page.getByRole("button", { name: t("pins.unpin"), exact: true })).toBeVisible();
}

test("cfg_05_pinned_stands_in_the_sidebar_and_first_in_more_on_a_phone", async ({ page }) => {
  test.skip(layoutOf(page) === "phone", "the phone has its own test");
  await pinFromThePage(page);
  await expect(pinned(page).getByRole("link", { name: project.name, exact: true })).toBeVisible();
  // Taking the pin off takes the section away.
  await page.getByRole("button", { name: t("pins.unpin"), exact: true }).click();
  await expect(pinned(page)).toHaveCount(0);
});

test("cfg_05_pinned_is_the_first_group_of_more_on_a_phone", async ({ page }) => {
  test.skip(layoutOf(page) !== "phone", "only a phone has the sheet More");
  await pinFromThePage(page);
  await page.getByRole("button", { name: t("shell.more"), exact: true }).click();
  const sheet = page.getByRole("dialog", { name: t("shell.more") });
  await expect(sheet.getByRole("heading", { level: 3 }).first()).toHaveText(t("pins.heading"));
  await expect(sheet.getByRole("link", { name: project.name, exact: true })).toBeVisible();
});

test("cfg_05_more_than_8_pins_fold_and_the_sidebar_fits_a_window_600_px_high", async ({
  page,
  server,
}) => {
  test.skip(layoutOf(page) === "phone", "the sidebar is a computer's");
  await signIn(page, ada);
  const names = await pinProjects(page, SHOWN + 2, "FOLD", server);
  await page.setViewportSize({ width: 1280, height: 900 });
  await page.goto(projectAddress(project.shortName));
  const section = pinned(page);
  // The sidebar shows eight and *Show all pinned*.
  await expect(section.getByRole("listitem")).toHaveCount(SHOWN + 1);
  await section.getByRole("button", { name: t("pins.show-all"), exact: true }).click();
  await expect(section.getByRole("listitem")).toHaveCount(names.length + 1);
  await expect(
    section.getByRole("button", { name: t("pins.show-fewer"), exact: true }),
  ).toBeVisible();
  // In a window 600 px high it shows one, and the sidebar needs no scrolling.
  await page.setViewportSize({ width: 1280, height: 600 });
  await page.goto(projectAddress(project.shortName));
  await expect(section.getByRole("listitem")).toHaveCount(LOW_SHOWN + 1);
  const menu = page.getByRole("navigation", { name: t("shell.menu") });
  const sizes = await menu.evaluate((nav) => ({
    content: nav.scrollHeight,
    shown: nav.clientHeight,
  }));
  expect(sizes.content).toBeLessThanOrEqual(sizes.shown);
});

test("cfg_05_a_pin_is_moved_by_the_keyboard_as_by_drag", async ({ page, server }) => {
  test.skip(layoutOf(page) === "phone", "the menu of a pin is a computer's");
  await signIn(page, ada);
  const [first = "", second = ""] = await pinProjects(page, 2, "MOVE", server);
  await page.goto(projectAddress(project.shortName));
  const section = pinned(page);
  const more = section.getByRole("button", { name: t("pins.menu", { name: second }) });
  await more.focus();
  await page.keyboard.press("Enter");
  await page.getByRole("button", { name: t("pins.up"), exact: true }).focus();
  await page.keyboard.press("Enter");
  const links = section.getByRole("link");
  await expect(links.first()).toHaveAccessibleName(second);
  await expect(links.nth(1)).toHaveAccessibleName(first);
});

test("cfg_05_the_narrow_sidebar_reaches_every_pin_past_the_fold", async ({ page, server }) => {
  test.skip(layoutOf(page) !== "computer", "only a computer narrows the sidebar");
  await signIn(page, ada);
  const names = await pinProjects(page, SHOWN + 2, "NARROW", server);
  await page.setViewportSize({ width: 1280, height: 900 });
  await page.goto(projectAddress(project.shortName));
  await page.getByRole("button", { name: t("shell.narrow"), exact: true }).click();
  const section = pinned(page);
  await expect(section.getByRole("link")).toHaveCount(SHOWN);
  await section.getByRole("button", { name: t("pins.show-all"), exact: true }).click();
  await expect(section.getByRole("link")).toHaveCount(names.length);
});

test("cfg_05_unpin_from_the_menu_puts_the_focus_on_the_menu_of_the_next_pin", async ({
  page,
  server,
}) => {
  test.skip(layoutOf(page) === "phone", "the menu of a pin is a computer's");
  await signIn(page, ada);
  const [first = "", second = ""] = await pinProjects(page, 2, "FOCUS", server);
  await page.goto(projectAddress(project.shortName));
  const section = pinned(page);
  const more = section.getByRole("button", { name: t("pins.menu", { name: first }) });
  await more.focus();
  await page.keyboard.press("Enter");
  await page.getByRole("button", { name: t("pins.unpin"), exact: true }).focus();
  await page.keyboard.press("Enter");
  await expect(
    section.getByRole("button", { name: t("pins.menu", { name: second }) }),
  ).toBeFocused();
});
