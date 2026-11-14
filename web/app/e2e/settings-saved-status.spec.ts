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

/** Where the words of an element are drawn, line by line: their ink, not the box around them. */
function inkOf(words: Locator) {
  return words.evaluate((element) => {
    const range = document.createRange();
    range.selectNodeContents(element);
    const lines = range.getClientRects();
    return Array.from({ length: lines.length }, (_, index) => {
      const line = lines.item(index);
      return {
        left: line?.left ?? 0,
        right: line?.right ?? 0,
        top: line?.top ?? 0,
        bottom: line?.bottom ?? 0,
      };
    });
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

test("ux_6_4 Saved beside a control a page draws itself covers none of its words", async ({
  page,
}) => {
  await signIn(page, ada);
  await savedBesideOwnControl(page, paths.appearance, (here) =>
    here.getByRole("radio", { name: t("theme.midnight"), exact: true }),
  );
  await savedBesideOwnControl(page, paths.themes, (here) =>
    here.getByRole("radio", { name: t("theme.paper"), exact: true }),
  );
  await savedBesideOwnControl(
    page,
    paths.startPage,
    (here) => here.getByRole("combobox", { name: t("nav.start-page"), exact: true }),
    async (found) => {
      await found.selectOption({ index: 1 });
    },
  );
  await savedBesideOwnControl(page, addressInProject(paths.projectSettings, "WEB"), (here) =>
    here.locator("main [data-part='setting-head']").getByRole("switch").first(),
  );
});
