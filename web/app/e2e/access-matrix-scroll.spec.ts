import { t } from "@bugaboo/messages";

import { addressInProject, paths } from "../src/shared/paths.ts";
import { ada, createAdmin } from "./support/accounts.ts";
import { levelList, roleName } from "./support/access.ts";
import { expect, test } from "./support/fixtures.ts";
import { signIn } from "./support/flows.ts";
import { openAppearance, setSwitch } from "./support/appearance.ts";
import { nothing, onThisLayout } from "./support/layout.ts";
import { createProject } from "./support/projects.ts";
import { partIn } from "./support/parts.ts";

// The matrix of roles by fields (M4, the final check): when there are more roles than the page is
// wide, the matrix scrolls and says so in words, and it says nothing when everything fits.

test.use({
  preparation: {
    run: async (server) => {
      const admin = await createAdmin(server);
      await createProject(admin, "ACME", "Acme Corporation");
      await admin.dispose();
    },
  },
});

/** A column too narrow for the matrix, one wider than it can be, and a table wider than that. */
const narrow = "200px";
const wide = "6000px";
const wider = "9000px";

for (const [name, address, subject] of [
  [
    "field permissions",
    addressInProject(paths.projectFields, "ACME"),
    t("issue-field.cost-center"),
  ],
  ["who sees people", paths.peopleVisibility, t("people-kind.assigned-to")],
] as const) {
  test(`ux_04 the matrix of ${name} says that it scrolls exactly when it does`, async ({
    page,
  }) => {
    await signIn(page, ada);
    await page.goto(address);
    const list = await levelList(page, subject, roleName.contributor);
    await expect(list).toBeVisible();
    const scrollers = partIn(page, "access.scroll");
    const scrolls = await scrollers.evaluateAll((nodes: readonly HTMLElement[]) =>
      nodes.some((node) => node.scrollWidth > node.clientWidth),
    );
    await expect(page.getByText(t("access.scrolls"), { exact: true })).toHaveCount(Number(scrolls));
  });
}

test("ux_04 the note follows the matrix when the region or the table changes size, and the region is described by it", async ({
  page,
}) => {
  await signIn(page, ada);
  await page.goto(addressInProject(paths.projectFields, "ACME"));
  await expect(
    await levelList(page, t("issue-field.cost-center"), roleName.contributor),
  ).toBeVisible();
  const region = partIn(page, "access.scroll");
  const note = page.getByText(t("access.scrolls"), { exact: true });
  /** Gives the region a width (or none) the way a narrow column would. */
  const widthOfRegion = (width: string) =>
    region.evaluate((node, value) => {
      node.style.width = value;
    }, width);

  // The region is narrower than the table: it scrolls, it says so, and the note describes it.
  await widthOfRegion(narrow);
  await expect(note).toBeVisible();
  await expect(region).toHaveAccessibleDescription(t("access.scrolls"));
  // The region becomes wider than the table: the note goes, and so does the description.
  await widthOfRegion(wide);
  await expect(note).toHaveCount(0);
  await expect(region).toHaveAccessibleDescription("");
  // The table grows by itself, inside a region that did not change: the note is back.
  await region.locator("table").evaluate((node, value) => {
    node.style.minWidth = value;
  }, wider);
  await expect(note).toBeVisible();
});

const levelLists = '[data-part="access.matrix-cell"] select';

/**
 * For each level list in the matrix: the width the chosen level needs in the list's own type, and
 * the width inside the list that it has for it. The words are measured with the list's own font.
 * It runs in the page, so it takes nothing from outside.
 */
function roomForLevels(): { needed: number; available: number }[] {
  const rooms: { needed: number; available: number }[] = [];
  for (const list of document.querySelectorAll<HTMLSelectElement>(levelLists)) {
    const style = getComputedStyle(list);
    const ruler = document.createElement("span");
    ruler.style.font = style.font;
    ruler.style.whiteSpace = "nowrap";
    ruler.style.position = "absolute";
    ruler.style.visibility = "hidden";
    ruler.textContent = list.selectedOptions[0]?.text ?? "";
    document.body.append(ruler);
    const needed = ruler.getBoundingClientRect().width;
    ruler.remove();
    // A computed padding is always in pixels ("12px").
    const padding =
      Number(style.paddingLeft.slice(0, -2)) + Number(style.paddingRight.slice(0, -2));
    const available = list.clientWidth - padding;
    rooms.push({ needed, available });
  }
  return rooms;
}

/** Makes every level list of the matrix as narrow as a list can be squeezed. */
function squeezeLevels(): void {
  for (const list of document.querySelectorAll<HTMLSelectElement>(levelLists)) {
    list.style.width = "20px";
  }
}

test("ux_04 no level in the matrix is cut with Larger text on, and the measure can tell a cut one", async ({
  page,
}) => {
  await onThisLayout(page, {
    phone: nothing,
    tablet: nothing,
    computer: async () => {
      await openAppearance(page);
      await setSwitch(page, t("appearance.larger-text"), true);
      await expect(page.getByRole("status").filter({ hasText: t("save.saved") })).toBeVisible();
      await page.goto(addressInProject(paths.projectFields, "ACME"));
      await expect(
        await levelList(page, t("issue-field.cost-center"), roleName.contributor),
      ).toBeVisible();
      const rooms = await page.evaluate(roomForLevels);
      expect(rooms.length).toBeGreaterThan(0);
      for (const room of rooms) {
        expect(room.needed).toBeLessThanOrEqual(room.available);
      }
      // The same measure finds lists that are too narrow for their level: the check can fail.
      await page.evaluate(squeezeLevels);
      const squeezed = await page.evaluate(roomForLevels);
      expect(squeezed.length).toBe(rooms.length);
      for (const room of squeezed) {
        expect(room.needed).toBeGreaterThan(room.available);
      }
    },
  });
});
