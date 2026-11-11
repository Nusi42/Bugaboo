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
export { agentWorkRead, openHandOver, showsTheAgentWork } from "./agent-work-view.ts";

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

/** One result an agent attached: a link, or a check with its outcome. */
export interface ResultScene {
  readonly kind: "link" | "check";
  readonly label: string;
  readonly link?: string;
  readonly outcome?: "passed" | "failed";
}

/** One attempt of an agent as a test puts it into the data. */
export interface AttemptScene {
  readonly number?: number;
  readonly plan?: string;
  readonly approved?: boolean;
  readonly ended?: boolean;
  readonly minutes?: number;
  readonly cost?: number;
  readonly run?: string;
  readonly results?: readonly ResultScene[];
}

/** What a test puts on the work of an agent. No way into Bugaboo makes an agent take, report and ask before the tools of MCP. */
export interface WorkScene {
  readonly state: Schema<"AgentWorkState">;
  readonly question?: string;
  /** The answer to the question, when it is answered. */
  readonly answer?: string;
  readonly attempt?: AttemptScene;
  /** More attempts, oldest first, that stand before `attempt`. */
  readonly earlier?: readonly AttemptScene[];
  /** A review that was made before: *send-back* with its note, or *accept* by itself. */
  readonly review?: { readonly answer: "send-back" | "accept"; readonly note?: string };
  readonly gaveUp?: string;
  /** Why the work stopped, when it stopped without a review and without giving up. */
  readonly stoppedFor?: Schema<"StopCause">;
  /** The result was accepted by itself and nobody looked at it. */
  readonly acceptedByItself?: boolean;
  /** The issue kept its status after it was accepted by itself, and a person chooses the next one. */
  readonly statusPending?: boolean;
}

/** Writes the rows of a scene into the data of a throwaway server. */
class SceneWriter {
  private readonly database: DatabaseSync;
  private readonly now = Date.now() * MICROSECONDS;

  constructor(server: Instance) {
    this.database = new DatabaseSync(join(server.dataDir, databaseFile));
    this.database.exec("PRAGMA busy_timeout = 30000");
  }

  close(): void {
    this.database.close();
  }

  private run(sql: string, ...values: readonly (string | number | Buffer | null)[]): void {
    this.database.prepare(sql).run(...values);
  }

  state(work: string, scene: WorkScene): void {
    const flag = scene.acceptedByItself === true ? 1 : 0;
    this.run(
      "UPDATE issue_work SET state = ?, gave_up = ?, stopped_for = ?, accepted_by_itself = ?, closed = ?, status_pending = ? WHERE id = ?",
      scene.state,
      scene.gaveUp ?? null,
      scene.stoppedFor ?? null,
      flag,
      flag,
      scene.statusPending === true ? 1 : 0,
      stored(work),
    );
  }

  question(work: string, scene: WorkScene, person?: string): void {
    if (scene.question === undefined) {
      return;
    }
    const answered = scene.answer === undefined || person === undefined ? null : stored(person);
    this.run(
      "INSERT INTO agent_question (id, work, text, answer, answered_by, answered_at, asked_at) VALUES (?, ?, ?, ?, ?, ?, ?)",
      stored(randomUUID()),
      stored(work),
      scene.question,
      scene.answer ?? null,
      answered,
      answered === null ? null : this.now,
      this.now,
    );
  }

  attempt(work: string, attempt: AttemptScene): void {
    const id = randomUUID();
    this.run(
      "INSERT INTO agent_attempt (id, work, number, plan, plan_approved, reported_minutes, reported_cost, started_at, ended_at, run) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
      stored(id),
      stored(work),
      attempt.number ?? 1,
      attempt.plan ?? null,
      attempt.approved === true ? 1 : 0,
      attempt.minutes ?? 0,
      attempt.cost ?? 0,
      this.now,
      attempt.ended === true ? this.now : null,
      attempt.run === undefined ? null : stored(attempt.run),
    );
    for (const result of attempt.results ?? []) {
      this.run(
        "INSERT INTO agent_result (id, attempt, kind, label, link, outcome) VALUES (?, ?, ?, ?, ?, ?)",
        stored(randomUUID()),
        stored(id),
        result.kind,
        result.label,
        result.link ?? null,
        result.outcome ?? null,
      );
    }
  }

  review(work: string, scene: WorkScene, person?: string): void {
    if (scene.review === undefined) {
      return;
    }
    const by = person === undefined || scene.acceptedByItself === true ? null : stored(person);
    this.run(
      'INSERT INTO agent_review (id, work, answer, note, "by", at) VALUES (?, ?, ?, ?, ?, ?)',
      stored(randomUUID()),
      stored(work),
      scene.review.answer,
      scene.review.note ?? null,
      by,
      this.now,
    );
  }
}

/**
 * Sets the state of a piece of work and gives it questions, attempts and a review, in the data of a
 * throwaway server: the agent reports through MCP tools that come later, so a screen that shows
 * what an agent reported needs the rows. `person` is who answered or reviewed.
 *
 * The IDs of these records are random (version 4), not the time-ordered kind of a run. It assumes
 * the tables `issue_work`, `agent_attempt`, `agent_result`, `agent_question` and `agent_review`
 * (`crates/bb-store/src/schema/`).
 */
export function putScene(server: Instance, work: string, scene: WorkScene, person?: string): void {
  const writer = new SceneWriter(server);
  try {
    writer.state(work, scene);
    writer.question(work, scene, person);
    for (const attempt of [...(scene.earlier ?? []), ...(scene.attempt ? [scene.attempt] : [])]) {
      writer.attempt(work, attempt);
    }
    writer.review(work, scene, person);
  } finally {
    writer.close();
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

/** The window that is open (the hand-over, the review). */
export async function openWindow(page: Page): Promise<Locator> {
  const window = page.getByRole("dialog");
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

/** Counts the version of a piece of work up, as if somebody else answered it meanwhile. */
export function changeWorkBehindTheScreen(server: Instance, work: string): void {
  const database = new DatabaseSync(join(server.dataDir, databaseFile));
  try {
    database.exec("PRAGMA busy_timeout = 30000");
    database.prepare("UPDATE issue_work SET version = version + 1 WHERE id = ?").run(stored(work));
  } finally {
    database.close();
  }
}
