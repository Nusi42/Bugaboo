import { t } from "@bugaboo/messages";

import { expect, test } from "./support/fixtures.ts";
import { openNewIssueDialog } from "./support/issues.ts";
import { layoutOf } from "./support/layout.ts";
import { openList, prepareLook } from "./support/list-look.ts";
import { partIn } from "./support/parts.ts";
import { resolved } from "./support/styles.ts";
import { boxOf } from "./support/windows.ts";

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
