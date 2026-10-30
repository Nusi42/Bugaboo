import { t } from "@bugaboo/messages";
import { type Page } from "@playwright/test";

import { ada, createAdmin } from "./support/accounts.ts";
import { expect, test } from "./support/fixtures.ts";
import { signIn } from "./support/flows.ts";
import { layoutOf } from "./support/layout.ts";
import { boxOf } from "./support/windows.ts";
import { paths } from "../src/shared/paths.ts";

// M5 exit, final pass, package A: the places of search in the shell. On a phone the Search tab of the bar is current on
// the Search page (F-A2); a section of the sidebar or of the sheet More with nothing in it keeps its heading and says so in
// one line, the projects and the saved searches alike (F-A13, UX §3.1); quick search of a phone keeps the end of a long text
// in view beside its clear mark (F-A7).

test.use({
  preparation: {
    run: async (server) => {
      await (await createAdmin(server)).dispose();
    },
  },
});

/** The dialog of quick search and its box. */
function quickSearch(page: Page) {
  const dialog = page.getByRole("dialog", { name: t("search.title"), exact: true });
  return { dialog, box: dialog.getByRole("searchbox", { name: t("search.label"), exact: true }) };
}

/** The sections of the sidebar, or of the sheet More on a phone, opened. */
async function sectionsOf(page: Page) {
  if (layoutOf(page) === "phone") {
    await page.getByRole("button", { name: t("shell.more"), exact: true }).click();
    return page.getByRole("dialog", { name: t("shell.more") });
  }
  return page.getByRole("navigation", { name: t("shell.menu"), exact: true });
}

test("cfg_08 on a phone the Search tab of the bar is current on the Search page", async ({
  page,
}) => {
  test.skip(layoutOf(page) !== "phone", "only a phone has the bar at the bottom");
  await signIn(page, ada);
  await page.goto(paths.search);
  const bar = page.getByRole("navigation", { name: t("shell.menu"), exact: true });
  const search = bar.getByRole("button", { name: t("shell.search"), exact: true });
  await expect(search).toHaveAttribute("aria-current", "page");
  const home = bar.getByRole("link", { name: t("nav.home"), exact: true });
  const fills = await Promise.all(
    [search, home].map((tab) => tab.evaluate((node) => getComputedStyle(node).backgroundColor)),
  );
  expect(fills[0], "the current tab is drawn like a current tab").not.toBe(fills[1]);
});

test("qry_05 an empty section says so in one line, the saved searches like the projects", async ({
  page,
}) => {
  test.skip(layoutOf(page) === "tablet", "the computer and the phone show both places");
  await signIn(page, ada);
  const sections = await sectionsOf(page);
  const projects = sections.getByRole("region", { name: t("projects.heading"), exact: true });
  const saved = sections.getByRole("region", { name: t("search.saved-heading"), exact: true });
  await expect(projects.getByText(t("projects.tree-empty"), { exact: true })).toBeVisible();
  await expect(saved.getByText(t("search.saved-none"), { exact: true })).toBeVisible();
  // The two notes are one part, so they look alike.
  await expect(projects.locator("p")).toHaveAttribute("data-part", "sidebar.note");
  await expect(saved.locator("p")).toHaveAttribute("data-part", "sidebar.note");
});

test("cfg_08 on a phone a long text in quick search keeps its end in view, beside the clear mark (M5 exit F-A7)", async ({
  page,
}) => {
  test.skip(layoutOf(page) !== "phone", "the box of a phone is the narrow one");
  await signIn(page, ada);
  await page
    .getByRole("navigation", { name: t("shell.menu"), exact: true })
    .getByRole("button", { name: t("shell.search"), exact: true })
    .click();
  const { dialog, box } = quickSearch(page);
  await box.fill("Image gallery does not save the name of the picture");
  const clear = dialog.getByRole("button", { name: t("search.clear"), exact: true });
  const [field, mark] = await Promise.all([boxOf(box), boxOf(clear)]);
  // The words end before the clear mark starts, and the end of the text is what the box shows.
  expect(field.x + field.width).toBeLessThanOrEqual(mark.x);
  await expect
    .poll(() =>
      box.evaluate((input) => input.scrollLeft + input.clientWidth >= input.scrollWidth - 1),
    )
    .toBe(true);
});
