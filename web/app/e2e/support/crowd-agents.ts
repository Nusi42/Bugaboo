import { t } from "@bugaboo/messages";
import { type Browser, type Page, expect } from "@playwright/test";

import { type Agents } from "../../../tooling/load/agents-run.ts";
import { type ApiSession } from "./api.ts";
import { type Instance } from "./server.ts";
import { issueAddress } from "../../src/shared/paths.ts";
import { type ScreenLook } from "./crowd-pages.ts";

// Ten people and their agents in ten browsers (architecture §11, M7 WP-13): the pages of the people
// and what each of them sees when the agents are done.

/** The administrator of a run, which is the first session. */
export function administratorOf(sessions: readonly ApiSession[]): ApiSession {
  const first = sessions[0];
  if (first === undefined) {
    throw new TypeError("A run needs an administrator.");
  }
  return first;
}

/** A page of a browser of its own for a session, open on an issue. */
async function pageOf(
  browser: Browser,
  server: Instance,
  session: ApiSession | undefined,
  look: ScreenLook,
  issue: string,
): Promise<Page> {
  if (session === undefined) {
    throw new TypeError("A person of the run has no session.");
  }
  const context = await browser.newContext({
    baseURL: server.url,
    ignoreHTTPSErrors: true,
    storageState: await session.state(),
    hasTouch: look.hasTouch,
    ...(look.viewport === null ? {} : { viewport: look.viewport }),
  });
  const page = await context.newPage();
  await page.goto(issueAddress(issue));
  return page;
}

/** Each person with an agent has a browser open on the issue they handed over. */
export function openIssuesOfCrews(
  browser: Browser,
  server: Instance,
  look: ScreenLook,
  sessions: readonly ApiSession[],
  agents: Agents,
): Promise<readonly Page[]> {
  return Promise.all(
    agents.crews.map((crew, at) => pageOf(browser, server, sessions[at + 1], look, crew.issue.key)),
  );
}

/** Every person sees the block of agent work and the mark of the short comment their agent wrote. */
export async function expectTheWorkOfTheAgents(pages: readonly Page[]): Promise<void> {
  await Promise.all(
    pages.map(async (page) => {
      await page.reload();
      await expect(
        page.getByRole("heading", { level: 2, name: t("agent-work.title"), exact: true }),
      ).toBeVisible();
      await expect(page.getByText(/^Written by an agent/u).first()).toBeVisible();
    }),
  );
}

/** The administrator sees that the work of the person who lost access stopped. */
export async function expectTheWorkToHaveStopped(
  browser: Browser,
  server: Instance,
  look: ScreenLook,
  admin: ApiSession,
  agents: Agents,
): Promise<Page> {
  const page = await pageOf(browser, server, admin, look, agents.lost.issue.key);
  await expect(page.getByText(t("agent-work.state.stopped")).first()).toBeVisible();
  return page;
}
