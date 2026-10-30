import { t } from "@bugaboo/messages";
import { type Locator, type Page } from "@playwright/test";

import { paths } from "../src/shared/paths.ts";
import { ada, createAdmin } from "./support/accounts.ts";
import { levelList, roleName } from "./support/access.ts";
import { expect, test } from "./support/fixtures.ts";
import { signIn } from "./support/flows.ts";
import { layoutOf } from "./support/layout.ts";
import { partIn } from "./support/parts.ts";
import { boxOf } from "./support/windows.ts";

// M5 exit, the final pass (L2): on a computer *Who sees people* has the chooser of a role for its preview, and the
// column of the chosen role is scrolled into view in the matrix and marked; before, the matrix showed the first roles
// while the preview spoke of Guest, the last one.

test.use({
  preparation: {
    run: async (server) => {
      const admin = await createAdmin(server);
      await admin.dispose();
    },
  },
});

/** Half a pixel either way: boxes are measured in fractions. */
const ROUNDING = 0.5;

/** The head of the column of a role in the matrix. */
function columnHead(page: Page, role: string): Locator {
  return partIn(page, "access.matrix-cell").and(
    page.getByRole("columnheader", { name: role, exact: true }),
  );
}

/** The column of the role stands whole in the region, right of the names that stay at the start, and is marked. */
async function expectChosenInView(page: Page, role: string): Promise<void> {
  const head = columnHead(page, role);
  await expect(head).toHaveAttribute("data-state", /chosen/u);
  // Screen readers hear it too, not only see its fill.
  await expect(head).toHaveAttribute("aria-current", "true");
  const region = await boxOf(partIn(page, "access.scroll"));
  const names = await boxOf(partIn(page, "access.matrix-cell", "head").first());
  await expect
    .poll(async () => {
      const box = await boxOf(head);
      return (
        box.x >= names.x + names.width - ROUNDING &&
        box.x + box.width <= region.x + region.width + ROUNDING
      );
    })
    .toBe(true);
}

test("priv_12 the chosen role's column of Who sees people is in view and marked on a computer", async ({
  page,
}) => {
  test.skip(layoutOf(page) !== "computer", "a phone and a tablet show the chosen role alone");
  await signIn(page, ada);
  await page.goto(paths.peopleVisibility);
  await expect(
    await levelList(page, t("people-kind.assigned-to"), roleName.administrator),
  ).toBeVisible();
  // The table opens on Guest, the last role, which is out of view at the start of the matrix.
  await expectChosenInView(page, roleName.guest);
  await expect(columnHead(page, roleName.administrator)).not.toHaveAttribute(
    "data-state",
    /chosen/u,
  );
  // Choosing the first role scrolls back to it, and the mark goes with the choice.
  await page
    .getByLabel(t("common.choose-role"), { exact: true })
    .selectOption({ label: roleName.administrator });
  await expectChosenInView(page, roleName.administrator);
  await expect(columnHead(page, roleName.guest)).not.toHaveAttribute("data-state", /chosen/u);
  await expect(columnHead(page, roleName.guest)).not.toHaveAttribute("aria-current", "true");
});
