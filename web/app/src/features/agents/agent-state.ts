import { t, type MessageKey } from "@bugaboo/messages";

import { agentStateText as workStateText } from "../../shared/agent-state.ts";
import type { AgentStateWord, LimitWord, ListedAgent, MyWorkItem } from "./agents.ts";

// The state of an agent on every screen: the pill, the one order of agents, and the limit that paused one.

/** The look of the pill of agent work that shows the state of an agent. */
export type AgentPillVariant = "agent-working" | "paused" | "stopped";

/** The look of the pill for each state of an agent. */
const PILL = {
  active: "agent-working",
  paused: "paused",
  stopped: "stopped",
} as const satisfies Record<AgentStateWord, AgentPillVariant>;

/**
 * Where an agent stands, as the pill of agent work shows it (UX §4.17): *Agent working* and
 * *Stopped* are the words of agent work, *Paused* is the agent's own.
 */
export function agentPill(state: AgentStateWord): {
  readonly variant: AgentPillVariant;
  readonly text: string;
} {
  const variant = PILL[state];
  return {
    variant,
    text: variant === "paused" ? t("agents.state.paused") : workStateText(variant),
  };
}

/** Where a state stands in the order of agents: working first, then paused, then stopped. */
const STATE_RANK = { active: 0, paused: 1, stopped: 2 } as const satisfies Record<
  AgentStateWord,
  number
>;

/**
 * The one order of agents, on every screen and in the question of *Stop all my agents*: the
 * agents something waits for and the working ones first, then the paused ones, then the stopped
 * ones; by name inside each group. `waiting` holds the agents whose work waits for their person.
 */
export function agentsInOrder<Agent extends Readonly<Pick<ListedAgent, "id" | "name" | "state">>>(
  agents: readonly Agent[],
  waiting: readonly string[] = [],
): readonly Agent[] {
  const rank = (agent: Readonly<Pick<ListedAgent, "id" | "state">>): number =>
    waiting.includes(agent.id) && agent.state !== "stopped" ? 0 : STATE_RANK[agent.state];
  return agents.toSorted(
    (one, other) =>
      rank(one) - rank(other) ||
      one.name.localeCompare(other.name, undefined, { sensitivity: "base" }) ||
      one.id.localeCompare(other.id),
  );
}

/** The agents whose work waits for their person: a question, a plan or a result to review. */
export function waitingAgentsOf(work: readonly MyWorkItem[]): readonly string[] {
  return work
    .filter((item) => item.waits_for !== undefined && item.waits_for !== null)
    .map((item) => item.work.agent);
}

/** The words of the limits that pause an agent, with the number of each in the agent. */
const PAUSED_FOR = {
  "changes-a-minute": ["agent-paused.changes-a-minute", "changesAMinute"],
  "issues-at-once": ["agent-paused.issues-at-once", "issuesAtOnce"],
  "attempts-on-one-issue": ["agent-paused.attempts-on-one-issue", "attemptsOnOneIssue"],
} as const satisfies Record<LimitWord, readonly [MessageKey, keyof ListedAgent]>;

/** Which limit paused the agent: *Fixer paused: 60 changes in a minute*; none unless a limit paused it. */
export function pausedSentence(agent: ListedAgent): string | undefined {
  if (agent.state !== "paused" || agent.pausedFor === undefined) {
    return undefined;
  }
  const [key, limit] = PAUSED_FOR[agent.pausedFor];
  return t(key, { agent: agent.name, limit: agent[limit] });
}
