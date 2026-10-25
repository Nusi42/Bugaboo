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
  // Only a computer draws the matrix; a phone and a tablet choose a role and show its lists.
  await onThisLayout(page, {
    phone: nothing,
    tablet: nothing,
    computer: async () => {
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
    },
  });
});

const levelLists = '[data-part="access.matrix-cell"] select';

/**
 * For each level list in the matrix: the width the list needs to show its chosen level whole, and
 * the width it has. The need is read from a copy of the list that holds only that level and takes
 * the width it wants, so the room of the list's own padding and of the arrow of a native list is
 * counted without being known. It runs in the page, so it takes what it needs as an argument.
 */
function roomForLevels(selector: string): Readonly<{ needed: number; available: number }>[] {
  const rooms: { needed: number; available: number }[] = [];
  for (const list of document.querySelectorAll<HTMLSelectElement>(selector)) {
    const copy = document.createElement("select");
    // The look comes from the theme, by the attributes; the style the test put on the list is not copied.
    for (const name of list.getAttributeNames().filter((one) => one !== "style")) {
      copy.setAttribute(name, list.getAttribute(name) ?? "");
    }
    copy.add(new Option(list.selectedOptions[0]?.text ?? ""));
    copy.style.position = "absolute";
    copy.style.visibility = "hidden";
    copy.style.width = "auto";
    list.after(copy);
    const needed = copy.getBoundingClientRect().width;
    copy.remove();
    rooms.push({ needed, available: list.getBoundingClientRect().width });
  }
  return rooms;
}

/** Makes each level list of the matrix as wide as it needs, less a little: cut under its arrow, where the text itself would still fit. */
function cutLevels({
  selector,
  widths,
}: Readonly<{ selector: string; widths: readonly number[] }>): void {
  const underTheArrow = 4;
  for (const [at, list] of document.querySelectorAll<HTMLSelectElement>(selector).entries()) {
    list.style.width = `${(widths[at] ?? 0) - underTheArrow}px`;
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
      const rooms = await page.evaluate(roomForLevels, levelLists);
      expect(rooms.length).toBeGreaterThan(0);
      for (const room of rooms) {
        expect(room.needed).toBeLessThanOrEqual(room.available);
      }
      // The same measure finds a list that is cut a little, under its arrow: the check can fail.
      await page.evaluate(cutLevels, {
        selector: levelLists,
        widths: rooms.map((room) => room.needed),
      });
      const cut = await page.evaluate(roomForLevels, levelLists);
      expect(cut.length).toBe(rooms.length);
      for (const room of cut) {
        expect(room.needed).toBeGreaterThan(room.available);
      }
    },
  });
});
