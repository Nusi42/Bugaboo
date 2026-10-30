import { t } from "@bugaboo/messages";

import { expect, test } from "./support/fixtures.ts";
import { webIssues } from "./support/issues.ts";
import { layoutOf } from "./support/layout.ts";
import { tickIssues } from "./support/many-changes-pages.ts";
import { openList, prepareLook } from "./support/list-look.ts";
import { partIn } from "./support/parts.ts";
import { boxOf } from "./support/windows.ts";

// Ticking issues (UX §4.2, M5 exit L4 to L7): the bar of the ticked issues stands under the list, so
// ticking moves nothing above it; its buttons carry the count and Delete is the danger button; the
// box in the head shows that some, not all, rows are ticked; a ticked row keeps its tint and has no
// ring of its own.

test.use({ preparation: { run: prepareLook } });

test("ux_4_2 ticking two issues moves nothing above the list and the bar says how many (L4, L5, L6)", async ({
  page,
}) => {
  test.skip(layoutOf(page) === "phone", "a phone ticks one card at a time");
  await openList(page);
  // The head of the list stands where it stood: the bar takes its room from the box of the rows, which takes the rest
  // of the window and scrolls inside (the rows in it may scroll to the ticked one).
  const listHead = partIn(page, "list-head").first();
  const before = await boxOf(listHead);
  await tickIssues(page, ["WEB-1", "WEB-2"]);
  const after = await boxOf(listHead);
  expect(after.y).toBe(before.y);
  const bar = page.getByRole("region", { name: t("issues.selected-bar"), exact: true });
  await expect(
    bar.getByRole("button", { name: t("many-changes.apply", { count: 2 }) }),
  ).toBeVisible();
  const remove = bar.getByRole("button", { name: t("many-changes.apply-delete", { count: 2 }) });
  await expect(remove).toHaveAttribute("data-variant", "danger");
  // Two of three: the box in the head is mixed, not empty.
  const head = page.getByRole("checkbox", { name: t("issues.select-shown"), exact: true });
  await expect(head).toHaveJSProperty("indeterminate", true);
  // The ticked row keeps its tint and has no ring: only the issue that is open has one.
  await expect(partIn(page, "list-row", "grid").filter({ hasText: webIssues.login })).toHaveCSS(
    "box-shadow",
    "none",
  );
});
