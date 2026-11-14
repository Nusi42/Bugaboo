import { type Locator, type Page } from "@playwright/test";

import { t } from "@bugaboo/messages";

import { ada } from "./support/accounts.ts";
import { expect, test } from "./support/fixtures.ts";
import { signIn } from "./support/flows.ts";
import { prepareWeb } from "./support/issues.ts";
import { savedWords } from "./support/saved.ts";
import { boxOf } from "./support/windows.ts";
import { addressInProject, paths } from "../src/shared/paths.ts";

// *Saving…* and *Saved* where the change was made (UX §6.4, M8 WP-13): every place that says them
// has its status from the start, so a screen reader hears the first change, and the words take a
// place that is there anyway: they cover nothing and push nothing.

test.use({ preparation: { run: prepareWeb } });

/**
 * Where the words of an element are drawn, line by line: their ink, not the boxes around them (a frame of a setting may
 * stand under the cell of *Saved* while its words do not).
 */
function inkOf(words: Locator) {
  return words.evaluate((element) => {
    const found: { left: number; right: number; top: number; bottom: number }[] = [];
    const texts = document.createTreeWalker(element, NodeFilter.SHOW_TEXT);
    const range = document.createRange();
    for (let text = texts.nextNode(); text !== null; text = texts.nextNode()) {
      range.selectNodeContents(text);
      for (const line of range.getClientRects()) {
        found.push({ left: line.left, right: line.right, top: line.top, bottom: line.bottom });
      }
    }
    return found;
  });
}

/** Where one line of words is drawn. */
type Ink = Readonly<{ left: number; right: number; top: number; bottom: number }>;

/** Whether the words of two elements share any room on the screen, on any of their lines. */
async function overlap(one: Locator, other: Locator): Promise<boolean> {
  const [lines, others] = await Promise.all([inkOf(one), inkOf(other)]);
  return lines.some((a: Ink) =>
    others.some(
      (b: Ink) => a.left < b.right && b.left < a.right && a.top < b.bottom && b.top < a.bottom,
    ),
  );
}

test("ux_6_4 every setting has its status from the start, and Saved covers none of its words", async ({
  page,
}) => {
  await signIn(page, ada);
  await page.goto(paths.appearance);
  const heads = page.locator("main [data-part='setting-head']");
  await expect(heads.first()).toBeVisible();
  // One status in each box, silent, before anything changed.
  await expect(heads.getByRole("status")).toHaveCount(await heads.count());
  const name = t("appearance.less-motion");
  const head = heads.filter({ has: page.getByRole("switch", { name, exact: true }) });
  await head.getByText(name, { exact: true }).click();
  const said = head.getByRole("status").filter({ hasText: savedWords });
  await expect(said).toBeVisible();
  expect(await overlap(said, head.getByText(name, { exact: true }))).toBe(false);
});

test("ux_6_4 a change the server refuses says Not saved yet in the head of its setting, on its first line", async ({
  page,
}) => {
  await signIn(page, ada);
  await page.goto(paths.appearance);
  const name = t("appearance.less-motion");
  const head = page
    .locator("main [data-part='setting-head']")
    .filter({ has: page.getByRole("switch", { name, exact: true }) });
  await expect(head).toBeVisible();
  await page.route(
    "**/api/v1/me/settings/*",
    async (route) => {
      await route.fulfill({
        status: 403,
        contentType: "application/problem+json",
        body: JSON.stringify({
          type: "about:blank",
          title: "Forbidden",
          status: 403,
          code: "access.forbidden",
          message_key: "error.access.forbidden",
        }),
      });
    },
    { times: 1 },
  );
  await head.getByText(name, { exact: true }).click();
  const said = head.getByRole("status").filter({ hasText: t("save.not-saved") });
  await expect(said).toBeVisible();
  // One line, beside the name and inside the head: the narrow head of a phone has room for it too.
  const [words, named, whole] = await Promise.all([
    inkOf(said),
    inkOf(head.getByText(name, { exact: true })),
    boxOf(head),
  ]);
  expect(words).toHaveLength(1);
  const firstLineEnd = Math.min(...named.map((line: Ink) => line.bottom));
  expect(words.map((line: Ink) => line.top < firstLineEnd)).toEqual([true]);
  expect(words.map((line: Ink) => line.right <= whole.x + whole.width + 1)).toEqual([true]);
  expect(await overlap(said, head.getByText(name, { exact: true }))).toBe(false);
});

test("ux_6_4 the notification settings say Saved in a row that is there from the start", async ({
  page,
}) => {
  await signIn(page, ada);
  await page.goto(paths.notificationSettings);
  const row = page.locator("main [data-part='customize-actions']");
  await expect(row.getByRole("status")).toHaveCount(1);
  await page
    .getByRole("switch", { name: t("notifications.in-app"), exact: true })
    .first()
    .locator("xpath=ancestor::label[1]")
    .click();
  await expect(row.getByRole("status").filter({ hasText: savedWords })).toBeVisible();
});

test("ux_6_4 Saved in the head of a card stays on the line of its heading", async ({ page }) => {
  await signIn(page, ada);
  await page.goto(paths.workTimes);
  const card = page.getByRole("region", { name: t("work-times.usual"), exact: true });
  await card.getByRole("checkbox").first().locator("xpath=ancestor::label[1]").click();
  const said = card.getByRole("status").filter({ hasText: savedWords });
  await expect(said).toBeVisible();
  const [heading, words] = await Promise.all([
    boxOf(card.getByRole("heading", { name: t("work-times.usual") })),
    boxOf(said),
  ]);
  // The words start within the height of the heading's line, not on a line of their own under it.
  expect(words.y).toBeLessThan(heading.y + heading.height);
});

test("ux_6_4 only the first line of a setting gives room to Saved, the box under it is as wide as the setting", async ({
  page,
}) => {
  await signIn(page, ada);
  await page.goto(paths.organizationSecurity);
  const input = "[data-part='field.input']";
  const head = page.locator("main [data-part='setting-head']").filter({ has: page.locator(input) });
  await expect(head.first()).toBeVisible();
  const [whole, box] = await Promise.all([
    boxOf(head.first()),
    boxOf(head.first().locator(input).first()),
  ]);
  expect(box.x).toBeCloseTo(whole.x, 0);
  expect(box.width).toBeCloseTo(whole.width, 0);
});

test("ux_6_4 a group of choices gives only its name to the line of Saved, and its choices run the whole width", async ({
  page,
}) => {
  await signIn(page, ada);
  await page.goto(addressInProject(paths.projectSettings, "WEB"));
  const choices = "[data-part='choice-group.choices']";
  const head = page
    .locator("main [data-part='setting-head']")
    .filter({ has: page.locator(choices) });
  await expect(head.first()).toBeVisible();
  const [whole, row, name, status] = await Promise.all([
    boxOf(head.first()),
    boxOf(head.first().locator(choices)),
    boxOf(head.first().locator("[data-part='choice-group.legend']")),
    boxOf(head.first().locator("[data-part~='save-status']")),
  ]);
  expect(row.x).toBeCloseTo(whole.x, 0);
  expect(row.width).toBeCloseTo(whole.width, 0);
  // The name stands in the first line, beside the cell of Saved and never under it.
  expect(name.y).toBeCloseTo(status.y, 0);
  expect(name.x + name.width).toBeLessThanOrEqual(status.x);
});

/**
 * Presses a control of a page that draws its own control for a setting (UX §6.4): *Saved* comes in
 * the head of its box and covers none of the control's words.
 */
async function savedBesideOwnControl(
  page: Page,
  path: string,
  control: (page: Page) => Locator,
  press: (control: Locator) => Promise<void> = (found) =>
    found.locator("xpath=ancestor::label[1]").click(),
) {
  await page.goto(path);
  const head = control(page).locator("xpath=ancestor::*[@data-part='setting-head'][1]");
  await press(control(page));
  const said = head.getByRole("status").filter({ hasText: savedWords });
  await expect(said).toBeVisible();
  expect(await overlap(said, head.locator("> :not([data-part='save-status'])"))).toBe(false);
}

/** A theme of the setting that offers the theme `named`, other than the one chosen now. */
function otherTheme(page: Page, named: string): Locator {
  return page
    .locator("main [data-part='setting-head']")
    .filter({ has: page.getByRole("radio", { name: named, exact: true }) })
    .getByRole("radio", { checked: false })
    .first();
}

test("ux_6_4 Saved beside a control a page draws itself covers none of its words", async ({
  page,
}) => {
  await signIn(page, ada);
  // Each press picks a value other than the one shown, so it changes the setting whatever an
  // earlier test left there.
  await savedBesideOwnControl(page, paths.appearance, (here) =>
    otherTheme(here, t("theme.midnight")),
  );
  await savedBesideOwnControl(page, paths.themes, (here) => otherTheme(here, t("theme.paper")));
  await savedBesideOwnControl(
    page,
    paths.startPage,
    (here) => here.getByRole("combobox", { name: t("nav.start-page"), exact: true }),
    async (found) => {
      const other = await found.locator("option:not(:checked)").first().textContent();
      await found.selectOption({ label: String(other) });
    },
  );
  await savedBesideOwnControl(page, addressInProject(paths.projectSettings, "WEB"), (here) =>
    here.locator("main [data-part='setting-head']").getByRole("switch").first(),
  );
});
