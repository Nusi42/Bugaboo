import { type Locator, type Page } from "@playwright/test";

import { part } from "../../packages/ui/src/tokens.ts";
import { paths } from "../src/shared/paths.ts";
import { ada, forgetChoices, createAdmin } from "./support/accounts.ts";
import { expect, test } from "./support/fixtures.ts";
import { signIn } from "./support/flows.ts";
import { layoutOf } from "./support/layout.ts";
import { openRoleWithTickedBox } from "./support/roles.ts";
import { focusByKeyboard, inEveryLook, partOn, resolved } from "./support/styles.ts";

// How the parts look when they are pointed at, pressed, given the focus or chosen, with more
// contrast and under forced colors (THM-01, THM-06, ADR-0026, M2C-F92). A test never writes a color
// or a size: it asks the browser what a token or a system color is on this page and compares the
// part with that, in every look.

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

/**
 * The ring of the theme file: a solid line of the ring width, in this color. The focus under forced
 * colors is a double line of twice the width (M5 exit, A16): `double` says so.
 */
async function expectRing(
  page: Page,
  element: Locator,
  color = "var(--accent)",
  double = false,
): Promise<void> {
  await expect(element).toHaveCSS("outline-style", double ? "double" : "solid");
  await expect(element).toHaveCSS(
    "outline-width",
    await resolved(
      page,
      "outline-width",
      double ? "calc(var(--ring-width) * 2)" : "var(--ring-width)",
    ),
  );
  await expect(element).toHaveCSS("outline-color", await resolved(page, "outline-color", color));
}

/** The element has this value of the theme (`var(--bg-hover)`, `Highlight`) as its property. */
async function expectCss(
  page: Page,
  element: Locator,
  property: string,
  value: string,
): Promise<void> {
  await expect(element).toHaveCSS(property, await resolved(page, property, value));
}

/** The label that a choice of a group is drawn with: the radio button itself is invisible. */
function labelOf(input: Locator): Locator {
  return input.locator("xpath=following-sibling::*[1]");
}

/** A part with one of the attributes that React Aria sets (`data-current`) or none of them. */
function partWhere(scope: Page | Locator, name: string, condition: string): Locator {
  return scope.locator(`[data-part="${name}"]${condition}`);
}

/** *My settings › Appearance*: a switch, a group of choices, and the menu. */
async function openAppearance(page: Page): Promise<void> {
  await signIn(page, ada);
  await page.goto(paths.appearance);
  await expect(partOn(page, part.choiceGroupInput).first()).toBeAttached();
}

test("thm_21 a link, a box and a button that are pointed at change as the theme file says", async ({
  page,
}) => {
  await openRoleWithTickedBox(page, "Reviewer hover");
  const link = partOn(page, part.link).first();
  const box = partWhere(page, part.checkbox, ":not([data-selected]):not([data-disabled])").first();
  const button = partWhere(page, part.button, ":not([data-variant]):not([data-disabled])").first();
  await link.hover();
  await inEveryLook(page, () =>
    expectCss(page, link, "text-decoration-thickness", "var(--ring-width)"),
  );
  await box.hover();
  await inEveryLook(page, () =>
    expectCss(page, partOn(box, part.checkboxBox), "background-color", "var(--bg-hover)"),
  );
  await button.hover();
  await inEveryLook(page, () => expectCss(page, button, "background-color", "var(--bg-hover)"));
});

test("thm_21 a part that has the keyboard focus shows the ring in every look", async ({ page }) => {
  await openAppearance(page);
  const sample = partOn(page, part.switch).first();
  const choice = partOn(page, part.choiceGroupInput).first();
  await focusByKeyboard(page, sample.locator("input"));
  await inEveryLook(page, () => expectRing(page, partOn(sample, part.switchTrack)));
  await focusByKeyboard(page, choice);
  await inEveryLook(page, () => expectRing(page, labelOf(choice)));
});

test("thm_21 a checkbox, a button and a link that have the keyboard focus show the ring", async ({
  page,
}) => {
  await openRoleWithTickedBox(page, "Reviewer focus");
  const box = partOn(page, part.checkbox).first();
  const button = partWhere(page, part.button, ":not([data-disabled])").first();
  const link = partOn(page, part.link).first();
  await focusByKeyboard(page, box.locator("input"));
  await inEveryLook(page, () => expectRing(page, partOn(box, part.checkboxBox)));
  await focusByKeyboard(page, button);
  await inEveryLook(page, () => expectRing(page, button));
  await focusByKeyboard(page, link);
  await inEveryLook(page, () => expectRing(page, link));
});

test("thm_21 a chosen choice and a switch that is on show it in every look", async ({ page }) => {
  await openAppearance(page);
  const choice = partOn(page, part.choiceGroupInput).first();
  await choice.check();
  const sample = partOn(page, part.switch).first();
  await sample.click();
  await expect(sample).toHaveAttribute("data-selected", "true");
  await inEveryLook(page, async () => {
    await expectCss(page, labelOf(choice), "border-top-color", "var(--accent)");
    await expectCss(page, labelOf(choice), "background-color", "var(--bg-selected)");
    await expectCss(page, partOn(sample, part.switchTrack), "background-color", "var(--accent)");
  });
});

test("thm_21 a pressed button takes the look of what is pressed", async ({ page }) => {
  await signIn(page, ada);
  await page.goto(paths.roles);
  const button = partWhere(page, part.button, '[data-variant~="primary"]').first();
  await button.hover();
  await page.mouse.down();
  await expect(button).toHaveAttribute("data-pressed", "true");
  await inEveryLook(page, () =>
    expectCss(page, button, "filter", "brightness(var(--hover-brightness))"),
  );
  await page.mouse.move(0, 0);
  await page.mouse.up();
});

test.describe("on a computer the menu is a sidebar", () => {
  test.skip(
    ({ page }) => layoutOf(page) !== "computer",
    "The sidebar is only there on a computer.",
  );

  test("thm_21 an item of the menu is pointed at, pressed, focused and open as the theme file says", async ({
    page,
  }) => {
    await openAppearance(page);
    const item = partWhere(page, part.navItem, ":not([data-current])").first();
    const current = partWhere(page, part.navItem, "[data-current]").first();
    await item.hover();
    await inEveryLook(page, () => expectCss(page, item, "background-color", "var(--bg-hover)"));
    await page.mouse.down();
    await expect(item).toHaveAttribute("data-pressed", "true");
    await inEveryLook(page, () => expectCss(page, item, "background-color", "var(--bg-selected)"));
    await page.mouse.move(0, 0);
    await page.mouse.up();
    await focusByKeyboard(page, item);
    await inEveryLook(page, () => expectRing(page, item));
    await inEveryLook(page, async () => {
      await expectCss(page, current, "background-color", "var(--bg-selected)");
      await expectCss(page, current, "color", "var(--accent-fg)");
    });
  });
});

test("thm_06 with more contrast the parts draw their lines and text with the stronger colors", async ({
  page,
}) => {
  await signIn(page, ada);
  await page.goto(paths.roles);
  const field = partOn(page, part.fieldInput).first();
  const hint = partOn(page, part.muted).first();
  const usual = await resolved(page, "border-top-color", "var(--border-control)");
  await page.emulateMedia({ contrast: "more" });
  await inEveryLook(page, async () => {
    // The edge of a box that is typed in is the edge of a control (3:1), stronger still here.
    await expectCss(page, field, "border-top-color", "var(--border-control)");
    await expectCss(page, hint, "color", "var(--fg-muted)");
  });
  expect(await resolved(page, "border-top-color", "var(--border-control)")).not.toBe(usual);
});

test("thm_20 under forced colors the ring, the chosen choice and the thumb use the device colors", async ({
  page,
}) => {
  await page.emulateMedia({ forcedColors: "active" });
  await openAppearance(page);
  const choice = partOn(page, part.choiceGroupInput).first();
  await choice.check();
  const sample = partOn(page, part.switch).first();
  // The focus is on the switch, so the line around the chosen choice is not the ring of the focus
  // but the mark that forced colors need (fills are dropped).
  await focusByKeyboard(page, sample.locator("input"));
  await inEveryLook(page, async () => {
    await expectRing(page, partOn(sample, part.switchTrack), "Highlight", true);
    await expectRing(page, labelOf(choice), "Highlight");
    await expectCss(page, partOn(sample, part.switchThumb), "background-color", "ButtonText");
  });
});

test.describe("on a computer the open item of the menu is marked under forced colors", () => {
  test.skip(
    ({ page }) => layoutOf(page) !== "computer",
    "The sidebar is only there on a computer.",
  );

  test("thm_20 the item of the page that is open has the ring of the device", async ({ page }) => {
    await page.emulateMedia({ forcedColors: "active" });
    await openAppearance(page);
    const current = partWhere(page, part.navItem, "[data-current]").first();
    await inEveryLook(page, () => expectRing(page, current, "Highlight"));
  });
});
