import type { Page } from "@playwright/test";

import { t } from "@bugaboo/messages";

import { ada } from "./support/accounts.ts";
import { expect, test } from "./support/fixtures.ts";
import { signIn } from "./support/flows.ts";
import { inOrder } from "./support/sequence.ts";
import { prepareWeb } from "./support/issues.ts";
import { expectStill, landmarkOf, movementWhile, partLandmark } from "./support/movement.ts";
import {
  coverColors,
  coveredSteps,
  foldsWith,
  headGap,
  keyThrough,
  LOW_WINDOW,
  MAIN_COLUMN,
  PANE_HEAD,
  SETTINGS_MENU,
  SETTINGS_TOPIC,
  wheelOver,
} from "./support/pane-head.ts";
import { paths } from "../src/shared/paths.ts";

// The heading and the introduction of a page of the settings fold to the heading as the rest of the topic scrolls under
// them, and open again at its top (UX §6.5); the menu of the topics beside it is a box of its own, and nothing else
// moves.

test.use({ preparation: { run: prepareWeb } });

/** What a person typed before the window changed. */
const TYPED = "kept while typing";

/** A topic with cards of its own after its first (*My themes*), and one whose sections are all cards. */
const TOPICS = [paths.appearance, paths.labels] as const;

/** Opens a topic of the settings in the window and waits until it is read whole. */
async function openTopic(
  page: Page,
  address: string,
  size: { readonly width: number; readonly height: number } = LOW_WINDOW,
): Promise<void> {
  await page.setViewportSize(size);
  await signIn(page, ada);
  await page.goto(address);
  await expect(page.locator(SETTINGS_TOPIC)).toBeVisible();
  await page.waitForLoadState("networkidle");
}

for (const address of TOPICS) {
  test(`ux_01 the head of a page of settings folds as its topic scrolls and nothing else moves in any frame (${address})`, async ({
    page,
  }) => {
    await openTopic(page, address);
    const topic = page.locator(SETTINGS_TOPIC);
    const movement = await movementWhile(
      page,
      [
        await landmarkOf(page.getByRole("heading", { level: 1 }), "the heading"),
        partLandmark("the menu", "settings-frame.menu"),
        partLandmark("the topic", "settings-topic"),
      ],
      () => wheelOver(page, topic),
    );
    // The head folds and opens again with the scroll; its heading, the menu and the box stay where they are.
    expectStill(movement, { scrolls: ["settings-topic"] });
    await foldsWith(page, topic);
  });
}

test("ux_06 the keyboard brings every control of the topic out from under the folded head", async ({
  page,
}) => {
  await openTopic(page, paths.appearance);
  const topic = page.locator(SETTINGS_TOPIC);
  // From the last control of the topic, with the box at its end and the head folded, back up through the box.
  await topic.locator("button, a[href], input, textarea, select").last().focus();
  await topic.evaluate((node) => {
    node.scrollTop = node.scrollHeight;
  });
  const steps = await keyThrough(page, "Shift+Tab", 12, SETTINGS_TOPIC, PANE_HEAD);
  expect(steps.filter((step) => step.inside).length).toBeGreaterThan(6);
  expect(coveredSteps(steps), "the controls the head covered").toEqual([]);
  // The hidden parts of the controls (the box of a switch) scroll with the topic: the main column never scrolls.
  expect(await page.locator(MAIN_COLUMN).evaluate((node) => node.scrollTop)).toBe(0);
});

test("ux_06 the menu of the topics scrolls in a box of its own beside the topic", async ({
  page,
}) => {
  await openTopic(page, paths.labels);
  const menu = page.locator(SETTINGS_MENU);
  await expect
    .poll(() => menu.evaluate((node) => node.scrollHeight - node.clientHeight))
    .toBeGreaterThan(0);
  // The main column itself never scrolls: the two boxes share it.
  const main = page.locator(MAIN_COLUMN);
  expect(await main.evaluate((node) => node.scrollHeight - node.clientHeight)).toBeLessThanOrEqual(
    0,
  );
});

test("ux_06 on paper the topic stands under the introduction as on the screen", async ({
  page,
}) => {
  await openTopic(page, paths.appearance, { width: 1280, height: 620 });
  const gap = () => headGap(page, `${SETTINGS_TOPIC} > :first-child`);
  const onScreen = await gap();
  await page.emulateMedia({ media: "print" });
  await expect.poll(gap).toBeCloseTo(onScreen, 0);
});

for (const scheme of ["light", "dark"] as const) {
  test(`ux_06 the head of a page of settings covers the topic with the color of the page (${scheme})`, async ({
    page,
  }) => {
    await page.emulateMedia({ colorScheme: scheme });
    await openTopic(page, paths.appearance);
    const colors = await coverColors(page);
    expect(colors.head).toBe(colors.holder);
  });
}

/** The windows a person may pass through: a computer, a phone, a tablet, and back. */
const LAYOUTS = [
  { width: 400, height: 860 },
  { width: 1280, height: 720 },
  { width: 900, height: 1000 },
  { width: 400, height: 860 },
  { width: 900, height: 1000 },
  { width: 1280, height: 720 },
] as const;

for (const [address, field] of [
  [paths.labels, t("labels.field-group-name")],
  [paths.handle, t("handle.label")],
] as const) {
  test(`ux_06 what is typed on a page of settings stays when the window changes its layout (${address})`, async ({
    page,
  }) => {
    await openTopic(page, address, { width: 1280, height: 720 });
    const box = page.getByRole("textbox", { name: field, exact: true }).first();
    await box.fill(TYPED);
    await inOrder(
      LAYOUTS.map((size) => async () => {
        await page.setViewportSize(size);
        await expect(box, `${String(size.width)} px`).toHaveValue(TYPED);
      }),
    );
  });
}
