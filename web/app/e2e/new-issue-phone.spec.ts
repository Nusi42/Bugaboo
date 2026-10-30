import { t } from "@bugaboo/messages";

import { expect, test } from "./support/fixtures.ts";
import { openNewIssueDialog } from "./support/issues.ts";
import { layoutOf } from "./support/layout.ts";
import { openList, prepareLook } from "./support/list-look.ts";
import { partIn } from "./support/parts.ts";
import { resolved } from "./support/styles.ts";
import { boxOf, shadowsOf } from "./support/windows.ts";

// M5 exit, package O (O13): on a phone the fixed footer of New issue holds the hint and the main button only;
// the other answers and the note stand at the end of the scrolling form, and the bar of the screen is the bar
// of every page.

test.use({ preparation: { run: prepareLook } });

test("ux_8 on a phone the footer of New issue is the hint and one button, the rest scrolls", async ({
  page,
}) => {
  test.skip(layoutOf(page) !== "phone", "the footer of a window on a computer has all its buttons");
  await openList(page);
  await openNewIssueDialog(page);
  const dialog = page.getByRole("dialog");
  const screenHeight = await page.evaluate(() => window.innerHeight);
  const footer = await boxOf(partIn(dialog, "window.footer"));
  expect(footer.height, "the fixed footer is a small part of the screen").toBeLessThan(
    screenHeight * 0.25,
  );
  const body = partIn(dialog, "window.body");
  // The form scrolls above the footer, and says so: a soft shadow at each edge that has more behind it.
  await expect.poll(() => shadowsOf(body)).toEqual({ start: 0, end: 1 });
  await expect(body.getByRole("button", { name: t("issues.create-another") })).toHaveCount(1);
  await expect(body.getByText(t("issues.draft-kept"))).toHaveCount(1);
  await expect(partIn(dialog, "window.footer").getByRole("button")).toHaveCount(1);
  // The bar is the bar of every page: Back in the color of a quiet button, the title as large as the one of the top bar.
  await expect(partIn(dialog, "window.control").first()).toHaveCSS(
    "color",
    await resolved(page, "color", "var(--accent-fg)"),
  );
  await expect(partIn(dialog, "window.title")).toHaveCSS(
    "font-size",
    await resolved(page, "font-size", "var(--fs-lg)"),
  );
  await expect(partIn(dialog, "window.title-bar")).toHaveCSS(
    "min-height",
    await resolved(page, "min-height", "var(--size-bar)"),
  );
  await expect(partIn(dialog, "window.title-bar")).toHaveCSS(
    "background-color",
    await resolved(page, "background-color", "var(--bg-canvas)"),
  );
});

test("ux_8 on a phone the bar of New issue is one line, as high as the bar of every page", async ({
  page,
}) => {
  test.skip(layoutOf(page) !== "phone", "a window on a computer has its own bar");
  await openList(page);
  await openNewIssueDialog(page);
  const dialog = page.getByRole("dialog");
  // One line: Back and the title are level, and the bar is as high as the bar of every other page.
  const [back, title, bar] = [
    await boxOf(partIn(dialog, "window.control").first()),
    await boxOf(partIn(dialog, "window.title")),
    await boxOf(partIn(dialog, "window.title-bar")),
  ];
  expect(
    Math.abs(back.y + back.height / 2 - (title.y + title.height / 2)),
    "Back and the title are level",
  ).toBeLessThanOrEqual(2);
  const sizeBar = Number((await resolved(page, "min-height", "var(--size-bar)")).replace("px", ""));
  expect(
    bar.height,
    "the bar is as high as the bar of a page (about 60, not 72)",
  ).toBeLessThanOrEqual(sizeBar + 5);
});

test("ux_8 on a phone the bar of a window shown as a screen is the bar of the page under it: Back of the same size, the title in the same place", async ({
  page,
}) => {
  test.skip(layoutOf(page) !== "phone", "a window on a computer has its own bar");
  await openList(page);
  const topBar = page.getByRole("banner");
  const pageBack = topBar.getByRole("button", { name: t("shell.back"), exact: true });
  const pageLook = {
    back: await boxOf(pageBack),
    size: await pageBack.evaluate((node) => getComputedStyle(node).fontSize),
    title: await boxOf(partIn(topBar, "top-bar.title")),
  };
  await openNewIssueDialog(page);
  const dialog = page.getByRole("dialog");
  const windowBack = partIn(dialog, "window.control").first();
  const windowLook = {
    back: await boxOf(windowBack),
    size: await windowBack.evaluate((node) => getComputedStyle(node).fontSize),
    title: await boxOf(partIn(dialog, "window.title")),
  };
  expect(windowLook.size, "Back is as large as on a page").toBe(pageLook.size);
  expect(
    Math.abs(windowLook.back.x - pageLook.back.x),
    "Back starts where it does on a page",
  ).toBeLessThanOrEqual(1);
  expect(
    Math.abs(windowLook.back.width - pageLook.back.width),
    "Back is as wide",
  ).toBeLessThanOrEqual(1);
  expect(
    Math.abs(windowLook.title.x - pageLook.title.x),
    "the title starts where it does on a page",
  ).toBeLessThanOrEqual(1);
});
