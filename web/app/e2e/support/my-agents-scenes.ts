import { join } from "node:path";
import { DatabaseSync } from "node:sqlite";

import type { Schema } from "@bugaboo/api-client";
import type { Locator } from "@playwright/test";

import { ownId } from "./accounts.ts";
import { handOverOne, putScene, type WorkScene } from "./agent-work.ts";
import { type AgentsScene } from "./agents-scene.ts";
import { createIssue } from "./issues.ts";
import { stored } from "./runs.ts";
import type { Instance } from "./server.ts";

/** The file the program keeps its data in. */
const databaseFile = "bugaboo.db";

/** The plan of the piece of work that waits for *Approve plan*. */
const PLAN = "1. Find the check\n2. Cut the lifetime of a reset link\n3. Add a test";

/** An attempt that ended with a result: a link and a check that passed. */
const ENDED = {
  plan: PLAN,
  minutes: 25,
  cost: 40,
  ended: true,
  results: [
    { kind: "link", label: "Pull request 91", link: "https://example.test/pull/91" },
    { kind: "check", label: "Tests", outcome: "passed" },
  ],
} as const;

/** What waits for the person on *My agents*, and what was accepted by itself, by the title of its issue. */
const WAITING: readonly { readonly title: string; readonly scene: WorkScene }[] = [
  {
    title: "Keep the old reset links?",
    scene: { state: "waiting-for-you", question: "Should the old reset links keep working?" },
  },
  {
    title: "A plan to approve",
    scene: { state: "waiting-for-you", attempt: { plan: PLAN, minutes: 25, cost: 40 } },
  },
  { title: "A result to review", scene: { state: "ready-for-review", attempt: ENDED } },
  {
    title: "Accepted when its checks passed",
    scene: {
      state: "ready-for-review",
      acceptedByItself: true,
      review: { answer: "accept" },
      attempt: ENDED,
    },
  },
];

/**
 * Gives Fixer of the scene a question, a plan to approve, a result to review and a result that was
 * accepted by itself, each on an issue of its own, so *Waiting for you* and *Accepted by itself*
 * show every kind of row. The agent reports through MCP tools, so the rows go into the data.
 */
export async function giveWaitingWork(server: Instance, scene: AgentsScene): Promise<void> {
  const person = await ownId(scene.admin);
  await Promise.all(
    WAITING.map(async ({ title, scene: work }) => {
      const issue = await createIssue(scene.admin, "WEB", title, "A description");
      const id = await handOverOne(scene.admin, scene.fixer, issue, {
        planNeedsApproval: work.attempt?.plan !== undefined && work.state === "waiting-for-you",
      });
      putScene(server, id, work, person);
    }),
  );
}

/**
 * Pauses an agent as a limit does, in the data of a throwaway server: the limits are reached by an
 * agent's own requests through MCP, which a browser test does not make. It assumes the columns
 * `state` and `paused_for` of the table `agent` (`crates/bb-store/src/schema/`).
 */
export function pauseByLimit(server: Instance, agent: string, limit: Schema<"LimitKind">): void {
  const database = new DatabaseSync(join(server.dataDir, databaseFile));
  try {
    database.exec("PRAGMA busy_timeout = 30000");
    database
      .prepare("UPDATE agent SET state = 'paused', paused_for = ? WHERE id = ?")
      .run(limit, stored(agent));
  } finally {
    database.close();
  }
}

/** Scrolls everything in the window that scrolls to its end, so its last fields are in the picture. */
export async function scrollToEnd(window: Locator): Promise<void> {
  await window.evaluate((root) => {
    for (const box of [root, ...root.querySelectorAll("*")]) {
      if (box.scrollHeight > box.clientHeight) {
        box.scrollTop = box.scrollHeight;
      }
    }
  });
}
