import { expect, type Locator, type Page } from "@playwright/test";

import { createAgent } from "./agent-work.ts";
import { signInThroughApi } from "./accounts.ts";
import { partIn } from "./parts.ts";
import { sampleAdministrator } from "./sample.ts";
import { type Instance } from "./server.ts";
import { type Towards } from "./swipe.ts";
import { paths, projectIssuesAddress } from "../../src/shared/paths.ts";

/** The agent the sample's administrator makes for the rows of *My agents*. */
const AGENT = "Swiper";

/** How many agents were made, from a start of this run's own: each has a name of its own. */
let agentsMade = Date.now() % 100_000;

/** The first row of a part on a page, once it is drawn. */
export async function firstRow(page: Page, address: string, row: string): Promise<Locator> {
  await page.goto(address);
  const first = partIn(page, row).first();
  await expect(first).toBeVisible();
  return first;
}

/** A list of a phone with swipe actions (UX §3.3): its first row, and the ways a finger opens its sides. */
export interface SwipedList {
  readonly name: string;
  readonly open: (page: Page) => Promise<Locator>;
  /**
   * The swipes that bring in its actions, in the order *More* lists them: towards the end edge brings in the start's
   * actions, towards the start edge the end's.
   */
  readonly swipes: readonly Towards[];
}

/** The lists whose rows are swiped: the issue list. */
export const swipedLists: readonly SwipedList[] = [
  {
    name: "the issue list",
    open: (page) => firstRow(page, projectIssuesAddress("WEB"), "list-row"),
    swipes: ["end", "start"],
  },
];

/**
 * The lists whose rows show every action they have as a button (*Mark as read*, *Pause*): a swipe would only repeat
 * one, so they have none (UX §3.3), and no *More* of a row either. `swipes` are the ways a finger tries.
 */
export const unswipedLists: readonly SwipedList[] = [
  {
    name: "notifications",
    open: async (page) => {
      await firstRow(page, paths.notifications, "notification");
      return page.locator("[data-part='notification'][data-state~='unread']").first();
    },
    swipes: ["start"],
  },
  {
    name: "My agents",
    open: (page) => firstRow(page, paths.myAgents, "agents.agent"),
    swipes: ["start"],
  },
];

/** Makes an agent of the sample's administrator, so *My agents* has a row. */
export async function prepareAgent(server: Instance): Promise<void> {
  const admin = await signInThroughApi(server, sampleAdministrator());
  agentsMade += 1;
  await createAgent(admin, `${AGENT} ${String(agentsMade)}`);
  await admin.dispose();
}
