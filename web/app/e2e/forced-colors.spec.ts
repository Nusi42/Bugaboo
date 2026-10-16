import { t } from "@bugaboo/messages";
import { themeIds } from "../../packages/ui/src/tokens.ts";

import { createAdmin } from "./support/accounts.ts";
import { expect, test } from "./support/fixtures.ts";
import { openRoleWithTickedBox } from "./support/roles.ts";

// Forced colors (ADR-0026): the values of the tokens that say `forced=` reach every theme, also
// when the page has its own look (`data-look` weighs more than `:root`). A ticked checkbox draws
// its border in the system color Highlight, as the old style of the box did.

test.use({
  preparation: {
    run: async (server) => {
      await (await createAdmin(server)).dispose();
    },
  },
});

/** What the page says about one look. */
interface Found {
  readonly look: string;
  readonly accent: string;
  readonly boxes: readonly string[];
  readonly highlight: string;
}

test("thm_20 under forced colors a ticked checkbox has the border of Highlight in every look", async ({
  page,
}) => {
  await page.emulateMedia({ forcedColors: "active" });
  await openRoleWithTickedBox(page);
  const ticked = page.getByRole("checkbox", { name: t("permission.see-projects"), exact: true });
  await expect(ticked).toBeChecked();
  const found: readonly Found[] = await page.evaluate((looks: readonly string[]) => {
    const reference = document.createElement("div");
    reference.style.borderTopColor = "Highlight";
    reference.style.borderTopStyle = "solid";
    document.body.append(reference);
    const highlight = getComputedStyle(reference).borderTopColor;
    reference.remove();
    return looks.map((look) => {
      document.documentElement.dataset["look"] = look;
      const boxes: string[] = [];
      for (const box of document.querySelectorAll('[data-selected] [data-part="checkbox.box"]')) {
        boxes.push(getComputedStyle(box).borderTopColor);
      }
      return {
        look,
        accent: getComputedStyle(document.documentElement).getPropertyValue("--accent").trim(),
        boxes,
        highlight,
      };
    });
  }, themeIds);
  expect(found.map((one) => one.look)).toEqual([...themeIds]);
  for (const one of found) {
    expect(one.accent, one.look).toBe("Highlight");
    expect(one.boxes.length, one.look).toBeGreaterThan(0);
    expect(new Set(one.boxes), one.look).toEqual(new Set([one.highlight]));
  }
});
