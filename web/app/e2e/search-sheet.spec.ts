import { t } from "@bugaboo/messages";
import { type Locator } from "@playwright/test";

import { createAdmin } from "./support/accounts.ts";
import { expect, test } from "./support/fixtures.ts";
import { createProjectWithIssue, webIssues } from "./support/issues.ts";
import { layoutOf, nothing, onThisLayout } from "./support/layout.ts";
import { partIn } from "./support/parts.ts";
import { box, boxOf, openBar, openSearch } from "./support/search-bar.ts";
import { resolved } from "./support/styles.ts";
import { shadowsOf } from "./support/windows.ts";

// The sheet of the search bar on a phone and the offers of Advanced search (exit polish of M5, S3,
// S4, S14): the offers stand in the flow, and the foot of the sheet is always in the screen.

test.use({
  preparation: {
    run: async (server) => {
      const admin = await createAdmin(server);
      await createProjectWithIssue(admin, "WEB", "Website");
      await admin.dispose();
    },
  },
});

test("qry_03 the offers never push the foot of the sheet off the screen or cover the page", async ({
  page,
}) => {
  await openSearch(page, "assignedTo is me");
  await openBar(page);
  await page.getByRole("button", { name: t("search.bar.to-advanced") }).click();
  await box(page).fill("prio is high and ");
  const offers = page.getByRole("listbox");
  await expect(offers).toBeVisible();
  await expect(offers).not.toHaveCSS("position", "absolute");
  const offered = await boxOf(offers);
  const field = await boxOf(box(page));
  // The offers stand under the box in the flow of the page, they do not float over what follows it.
  expect(offered.y).toBeGreaterThanOrEqual(field.y + field.height);
  await onThisLayout(page, {
    computer: nothing,
    tablet: nothing,
    phone: async () => {
      const sheet = page.getByRole("dialog", { name: t("search.bar.filters") });
      const button = sheet.getByRole("button", { name: t("search.bar.sheet-done"), exact: true });
      await expect(button).toBeInViewport({ ratio: 1 });
      const [sheetBox, buttonBox] = await Promise.all([boxOf(sheet), boxOf(button)]);
      // The offers start above the foot (the body of the sheet scrolls to the rest), and the foot
      // stays where it is, in the screen.
      expect(offered.y).toBeLessThan(buttonBox.y);
      expect(buttonBox.y + buttonBox.height).toBeLessThanOrEqual(sheetBox.y + sheetBox.height);
    },
  });
});

test("qry_06 on a phone Save search stands with Done in the foot of the sheet", async ({
  page,
}) => {
  await openSearch(page, "priority is High");
  await onThisLayout(page, {
    computer: nothing,
    tablet: nothing,
    phone: async () => {
      await openBar(page);
      const sheet = page.getByRole("dialog", { name: t("search.bar.filters") });
      const save = sheet.getByRole("button", { name: t("search.save"), exact: true });
      const done = sheet.getByRole("button", { name: t("search.bar.sheet-done"), exact: true });
      // The sheet slides in from the bottom: the boxes are read together, until it stands still.
      await expect
        .poll(async () => {
          const [sheetBox, saveBox, doneBox] = await Promise.all([
            boxOf(sheet),
            boxOf(save),
            boxOf(done),
          ]);
          // The foot of a window has one pattern (package D): Done as wide as the foot, Save search
          // under it, and the whole foot inside the sheet.
          // The least of the three gaps (aligned, under, inside) is not negative when all hold.
          return Math.min(
            1 - Math.abs(saveBox.x - doneBox.x),
            saveBox.y - (doneBox.y + doneBox.height) + 1,
            sheetBox.y + sheetBox.height - (saveBox.y + saveBox.height),
          );
        })
        .toBeGreaterThanOrEqual(0);
    },
  });
});

/** How far the focus ring of an element reaches out of its box, in pixels. */
function ringReach(element: Locator): Promise<number> {
  return element.evaluate((node) => {
    const style = getComputedStyle(node);
    return (
      Number(style.outlineWidth.replace("px", "")) + Number(style.outlineOffset.replace("px", ""))
    );
  });
}

test("qry_03 on a phone the body of the Filters sheet scrolls under a shadow above a divided foot, and the ring has room (M5 exit F-A6)", async ({
  page,
}) => {
  test.skip(layoutOf(page) !== "phone", "only a phone has the sheet");
  await page.setViewportSize({ width: 360, height: 640 });
  await openSearch(page, "assigned to is me and priority >= High");
  await openBar(page);
  const sheet = page.getByRole("dialog", { name: t("search.bar.filters") });
  const body = partIn(sheet, "search-bar").first();
  await expect(sheet.getByText(t("appearance.spacing.legend"), { exact: true })).toBeAttached();
  // Group by and Spacing do not fit at this height: the body scrolls, and says so with the shared shadow.
  await expect
    .poll(() => body.evaluate((node) => node.scrollHeight > node.clientHeight + 1))
    .toBe(true);
  await expect.poll(() => shadowsOf(body)).toEqual({ start: 0, end: 1 });
  await body.evaluate((node) => {
    node.scrollTop = node.scrollHeight;
  });
  await expect.poll(() => shadowsOf(body)).toEqual({ start: 1, end: 0 });
  const spacing = sheet.getByText(t("appearance.spacing.legend"), { exact: true });
  const [bodyBox, spacingBox] = await Promise.all([boxOf(body), boxOf(spacing)]);
  expect(spacingBox.y + spacingBox.height).toBeLessThanOrEqual(bodyBox.y + bodyBox.height);
  // The foot is told from what scrolls above it by the divider of a window's foot.
  const foot = partIn(sheet, "bottom-sheet.actions");
  await expect(foot).toHaveCSS(
    "border-top-color",
    await resolved(page, "border-top-color", "var(--divider)"),
  );
  await expect(foot).not.toHaveCSS("border-top-width", "0px");
  // The ring of the box of Advanced search is drawn whole: the body that clips leaves room at both sides.
  await body.evaluate((node) => {
    node.scrollTop = 0;
  });
  await sheet.getByRole("button", { name: t("search.bar.to-advanced") }).click();
  await box(page).focus();
  const frame = partIn(sheet, "search-text").first();
  const [reach, frameBox, clip] = await Promise.all([ringReach(frame), boxOf(frame), boxOf(body)]);
  expect(reach).toBeGreaterThan(0);
  expect(frameBox.x - reach).toBeGreaterThanOrEqual(clip.x);
  expect(frameBox.x + frameBox.width + reach).toBeLessThanOrEqual(clip.x + clip.width);
});

test("qry_03 the offers that do not fit show the shadow where more follows (M5 exit F-A6, L6)", async ({
  page,
}) => {
  await openSearch(page, "priority is High");
  await openBar(page);
  await page.getByRole("button", { name: t("search.bar.to-advanced") }).click();
  await box(page).fill("prio is high and ");
  const offers = page.getByRole("listbox");
  await expect(offers).toBeVisible();
  await expect
    .poll(() => offers.evaluate((node) => node.scrollHeight > node.clientHeight + 1))
    .toBe(true);
  await expect.poll(() => shadowsOf(offers)).toEqual({ start: 0, end: 1 });
});

test("qry_01 the help that does not fit shows the shadow where more follows (M5 exit F-A12, L6)", async ({
  page,
}) => {
  test.skip(layoutOf(page) === "tablet", "the computer and the phone show the help");
  await openSearch(page, "priority is High");
  await openBar(page);
  await page.getByRole("button", { name: t("search.help"), exact: true }).click();
  const help = page.getByRole("dialog", { name: t("search.help-title") });
  const panel = partIn(help, "search-panel");
  await expect(panel).toBeVisible();
  await expect
    .poll(() => panel.evaluate((node) => node.scrollHeight > node.clientHeight + 1))
    .toBe(true);
  await expect.poll(() => shadowsOf(panel)).toEqual({ start: 0, end: 1 });
});

test("qry_02 the issue finder writes a long title out on more lines, never cut with dots (M5 exit F-A7)", async ({
  page,
}) => {
  await openSearch(page, "priority is High");
  await openBar(page);
  await page.getByRole("button", { name: t("search.bar.add-filter") }).click();
  await page.getByRole("button", { name: t("search.bar.filter-by", { field: "Part of" }) }).click();
  await page.getByRole("searchbox", { name: t("search.bar.issue-search") }).fill("login");
  const name = page.locator("[data-part='picker.choice'] [data-part='picker.name']").first();
  await expect(name).toContainText(webIssues.login);
  await expect(name).not.toHaveCSS("white-space", "nowrap");
  expect(await name.evaluate((node) => node.scrollWidth <= node.clientWidth)).toBe(true);
});
