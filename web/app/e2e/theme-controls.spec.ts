import { t } from "@bugaboo/messages";
import { type Locator, type Page } from "@playwright/test";

import { openAppearance } from "./support/appearance.ts";
import { expect, test } from "./support/fixtures.ts";
import { layoutOf } from "./support/layout.ts";
import { openList, prepareLook } from "./support/list-look.ts";
import { openBar, openSearch } from "./support/search-bar.ts";
import { partIn } from "./support/parts.ts";
import { inOrder } from "./support/sequence.ts";
import { focusByKeyboard, inEveryLook, resolved } from "./support/styles.ts";

// M5 exit, package T (S11, L21, A6, S10): the controls above a list are one family. One height (the
// control height of the theme: 36 with a mouse), one text size, one baseline, and the label of every
// control is the one muted label color. The unselected choice of *Spacing* (My settings ›
// Appearance) has the edge of a control, not the faint divider (nearly invisible in Daylight and
// Paper). Group by and Customize above a list are quiet words in that family (UX §4.2): no box,
// the color of a link, the name of Group by beside its choice.

test.use({ preparation: { run: prepareLook } });

interface Measured {
  readonly left: number;
  readonly middle: number;
  readonly height: number;
  readonly bottom: number;
  readonly fontSize: string;
}

/** What the browser draws for a control: where it starts, how high, where its middle and bottom are, and the size of its text. */
function measure(control: Locator): Promise<Measured> {
  return control.evaluate((node) => {
    const box = node.getBoundingClientRect();
    return {
      left: box.left,
      middle: box.top + box.height / 2,
      height: box.height,
      bottom: box.bottom,
      fontSize: getComputedStyle(node).fontSize,
    };
  });
}

/** The controls of the options row and of the search bar that a person presses or picks from, by name. */
function controlsOf(page: Page): readonly (readonly [string, Locator])[] {
  const options = partIn(page, "list-options");
  return [
    ["select", options.locator("select")],
    ["Customize", options.getByRole("button")],
    ["filter", partIn(page, "search-filter").first()],
    ["Add filter", partIn(page, "search-filters.action").getByRole("button").first()],
  ];
}

test("ux_7_3 the selects and the buttons of a list are as high as a control and as big in text", async ({
  page,
}) => {
  test.skip(layoutOf(page) === "phone", "a phone's controls are the subject of the phone list");
  await openSearch(page, "priority is high");
  const height = Number((await resolved(page, "height", "var(--size-control)")).replace("px", ""));
  const text = await resolved(page, "font-size", "var(--fs-ui)");
  expect(height).toBe(36);
  await inOrder(
    controlsOf(page).map(([name, control]) => async () => {
      const seen = await measure(control);
      expect(seen.height, `${name} is one control high`).toBeCloseTo(height, 0);
      expect(seen.fontSize, `${name} has the text size of a control`).toBe(text);
    }),
  );
});

test("ux_7_3 the select and Customize stand level, bottoms on one line", async ({ page }) => {
  test.skip(layoutOf(page) === "phone", "Customize is for a computer");
  await openList(page);
  const options = partIn(page, "list-options");
  const select = await measure(options.locator("select"));
  const customize = await measure(options.getByRole("button"));
  expect(customize.bottom).toBeCloseTo(select.bottom, 0);
});

test("ux_4_2 Group by and Customize are quiet words with no box, the name beside the choice, in every theme", async ({
  page,
}) => {
  test.skip(layoutOf(page) === "phone", "a phone has Group by in the sheet of the filters");
  await openList(page);
  const options = partIn(page, "list-options");
  const select = options.locator("select");
  const label = partIn(options, "field.label");
  const customize = options.getByRole("button", { name: t("issues.customize"), exact: true });
  // The name stands before the choice on the same line, not above it.
  const [name, choice] = await Promise.all([measure(label), measure(select)]);
  expect(name.left).toBeLessThan(choice.left);
  expect(Math.abs(name.middle - choice.middle)).toBeLessThanOrEqual(2);
  // Customize has its icon before its word.
  await expect(partIn(customize, "icon")).toHaveCount(1);
  await inEveryLook(page, async (look) => {
    const link = await resolved(page, "color", "var(--accent-fg)");
    const none = await resolved(page, "background-color", "transparent");
    await inOrder(
      [select, customize].map((control) => async () => {
        await expect(control, look).toHaveCSS("color", link);
        await expect(control, look).toHaveCSS("background-color", none);
        await expect(control, look).toHaveCSS("border-top-color", none);
      }),
    );
  });
});

test("ux_4_2 the keyboard focus on Group by and Customize shows the ring", async ({ page }) => {
  test.skip(layoutOf(page) === "phone", "a phone has no keyboard");
  await openList(page);
  const options = partIn(page, "list-options");
  await inOrder(
    [options.locator("select"), options.getByRole("button")].map((control) => async () => {
      await focusByKeyboard(page, control);
      await expect(control).toBeFocused();
      await expect(control).not.toHaveCSS("outline-style", "none");
    }),
  );
});

test("ux_7_3 every label above a control is the one muted color and size, in every theme", async ({
  page,
}) => {
  await openList(page);
  // On a phone Group by is in the sheet of the filters (M5 exit D2).
  await openBar(page);
  const options = partIn(page, "list-options");
  const group = partIn(options, "field.label");
  await inEveryLook(page, async (look) => {
    const muted = await resolved(page, "color", "var(--fg-muted)");
    await expect(group, look).toHaveCSS("color", muted);
    // UX §7.3: field names and control labels are 13.
    const size = await resolved(page, "font-size", "var(--fs-sm)");
    expect((await measure(group)).fontSize, look).toBe(size);
  });
});

test("ux_7_3 the segment of Spacing that is not chosen has the edge of a control and a control's size, in every theme", async ({
  page,
}) => {
  await openAppearance(page);
  const spacing = partIn(page, "choice-group").filter({
    has: page.getByText(t("appearance.spacing.legend"), { exact: true }),
  });
  const legend = partIn(spacing, "choice-group.legend");
  const unchosen = partIn(spacing, "choice-group.label").filter({
    hasText: t("appearance.spacing.compact"),
  });
  await inEveryLook(page, async (look) => {
    const edge = await resolved(page, "border-top-color", "var(--border-control)");
    await expect(unchosen, look).toHaveCSS("border-top-color", edge);
    await expect(legend, look).toHaveCSS("color", await resolved(page, "color", "var(--fg-muted)"));
    const seen = await measure(unchosen);
    expect(seen.fontSize, look).toBe(await resolved(page, "font-size", "var(--fs-ui)"));
  });
});
