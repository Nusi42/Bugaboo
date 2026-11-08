import { t } from "@bugaboo/messages";
import { expect, type Page } from "@playwright/test";

import { issueAddress } from "../../src/shared/paths.ts";
import { ada, ownId } from "./accounts.ts";
import { openDetails, type ResultScene } from "./agent-work.ts";
import { readyForReview, sceneIssue } from "./agent-work-ready.ts";
import { signIn } from "./flows.ts";
import { issueOf, readIssue, setStatusAndPriority, versionOf } from "./issues.ts";
import { partIn } from "./parts.ts";
import type { Instance } from "./server.ts";

/** What a plain list of results looks like: a link, a check that passed and one that failed. */
export const RESULTS: readonly ResultScene[] = [
  { kind: "link", label: "Pull request 91", link: "https://example.test/pull/91" },
  { kind: "check", label: "Tests", outcome: "passed" },
  { kind: "check", label: "Lint", outcome: "failed" },
];

const ASKED = "Should a reset link from before the fix still work for 24 hours, or stop at once?";
const PLAN = "1. Find the check\n2. Cut the lifetime of a reset link\n3. Add a test";

/** Scrolls the body of the open window to its top or its end. */
export async function scrollWindow(page: Page, to: "top" | "end"): Promise<void> {
  await partIn(page, "window.body")
    .first()
    .evaluate((body, where) => {
      body.scrollTop = where === "top" ? 0 : body.scrollHeight;
    }, to);
}

/** Signs in and opens the issue, and waits for its block of agent work. */
export async function openBlock(page: Page, key: string): Promise<void> {
  await signIn(page, ada);
  await goToBlock(page, key);
}

/**
 * Opens another issue and waits for its block, with the Details open where they are behind a button
 * (a phone), so every picture of the block shows them the same way.
 */
export async function goToBlock(page: Page, key: string): Promise<void> {
  await page.goto(issueAddress(key));
  await expect(page.getByRole("heading", { level: 2, name: t("agent-work.title") })).toBeVisible();
  await openDetails(page);
}

/** An issue its person has, with an agent that works on it. */
export async function assignedAndWorkedOn(server: Instance): Promise<string> {
  const { admin, issue } = await sceneIssue(server, "Assigned and worked on", "Sorter", {
    state: "agent-working",
    attempt: { plan: "1. Read the check\n2. Fix it", approved: true },
  });
  const version = (await readIssue(admin, issue)).issue.version ?? 1;
  await admin.client.PATCH("/api/v1/issues/{issue}", {
    params: { path: { issue: issue.key } },
    body: { version, assigned_to: [await ownId(admin)] },
  });
  return issue.key;
}

/** The keys of the issues of the states of the block, by name. */
export async function blockIssues(
  server: Instance,
  who: string,
): Promise<Readonly<Record<string, string>>> {
  const key = async (title: string, name: string, scene: Parameters<typeof sceneIssue>[3]) =>
    (await sceneIssue(server, `${title} ${who}`, `${name} ${who}`, scene)).issue.key;
  const attempt = { plan: PLAN, minutes: 25, cost: 40 };
  return {
    waiting: await key("Waiting for the agent", "Waiter", { state: "waiting-for-the-agent" }),
    working: await key("Agent working", "Worker", {
      state: "agent-working",
      attempt: { ...attempt, approved: true },
    }),
    asking: await key("A question waits", "Asker", {
      state: "waiting-for-you",
      question: ASKED,
      attempt,
    }),
    answered: await key("A question answered", "Answerer", {
      state: "agent-working",
      question: ASKED,
      answer: "Stop at once.",
      attempt: { ...attempt, approved: true },
    }),
    gaveUp: await key("The agent gave up", "Quitter", {
      state: "agent-gave-up",
      gaveUp: "The tests need a database the agent cannot reach.",
      attempt: { ...attempt, ended: true },
    }),
    stopped: await key("The agent was stopped", "Stopper", {
      state: "stopped",
      attempt: { ...attempt, ended: true },
    }),
    itself: await acceptedByItself(server, `Accepted by itself ${who}`, `Quick ${who}`, true),
    itselfKept: await acceptedByItself(
      server,
      `Accepted by itself, status kept ${who}`,
      `Slow ${who}`,
      false,
    ),
  };
}

/**
 * An issue whose result was accepted by itself: the issue moved to the status the workflow proposes
 * (*Done*), or kept its status because there is no one to move to and a person chooses.
 */
async function acceptedByItself(
  server: Instance,
  title: string,
  name: string,
  moved: boolean,
): Promise<string> {
  const attempt = { plan: PLAN, minutes: 25, cost: 40 };
  const { admin, issue } = await sceneIssue(server, title, name, {
    state: "ready-for-review",
    acceptedByItself: true,
    statusPending: !moved,
    review: { answer: "accept" },
    attempt: {
      ...attempt,
      ended: true,
      results: [RESULTS[0], RESULTS[1]].flatMap((each) => (each === undefined ? [] : [each])),
    },
  });
  if (moved) {
    const made = await issueOf(admin, issue.key);
    await setStatusAndPriority(admin, made, await versionOf(admin, made), {
      status: "done",
      priority: "normal",
    });
  }
  return issue.key;
}

/** An issue whose plan waits for *Approve plan*. */
export async function planWaiting(server: Instance): Promise<string> {
  const { issue } = await sceneIssue(
    server,
    "A plan to approve",
    "Planner",
    { state: "waiting-for-you", attempt: { plan: PLAN, minutes: 25, cost: 40 } },
    { planNeedsApproval: true },
  );
  return issue.key;
}

/** An issue whose first attempt was sent back and whose second waits for the review. */
export async function sentBack(server: Instance): Promise<string> {
  const { issue } = await readyForReview(server, "Sent back and tried again", "Retrier", true, {
    earlier: [
      { number: 1, plan: "1. Fix the check", ended: true, minutes: 25, cost: 40, results: RESULTS },
    ],
    review: { answer: "send-back", note: "Also cover the passkey" },
  });
  return issue.key;
}

/** The key of the issue of a named state of the block. */
export function keyOf(issues: Readonly<Record<string, string>>, name: string): string {
  const key = issues[name];
  if (key === undefined) {
    throw new Error(`There is no issue for ${name}.`);
  }
  return key;
}
