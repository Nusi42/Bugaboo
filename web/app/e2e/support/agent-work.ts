import { randomUUID } from "node:crypto";
import { join } from "node:path";
import { DatabaseSync } from "node:sqlite";

import { dataOf, type Schema } from "@bugaboo/api-client";
import { t } from "@bugaboo/messages";
import { expect, type Locator, type Page } from "@playwright/test";

import type { ApiSession } from "./api.ts";
import { stored } from "./runs.ts";
import type { Instance } from "./server.ts";

/** The file the program keeps its data in. */
const databaseFile = "bugaboo.db";
const MICROSECONDS = 1000;

export { tickTwo } from "./labels.ts";

/** Makes an agent of the person who is signed in, through the API; its ID. */
export async function createAgent(admin: ApiSession, name: string): Promise<string> {
  const made = dataOf(await admin.client.POST("/api/v1/agents", { body: { name } }));
  return made.id ?? "";
}

/** What a hand-over of the tests asks besides the issues and the agent. */
export interface HandOverAsk {
  readonly note?: string;
  readonly planNeedsApproval?: boolean;
}

/** Hands issues over to the agent, through the API: the work on each issue, by issue. */
export async function handOver(
  admin: ApiSession,
  agent: string,
  issues: readonly string[],
  ask: HandOverAsk = {},
): Promise<Readonly<Record<string, string>>> {
  const made = dataOf(
    await admin.client.POST("/api/v1/hand-overs", {
      body: {
        agent,
        issues: [...issues],
        note: ask.note ?? "Please look at the login button",
        plan_needs_approval: ask.planNeedsApproval ?? false,
      },
    }),
  );
  const handed: readonly Schema<"HandedIssueBody">[] = made.issues;
  return Object.fromEntries(handed.map((each) => [each.issue, each.work]));
}

/** The agent work of an issue as the API gives it. */
export async function agentWorkOf(
  admin: ApiSession,
  issue: string,
): Promise<Schema<"IssueAgentWorkBody">> {
  return dataOf(
    await admin.client.GET("/api/v1/issues/{issue}/agent-work", { params: { path: { issue } } }),
  );
}

/** What a test puts on the work of an agent. No way into Bugaboo makes an agent take, report and ask before the tools of MCP. */
export interface WorkScene {
  readonly state: Schema<"AgentWorkState">;
  readonly question?: string;
  readonly attempt?: {
    readonly plan?: string;
    readonly minutes?: number;
    readonly cost?: number;
    readonly run?: string;
    readonly results?: readonly {
      readonly kind: "link" | "check";
      readonly label: string;
      readonly link?: string;
      readonly outcome?: "passed" | "failed";
    }[];
  };
}

/**
 * Sets the state of a piece of work and gives it a question and an attempt, in the data of a
 * throwaway server: the agent reports through MCP tools that come later, so a screen that shows
 * what an agent reported needs the rows.
 *
 * The IDs of these records are random (version 4), not the time-ordered kind of a run. It assumes the tables `issue_work`, `agent_attempt`, `agent_result` and `agent_question`
 * (`crates/bb-store/src/schema/`).
 */
export function putScene(server: Instance, work: string, scene: WorkScene): void {
  const database = new DatabaseSync(join(server.dataDir, databaseFile));
  try {
    database.exec("PRAGMA busy_timeout = 30000");
    const now = Date.now() * MICROSECONDS;
    database.prepare("UPDATE issue_work SET state = ? WHERE id = ?").run(scene.state, stored(work));
    if (scene.question !== undefined) {
      database
        .prepare(
          "INSERT INTO agent_question (id, work, text, answer, answered_by, asked_at) VALUES (?, ?, ?, NULL, NULL, ?)",
        )
        .run(stored(randomUUID()), stored(work), scene.question, now);
    }
    if (scene.attempt !== undefined) {
      const attempt = randomUUID();
      database
        .prepare(
          "INSERT INTO agent_attempt (id, work, number, plan, plan_approved, reported_minutes, reported_cost, started_at, ended_at, run) VALUES (?, ?, 1, ?, 0, ?, ?, ?, NULL, ?)",
        )
        .run(
          stored(attempt),
          stored(work),
          scene.attempt.plan ?? null,
          scene.attempt.minutes ?? 0,
          scene.attempt.cost ?? 0,
          now,
          scene.attempt.run === undefined ? null : stored(scene.attempt.run),
        );
      for (const result of scene.attempt.results ?? []) {
        database
          .prepare(
            "INSERT INTO agent_result (id, attempt, kind, label, link, outcome) VALUES (?, ?, ?, ?, ?, ?)",
          )
          .run(
            stored(randomUUID()),
            stored(attempt),
            result.kind,
            result.label,
            result.link ?? null,
            result.outcome ?? null,
          );
      }
    }
  } finally {
    database.close();
  }
}

/** Hands one issue over and gives the ID of the work on it. */
export async function handOverOne(
  admin: ApiSession,
  agent: string,
  issue: { readonly key: string; readonly id: string },
  ask: HandOverAsk = {},
): Promise<string> {
  const works = await handOver(admin, agent, [issue.key], ask);
  return works[issue.id] ?? "";
}

/** The first piece of work on an issue. */
export async function firstWorkOf(admin: ApiSession, issue: string) {
  const all = await agentWorkOf(admin, issue);
  const first = all.work[0];
  if (first === undefined) {
    throw new Error(`${issue} has no agent work.`);
  }
  return first;
}

/** Where the top of an element is on the page. */
export async function topOf(element: Locator): Promise<number> {
  const box = await element.boundingBox();
  return box === null ? Number.NaN : box.y;
}

/** Opens *More* of the issue page and chooses *Hand over to an agent*; the window it opens. */
export async function openHandOver(page: Page): Promise<Locator> {
  await page.getByRole("button", { name: t("issues.more"), exact: true }).click();
  await page.getByRole("button", { name: t("agent-work.hand-over"), exact: true }).click();
  const window = page.getByRole("alertdialog");
  await expect(window).toBeVisible();
  return window;
}

/** The window that is open (the hand-over, the review). */
export async function openWindow(page: Page): Promise<Locator> {
  const window = page.getByRole("alertdialog");
  await expect(window).toBeVisible();
  return window;
}

/** The piece of work of the agent of this name on an issue. */
export async function workByAgent(admin: ApiSession, issue: string, name: string) {
  const all = await agentWorkOf(admin, issue);
  const found = all.work.find((each) => each.agent_name === name);
  if (found === undefined) {
    throw new Error(`${name} has no work on ${issue}.`);
  }
  return found;
}

/** Opens the Details where they are behind a button (a phone); they stand open on a computer. */
export async function openDetails(page: Page): Promise<void> {
  await expect(page.getByRole("heading", { level: 1 })).toBeVisible();
  const toggle = page.getByRole("button", { name: t("issues.all-details"), exact: true });
  if ((await toggle.count()) > 0) {
    await toggle.click();
  }
}

export { readyForReview } from "./agent-work-ready.ts";
