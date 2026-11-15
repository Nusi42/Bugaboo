import type { Page } from "@playwright/test";

import { signInThroughApi } from "./support/accounts.ts";
import { expect, test } from "./support/fixtures.ts";
import { signIn } from "./support/flows.ts";
import { createLabel } from "./support/labels.ts";
import { PANE_HEAD, SETTINGS_TOPIC } from "./support/pane-head.ts";
import { sampleAdministrator } from "./support/sample.ts";
import { inOrder } from "./support/sequence.ts";
import { editorAddress, uploadedTheme } from "./support/theme-editor.ts";
import { paths } from "../src/shared/paths.ts";

// What stays in sight beside the cards of a topic (an open item beside a list, the preview of a theme) under a head
// that folds (UX §6.5): at the top of the box it stands in its place, while the head folds it moves with what it stands
// beside, and once it sticks it stands under the folded head, nothing over its top.

test.use({ sample: true });

/** A window as high as a laptop's. */
const LAPTOP = { width: 1280, height: 720 } as const;

/** How many more labels than the list shows. */
const LABELS = 40;

/** An open label beside the list of labels. */
const LABEL_PANEL = "[data-part~='list-editor'] [data-part~='side-panel']";

/** The preview beside the groups of colours of the theme editor. */
const PREVIEW = "[data-part~='theme-editor.aside']";

/** Scrolls the box of the topic to `top` and says where `panel` and the bottom of the head then stand. */
function placesAt(
  page: Page,
  panel: string,
  top: number,
): Promise<{ scrolled: number; panel: number; head: number; onTop: boolean }> {
  return page.evaluate(
    async ([box, sticky, head, to]) => {
      const topic = document.querySelector<HTMLElement>(box);
      topic?.scrollTo(0, to);
      await new Promise<void>((done) => {
        requestAnimationFrame(() => {
          requestAnimationFrame(() => {
            done();
          });
        });
      });
      const place = document.querySelector(sticky)?.getBoundingClientRect() ?? new DOMRect();
      const hit = document.elementFromPoint(place.left + place.width / 2, place.top + 2);
      return {
        scrolled: topic?.scrollTop ?? 0,
        panel: place.top,
        head: document.querySelector(head)?.getBoundingClientRect().bottom ?? 0,
        onTop: hit?.closest(sticky) instanceof HTMLElement,
      };
    },
    [SETTINGS_TOPIC, panel, PANE_HEAD, top] as const,
  );
}

/** Where `panel` would stand at the top of the box if it did not stick: its place in the flow. */
function flowPlace(page: Page, panel: string): Promise<number> {
  return page.locator(panel).evaluate((node) => {
    const sticky = node.style.position;
    node.style.position = "static";
    const top = node.getBoundingClientRect().top;
    node.style.position = sticky;
    return top;
  });
}

/** How far the head of the pane folds: how much higher its bottom stands far down the box than at its top. */
async function foldLength(page: Page, panel: string): Promise<number> {
  const top = await placesAt(page, panel, 0);
  const down = await placesAt(page, panel, 100_000);
  return top.head - down.head;
}

/** The panel stands in its place at the top, moves with the box while the head folds, and sticks under the head. */
async function sticksUnderTheHead(page: Page, panel: string): Promise<void> {
  const fold = await foldLength(page, panel);
  expect(fold, "the head folds").toBeGreaterThan(2);
  const start = await placesAt(page, panel, 0);
  expect(start.panel, "at the top it stands in its place").toBeCloseTo(
    await flowPlace(page, panel),
    0,
  );
  // Steps of the scroll that each stay within the fold.
  const steps = [0.25, 0.5, 0.75].map((part) => Math.round(fold * part));
  const folding: { readonly by: number; readonly moved: number }[] = [];
  await inOrder(
    steps.map((by) => async () => {
      const at = await placesAt(page, panel, by);
      folding.push({ by: at.scrolled, moved: Math.round(start.panel - at.panel) });
    }),
  );
  expect(folding, "it moves with the box while the head folds").toEqual(
    folding.map(({ by }) => ({ by, moved: by })),
  );
  // Far down, where it sticks: two places of the scroll show it at the same height.
  const stuck = await placesAt(page, panel, 900);
  const further = await placesAt(page, panel, 950);
  expect(further.panel, "it sticks").toBeCloseTo(stuck.panel, 0);
  expect(stuck.panel, "it sticks under the folded head").toBeGreaterThanOrEqual(stuck.head - 0.5);
  expect(stuck.onTop, "nothing covers its top").toBe(true);
}

test("ux_06 an open label beside the list sticks under the folded head of the page", async ({
  page,
  server,
}) => {
  const admin = await signInThroughApi(server, sampleAdministrator());
  await inOrder(
    Array.from(
      { length: LABELS },
      (_, made) => () => createLabel(admin, `Sticky ${String(made).padStart(2, "0")}`),
    ),
  );
  await admin.dispose();
  await page.setViewportSize(LAPTOP);
  await signIn(page, sampleAdministrator());
  await page.goto(paths.labels);
  await page.locator("[data-part~='list-editor.open']").nth(2).click();
  await expect(page.locator(LABEL_PANEL)).toBeVisible();
  await sticksUnderTheHead(page, LABEL_PANEL);
});

test("ux_06 the preview of a theme sticks under the folded head, beside the verdict of the readability", async ({
  page,
  server,
}) => {
  const theme = await uploadedTheme(server, sampleAdministrator(), "for-everyone", {
    name: "Sticky",
    extends: "dusk",
    values: {},
  });
  await page.setViewportSize(LAPTOP);
  await signIn(page, sampleAdministrator());
  await page.goto(editorAddress(theme));
  await expect(page.locator(PREVIEW)).toBeVisible();
  // Every group open, so the groups are longer than the window: once the groups are there, which come after the preview.
  const groups = page.locator("[data-part~='theme-group'] summary");
  await expect(groups.first()).toBeVisible();
  await inOrder(
    (await groups.all()).map((group) => async () => {
      await group.click();
    }),
  );
  await sticksUnderTheHead(page, PREVIEW);
});
