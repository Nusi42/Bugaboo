import { timed } from "../../app/e2e/support/perf.ts";
import { tool } from "../load/agents.ts";
import { type Context, type Scenario } from "./scenarios.ts";

// The scenarios of agent work on the large dataset (M7 WP-13, PERF-01), registered in the list of
// `scenarios.ts`, which spreads this one. Their data is made by the large dataset itself: 1 % of
// the issues are handed over to agents, and the standing rules have their match record.

const RUNS = 200;

/** The agent work that waits for its agent: the biggest share of what is handed over. */
const WAITING_FOR_THE_AGENT = 'agent state is "waiting-for-the-agent"';

/**
 * What an agent asks for when it comes back after a long time: what changed since the oldest mark
 * (*changes since*, AI-36). One request reads a bounded part of the log and the match record of the
 * rules of the agent, so the time does not grow with the log.
 */
async function agentChangesSince(context: Context): Promise<number> {
  const asked = await timed(() =>
    tool(context.agent.server, context.agent.key, "changes_since", { since: "0.0" }),
  );
  return asked.ms;
}

/** The first page of a search for the agent work that waits for its agent (AI-37). */
async function searchAgentState(context: Context): Promise<number> {
  return (
    await timed(() =>
      context.session.client.GET("/api/v1/search/issues", {
        params: { query: { q: WAITING_FOR_THE_AGENT, limit: 40 } },
      }),
    )
  ).ms;
}

/** *My agents* of the person who has the agents: what waits for them and the work in progress (AI-37). */
async function myAgents(context: Context): Promise<number> {
  return (await timed(() => context.agent.person.client.GET("/api/v1/me/agents/work"))).ms;
}

export const agentScenarios: readonly Scenario[] = [
  {
    // What an agent that comes back asks first (AI-36): within the budget of a page.
    name: "agent-changes-since",
    budget: "page",
    runs: RUNS,
    once: agentChangesSince,
  },
  {
    // The search of the states of agent work, on the 1 % of the issues that agents hold.
    name: "search-agent-state",
    budget: "search",
    runs: RUNS,
    once: searchAgentState,
  },
  {
    // The answer of *My agents* for a person with three agents and many hand-overs.
    name: "my-agents",
    budget: "page",
    runs: RUNS,
    once: myAgents,
  },
];
