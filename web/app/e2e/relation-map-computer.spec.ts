import { t } from "@bugaboo/messages";
import type { Locator, Page } from "@playwright/test";

import { issueAddress } from "../src/shared/paths.ts";
import { ana } from "./support/access.ts";
import { ada } from "./support/accounts.ts";
import { expect, test } from "./support/fixtures.ts";
import { signIn } from "./support/flows.ts";
import { expectStill, landmarkOf, movementWhile } from "./support/movement.ts";
import { inOrder } from "./support/sequence.ts";
import {
  cardKeys,
  cardOf,
  chainPanel,
  kindWords,
  mapList,
  mapped,
  openMap,
  prepareRelationMap,
} from "./support/relation-map.ts";

// The relation map on a computer (ISS-16, UX §4.15, ADR-0009 amendment 2026-11-24): the start on
// the left and a column for each step away, the chain of a pointed issue, *Start from here*,
// *Kinds shown* and *Steps away*, the list form, *Show as › Map* of a search, and nothing drawn
// that the answer does not hold.

test.use({ preparation: { run: prepareRelationMap } });

/** Where a card stands across the picture. */
async function leftOf(card: Locator): Promise<number> {
  return (await card.boundingBox())?.x ?? Number.NaN;
}

/** The sentence of a chain, as the side and the list write it. */
function chainSentence(first: string, word: string, second: string, then: string, last: string) {
  const lead = t("relation-map.lead-next", {
    lead: t("relation-map.lead-first", { key: first, word }),
    key: second,
    word: then,
  });
  return t("relation-map.sentence", { lead, key: last });
}

/** WEB-1 blocks WEB-4, which is blocked by: the chain up to WEB-5. */
const timesheetLead = t("relation-map.lead-next", {
  lead: t("relation-map.lead-first", { key: mapped.start, word: kindWords.blocks }),
  key: mapped.notes,
  word: "is blocked by",
});

/** WEB-1 blocks WEB-4, which is blocked by WEB-5. */
const timesheetChain = chainSentence(
  mapped.start,
  kindWords.blocks,
  mapped.notes,
  "is blocked by",
  mapped.timesheet,
);

/** The relations between issues of WEB: WEB-1 blocks WEB-4, WEB-5 blocks WEB-4, WEB-1 related to WEB-2, WEB-6 blocks WEB-2, WEB-7 causes WEB-1. */
const webRelations = 5;

/** The color a token of the theme gives, as the browser writes a computed color. */
function tokenColor(page: Page, token: string): Promise<string> {
  return page.evaluate((name) => {
    const probe = document.createElement("span");
    probe.style.color = `var(--${name})`;
    document.body.append(probe);
    const color = getComputedStyle(probe).color;
    probe.remove();
    return color;
  }, token);
}

/** Presses a chip of *Kinds shown*. */
async function toggleKind(page: Page, word: string): Promise<void> {
  await page.getByRole("button", { name: word, exact: true }).click();
}

test("iss_16_show_relation_map_draws_the_start_on_the_left_and_columns_by_steps_away", async ({
  page,
}) => {
  await signIn(page, ada);
  await page.goto(issueAddress(mapped.start));
  await page
    .getByRole("button", { name: t("issues.more"), exact: true })
    .first()
    .click();
  await page
    .getByRole("dialog", { name: t("issues.more") })
    .getByRole("button", { name: t("relation-map.show"), exact: true })
    .click();
  await expect(
    page.getByRole("heading", { name: t("relation-map.title", { key: mapped.start }) }),
  ).toBeVisible();
  const start = await leftOf(cardOf(page, mapped.start));
  const first = await Promise.all(
    [mapped.notes, mapped.related, mapped.reset].map((key) => leftOf(cardOf(page, key))),
  );
  const second = await Promise.all(
    [mapped.timesheet, mapped.design].map((key) => leftOf(cardOf(page, key))),
  );
  expect(new Set(first).size).toBe(1);
  expect(new Set(second).size).toBe(1);
  expect(start).toBeLessThan(Math.min(...first));
  expect(Math.max(...first)).toBeLessThan(Math.min(...second));
  await expect(cardOf(page, mapped.start)).toContainText(t("relation-map.start"));
  await expect(
    page.getByText(t("relation-map.column", { count: 2 }), { exact: true }),
  ).toBeVisible();
  await expect(page.getByText(t("relation-map.only-visible"), { exact: true })).toBeVisible();
});

test("iss_16_pointing_highlights_the_chain_and_moves_nothing", async ({ page }) => {
  await signIn(page, ada);
  await openMap(page, mapped.start);
  // The card is brought into view first: what the test scrolls is not the map moving.
  await cardOf(page, mapped.timesheet).scrollIntoViewIfNeeded();
  const marks = await Promise.all(
    [mapped.start, mapped.notes, mapped.timesheet, mapped.design].map((key) =>
      landmarkOf(cardOf(page, key), key),
    ),
  );
  const pointing = await movementWhile(page, marks, async () => {
    await cardOf(page, mapped.timesheet).hover();
    await expect(cardOf(page, mapped.notes)).toHaveAttribute("data-state", /\bon-chain\b/u);
  });
  expectStill(pointing);
  await expect(cardOf(page, mapped.start)).toHaveAttribute("data-state", /\bon-chain\b/u);
  await expect(cardOf(page, mapped.design)).toHaveAttribute("data-state", /\bfaded\b/u);
  // The pointer leaves: nothing stays highlighted.
  await page.mouse.move(0, 0);
  await expect(cardOf(page, mapped.design)).not.toHaveAttribute("data-state", /\bfaded\b/u);
});

test("iss_16_clicking_shows_the_chain_as_sentences_and_start_from_here_walks_on", async ({
  page,
}) => {
  await signIn(page, ada);
  await openMap(page, mapped.start);
  await cardOf(page, mapped.timesheet).click();
  const panel = chainPanel(page, mapped.timesheet);
  await expect(panel).toContainText(timesheetChain);
  await expect(panel.getByRole("link", { name: t("relation-map.open-issue") })).toHaveAttribute(
    "href",
    issueAddress(mapped.timesheet),
  );
  await panel.getByRole("button", { name: t("relation-map.start-from-here"), exact: true }).click();
  await expect(
    page.getByRole("heading", { name: t("relation-map.title", { key: mapped.timesheet }) }),
  ).toBeVisible();
  await expect(cardOf(page, mapped.timesheet)).toHaveAttribute("data-state", /\bstart\b/u);
  // The new start stands on the left.
  expect(await leftOf(cardOf(page, mapped.timesheet))).toBeLessThan(
    await leftOf(cardOf(page, mapped.notes)),
  );
});

test("iss_16_kinds_shown_and_steps_away_narrow_the_map", async ({ page }) => {
  await signIn(page, ada);
  await openMap(page, mapped.start);
  await expect(cardOf(page, mapped.timesheet)).toBeVisible();
  await page.getByRole("radio", { name: "1", exact: true }).check();
  await expect(cardOf(page, mapped.timesheet)).toHaveCount(0);
  await expect(cardOf(page, mapped.notes)).toBeVisible();
  await page.getByRole("radio", { name: t("relation-map.everything"), exact: true }).check();
  await expect(cardOf(page, mapped.timesheet)).toBeVisible();
  await toggleKind(page, kindWords.blocks);
  await expect(cardOf(page, mapped.notes)).toHaveCount(0);
  await expect(cardOf(page, mapped.related)).toBeVisible();
  await toggleKind(page, kindWords.related);
  await expect(cardOf(page, mapped.related)).toHaveCount(0);
  expect(await cardKeys(page)).toStrictEqual([mapped.start, mapped.reset]);
  await expect(page).toHaveURL(/hide=/u);
});

test("iss_16_the_list_form_holds_the_same_chains_as_the_picture", async ({ page }) => {
  await signIn(page, ada);
  await openMap(page, mapped.start);
  const onPicture = (await cardKeys(page)).toSorted();
  await page.getByRole("radio", { name: t("relation-map.list"), exact: true }).check();
  const list = mapList(page);
  // The row of WEB-5 tells its chain up to it; its ID stands in its own place, as in every row (R9).
  const row = list.getByRole("row").filter({ hasText: timesheetLead });
  await expect(row).toContainText(mapped.timesheet);
  await inOrder(
    onPicture.map(
      (key) => () => expect(list.getByText(key, { exact: true }).first()).toBeVisible(),
    ),
  );
  await expect(
    list.getByRole("heading", { name: t("relation-map.column", { count: 1 }) }),
  ).toBeVisible();
  await expect(
    list.getByRole("heading", { name: t("relation-map.column", { count: 2 }) }),
  ).toBeVisible();
  // Each row but the start's walks on with *Start from here*, as the side does in the picture.
  await row
    .getByRole("button", { name: t("relation-map.start-from-key", { key: mapped.timesheet }) })
    .click();
  await expect(
    page.getByRole("heading", { name: t("relation-map.title", { key: mapped.timesheet }) }),
  ).toBeVisible();
});

test("iss_16_show_as_map_draws_a_search_and_the_relations_between_its_results", async ({
  page,
}) => {
  await signIn(page, ada);
  await page.goto("/search?s=project%20is%20WEB");
  await page
    .getByLabel(t("timeline.show-as"), { exact: true })
    .first()
    .selectOption({ label: t("timeline.show-as-map") });
  const picture = page.getByRole("figure", { name: /./u });
  await expect(picture).toBeVisible();
  await expect(cardOf(page, mapped.start)).toBeVisible();
  await expect(cardOf(page, mapped.timesheet)).toBeVisible();
  // The relations between the results, each with its words; none to the issue of SECRET, which
  // the search does not find.
  await expect(page.locator('[data-part="map-lines"] [data-part="map-line"]')).toHaveCount(
    webRelations,
  );
  await expect(
    page.locator('[data-part="map-label"]').filter({ hasText: kindWords.causes }),
  ).toHaveCount(1);
  await expect(page.locator('[data-part="map-card"][data-issue^="SECRET-"]')).toHaveCount(0);
  // *Kinds shown* stand in the address.
  await page.getByRole("button", { name: kindWords.causes, exact: true }).click();
  await expect(page).toHaveURL(/hide=/u);
  await expect(page.locator('[data-part="map-lines"] [data-part="map-line"]')).toHaveCount(
    webRelations - 1,
  );
  // *Start from here* opens the map of that issue.
  await cardOf(page, mapped.notes).click();
  await page.getByRole("button", { name: t("relation-map.start-from-here"), exact: true }).click();
  await expect(
    page.getByRole("heading", { name: t("relation-map.title", { key: mapped.notes }) }),
  ).toBeVisible();
});

test("prj_06_a_projects_issues_shown_as_a_map_hold_that_project_only", async ({ page }) => {
  await signIn(page, ada);
  await page.goto("/p/WEB/issues?show=map");
  await expect(cardOf(page, mapped.start)).toBeVisible();
  await expect(page.locator('[data-part="map-lines"] [data-part="map-line"]')).toHaveCount(
    webRelations,
  );
  await expect(page.locator('[data-part="map-card"][data-issue^="SECRET-"]')).toHaveCount(0);
});

test("iss_16_the_screen_draws_nothing_the_answer_does_not_hold", async ({ page }) => {
  // Ana sees WEB but not SECRET: the issue between WEB-8 and WEB-9 is hidden, so the map of WEB-8
  // holds WEB-8 alone, with no line, no gap and no word of another issue.
  await signIn(page, ana);
  await openMap(page, mapped.beforeHidden);
  expect(await cardKeys(page)).toStrictEqual([mapped.beforeHidden]);
  await expect(page.locator('[data-part="map-lines"] [data-part="map-line"]')).toHaveCount(0);
  await expect(page.locator('[data-part="map-label"]')).toHaveCount(0);
  await expect(
    page.getByText(t("relation-map.nothing", { key: mapped.beforeHidden })),
  ).toBeVisible();
  await expect(page.getByText(mapped.afterHidden)).toHaveCount(0);
});

test("iss_16_the_keyboard_reaches_every_card_and_start_from_here", async ({ page }) => {
  await signIn(page, ada);
  await openMap(page, mapped.start);
  const keys = await cardKeys(page);
  await cardOf(page, mapped.start).focus();
  // Each card after the start is the next stop of Tab, in the order of the picture.
  await inOrder(
    keys.slice(1).map((key) => async () => {
      await page.keyboard.press("Tab");
      await expect(cardOf(page, key)).toBeFocused();
    }),
  );
  await page.keyboard.press("Enter");
  // From the chosen card, the next stop of Tab is *Start from here* of its chain.
  await page.keyboard.press("Tab");
  await expect(
    page
      .locator('[data-part="map-chain"]')
      .getByRole("button", { name: t("relation-map.start-from-here"), exact: true }),
  ).toBeFocused();
});

test("iss_16_the_start_and_the_chain_keep_the_edge_of_their_priority", async ({ page }) => {
  // WEB-1 is Critical and WEB-4 High: on the start and on a shown chain, their start edges keep the
  // colors of their priorities, the same as the edges of the rows of a list.
  await signIn(page, ada);
  await openMap(page, mapped.start);
  await cardOf(page, mapped.timesheet).click();
  await expect(cardOf(page, mapped.notes)).toHaveAttribute("data-state", /\bon-chain\b/u);
  const startEdge = (key: string) =>
    cardOf(page, key).evaluate((card) => getComputedStyle(card).borderInlineStartColor);
  expect(await startEdge(mapped.start)).toBe(await tokenColor(page, "danger"));
  expect(await startEdge(mapped.notes)).toBe(await tokenColor(page, "edge-high"));
});
