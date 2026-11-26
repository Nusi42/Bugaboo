import { dataOf } from "@bugaboo/api-client";
import { t } from "@bugaboo/messages";
import { expect, type Locator, type Page } from "@playwright/test";

import { issueMapAddress } from "../../src/shared/paths.ts";
import { ada, signInThroughApi, type ApiSession, type Instance } from "./accounts.ts";
import { createIssue, prepareWeb } from "./issues.ts";
import { createProject } from "./projects.ts";
import { BLOCKS } from "./relate.ts";
import { inOrder } from "./sequence.ts";
import { changeIssue } from "./timeline.ts";

// The scene of the relation map (ISS-16, UX §4.15): the project WEB of the tests of issues, with
// the map of UX §4.15 around WEB-1 (*Login fails after password reset*), two steps away, and an
// issue of a project the Viewer Ana does not see between two of hers.

/** What the starting kinds of relation read from their first issue, as the server writes them. */
export const kindWords = {
  blocks: BLOCKS,
  causes: "causes",
  related: "related to",
  duplicate: "duplicate of",
} as const;

/** The issues of the scene by what they stand for. WEB-1 to WEB-3 are those of `prepareWeb`. */
export const mapped = {
  start: "WEB-1",
  related: "WEB-2",
  notes: "WEB-4",
  timesheet: "WEB-5",
  design: "WEB-6",
  reset: "WEB-7",
  beforeHidden: "WEB-8",
  afterHidden: "WEB-9",
} as const;

/** The issues the scene makes after WEB-3, in the order of their IDs. */
const titles = [
  "Write the release notes for 2.4",
  "Download the timesheet as a spreadsheet",
  "Design the new sign-in page",
  "Reset e-mail link expires too early",
  "Export the audit log",
  "Archive old exports",
] as const;

/** The ID of a kind of relation by what its first issue reads. */
export async function kindOf(session: ApiSession, forward: string): Promise<string> {
  const kinds = dataOf(await session.client.GET("/api/v1/relation-kinds"));
  const found = kinds.items.find((kind) => kind.forward === forward)?.id;
  if (found === undefined || found === null) {
    throw new TypeError(`The server has no kind ${forward}.`);
  }
  return found;
}

/** Makes `from` read `forward` of `to`: `WEB-1 blocks WEB-4`. */
export async function relate(
  session: ApiSession,
  from: string,
  forward: string,
  to: string,
): Promise<void> {
  dataOf(
    await session.client.POST("/api/v1/issues/{issue}/relations", {
      params: { path: { issue: from } },
      body: { other: to, relation_kind: await kindOf(session, forward), direction: "forward" },
    }),
  );
}

/**
 * Gets the server ready (WEB-1 Critical, WEB-4 High): WEB-1 blocks WEB-4, which is blocked by WEB-5; WEB-1 is related to WEB-2,
 * which is blocked by WEB-6; WEB-7 causes WEB-1; and WEB-8 blocks SECRET-1, which blocks WEB-9.
 */
export async function prepareRelationMap(server: Instance): Promise<void> {
  await prepareWeb(server);
  const admin = await signInThroughApi(server, ada);
  await inOrder(titles.map((title) => () => createIssue(admin, "WEB", title)));
  await createProject(admin, "SECRET", "Secret");
  const hidden = await createIssue(admin, "SECRET", "Hidden between two issues");
  await relate(admin, mapped.start, kindWords.blocks, mapped.notes);
  await relate(admin, mapped.timesheet, kindWords.blocks, mapped.notes);
  await relate(admin, mapped.start, kindWords.related, mapped.related);
  await relate(admin, mapped.design, kindWords.blocks, mapped.related);
  await relate(admin, mapped.reset, kindWords.causes, mapped.start);
  await relate(admin, mapped.beforeHidden, kindWords.blocks, hidden.key);
  await relate(admin, hidden.key, kindWords.blocks, mapped.afterHidden);
  // Priorities that draw an edge, on the start and on a chain.
  await changeIssue(admin, mapped.start, { priority: "critical" });
  await changeIssue(admin, mapped.notes, { priority: "high" });
  await admin.dispose();
}

/** Opens the relation map of an issue and waits for its picture or its list. */
export async function openMap(page: Page, key: string, query = ""): Promise<void> {
  await page.goto(`${issueMapAddress(key)}${query}`);
  await expect(
    page.getByRole("heading", { name: t("relation-map.title", { key }), level: 1 }),
  ).toBeVisible();
}

/** The card of an issue on the picture. */
export function cardOf(page: Page, key: string): Locator {
  return page.locator(`[data-part="map-card"][data-issue="${key}"]`);
}

/** Every card on the picture, by its Issue ID, in the order of the page. */
export function cardKeys(page: Page): Promise<string[]> {
  return page
    .locator('[data-part="map-card"]')
    .evaluateAll((all: readonly HTMLElement[]) => all.map((card) => card.dataset["issue"] ?? ""));
}

/** The side panel with the chain of the chosen issue. */
export function chainPanel(page: Page, key: string): Locator {
  return page.getByRole("region", { name: t("relation-map.chain", { key }) });
}

/** The list form of the map. */
export function mapList(page: Page): Locator {
  return page.getByRole("region", { name: t("relation-map.list-label") });
}
