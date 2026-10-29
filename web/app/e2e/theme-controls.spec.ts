import { type Locator, type Page } from "@playwright/test";

import { expect, test } from "./support/fixtures.ts";
import { layoutOf } from "./support/layout.ts";
import { openList, prepareLook } from "./support/list-look.ts";
import { openSearch } from "./support/search-bar.ts";
import { partIn } from "./support/parts.ts";
import { inOrder } from "./support/sequence.ts";
import { inEveryLook, resolved } from "./support/styles.ts";

// M5 exit, package T (S11, L21, A6, S10): the controls above a list are one family. One height (the
// control height of the theme: 36 with a mouse), one text size, one baseline, and the label of every
// control is the one muted label color. The unselected choice of *Spacing* has the edge of a control,
// not the faint divider (nearly invisible in Daylight and Paper).

test.use({ preparation: { run: prepareLook } });

interface Measured {
  readonly height: number;
  readonly bottom: number;
  readonly fontSize: string;
}

/** What the browser draws for a control: how high, where its bottom is, and the size of its text. */
function measure(control: Locator): Promise<Measured> {
  return control.evaluate((node) => {
    const box = node.getBoundingClientRect();
    return {
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
    ["segment", partIn(options, "choice-group.label").first()],
    ["Customize", options.getByRole("button")],
    ["filter", partIn(page, "search-filter").first()],
    ["Add filter", partIn(page, "search-filters.action").getByRole("button").first()],
  ];
}

test("ux_7_3 the selects, the segments and the buttons of a list are as high as a control and as big in text", async ({
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

test("ux_7_3 the select, the segments and Customize stand level, bottoms on one line", async ({
  page,
}) => {
  test.skip(layoutOf(page) === "phone", "Customize is for a computer");
  await openList(page);
  const options = partIn(page, "list-options");
  const select = await measure(options.locator("select"));
  const segment = await measure(partIn(options, "choice-group.label").first());
  const customize = await measure(options.getByRole("button"));
  expect(segment.bottom).toBeCloseTo(select.bottom, 0);
  expect(customize.bottom).toBeCloseTo(select.bottom, 0);
});

test("ux_7_3 every label above a control is the one muted color and size, in every theme", async ({
  page,
}) => {
  await openList(page);
  const options = partIn(page, "list-options");
  const group = partIn(options, "field.label");
  const spacing = partIn(options, "choice-group.legend");
  await inEveryLook(page, async (look) => {
    const muted = await resolved(page, "color", "var(--fg-muted)");
    await expect(group, look).toHaveCSS("color", muted);
    await expect(spacing, look).toHaveCSS("color", muted);
    // UX §7.3: field names and control labels are 13.
    const size = await resolved(page, "font-size", "var(--fs-sm)");
    expect((await measure(spacing)).fontSize, look).toBe(size);
    expect((await measure(group)).fontSize, look).toBe(size);
  });
});

test("ux_7_3 the segment that is not chosen has the edge of a control, in every theme", async ({
  page,
}) => {
  await openList(page);
  const segments = partIn(partIn(page, "list-options"), "choice-group.label");
  const unchosen = segments.filter({ hasText: /compact/iu });
  await inEveryLook(page, async (look) => {
    const edge = await resolved(page, "border-top-color", "var(--border-control)");
    await expect(unchosen, look).toHaveCSS("border-top-color", edge);
  });
});
