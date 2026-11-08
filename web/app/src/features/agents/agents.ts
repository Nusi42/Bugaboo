import { agentRuleWarningDays, type Schema } from "@bugaboo/api-client";
import { t, type MessageKey } from "@bugaboo/messages";
import { useQueries } from "@tanstack/react-query";

import { agentWorkKeys } from "../../shared/agent-work-keys.ts";
import { api } from "../../shared/api.ts";
import { daysUntil, formatDate } from "../../shared/format.ts";
import { personName } from "../../shared/person.ts";
import { pageSize } from "../../shared/paged.ts";
import { queryClient } from "../../shared/query-client.ts";
import { refreshIssues, refreshRoutes } from "../../shared/query-keys.ts";

/** An agent as the list of agents shows it: every field can be left out of an answer. */
export type AgentEntry = Schema<"AgentItem">;
/** An agent with its keys, as its own page shows it. */
export type AgentRecord = Schema<"AgentBody">;
/** One agent key, as the page of an agent lists it. */
export type AgentKey = Schema<"AccessKeyBody">;
/** A piece of work of one of the person's agents, with its issue and what it waits for. */
export type MyWorkItem = Schema<"MyWorkBody">;
/** An agent another person handed to this person, who has not accepted it yet. */
export type OfferedAgent = Schema<"OfferedAgentBody">;
/** The work of the person's agents. */
export type MyWork = Schema<"MyAgentWorkBody">;
/** A standing rule. */
export type Rule = Schema<"StandingRuleItem">;
/** The state of an agent: it works, rests, or is stopped for good. */
export type AgentStateWord = Schema<"AgentState">;
/** What an action on an agent does. */
export type AgentActionWord = "pause" | "resume" | "stop";

/** An agent whose list entry holds everything the screens need. */
export interface ListedAgent {
  readonly id: string;
  readonly name: string;
  readonly state: AgentStateWord;
  readonly version: number;
  readonly offered: boolean;
  readonly person: AgentEntry["person"];
  readonly changesAMinute: number;
  readonly issuesAtOnce: number;
  readonly attemptsOnOneIssue: number;
}

/** The agents of a list that came with all they need (a field can be missing in an answer). */
export function listedAgents(items: readonly AgentEntry[] | undefined): readonly ListedAgent[] {
  return (items ?? []).flatMap((item) =>
    item.id === undefined ||
    item.id === null ||
    item.name === undefined ||
    item.name === null ||
    item.state === undefined ||
    item.state === null ||
    item.version === undefined ||
    item.version === null
      ? []
      : [
          {
            id: item.id,
            name: item.name,
            state: item.state,
            version: item.version,
            offered: item.offered === true,
            person: item.person,
            changesAMinute: item.changes_a_minute ?? 0,
            issuesAtOnce: item.issues_at_once ?? 0,
            attemptsOnOneIssue: item.attempts_on_one_issue ?? 0,
          },
        ],
  );
}

/** The agents of the person (at most fifty, so one page holds them all). */
export function useMyAgentList() {
  return api.query.useQuery("get", "/api/v1/agents", {
    params: { query: { limit: pageSize.largest } },
  });
}

/** The work of the person's agents: what waits for them, what is in progress, what was accepted by itself. */
export function useMyAgentWork() {
  return api.query.useQuery("get", "/api/v1/me/agents/work");
}

/** One agent with its keys. */
export function useAgentRecord(id: string) {
  return api.query.useQuery("get", "/api/v1/agents/{agent}", {
    params: { path: { agent: id } },
  });
}

/** The standing rules of one agent. */
export function agentRulesOptions(agent: string) {
  return api.query.queryOptions("get", "/api/v1/agents/{agent}/standing-rules", {
    params: { path: { agent }, query: { limit: pageSize.largest } },
  });
}

/** The standing rules of several agents, each agent's own list. */
export function useRulesOfAgents(agents: readonly string[]) {
  const lists = useQueries({ queries: agents.map((agent) => agentRulesOptions(agent)) });
  const rules: Rule[] = [];
  let failed: unknown;
  for (const list of lists) {
    rules.push(...(list.data?.items ?? []));
    failed ??= list.error;
  }
  return { failed, rules };
}

/** The projects the person can name, flat: the list access keys use, so the answer is read once for both. */
export function useProjectChoices() {
  return api.query.useQuery("get", "/api/v1/access-keys/project-choices", {
    params: { query: { limit: pageSize.largest } },
  });
}

/** Every agent of the organization, for administrators. */
export function useOrganizationAgents() {
  return api.query.useQuery("get", "/api/v1/organization/agents", {
    params: {
      query: { limit: pageSize.largest, fields: "id,name,state,person,last_used_at,version" },
    },
  });
}

/** Agents, their work or their rules changed: every list that shows them is read again. */
export function refreshAgents(): void {
  refreshRoutes([
    "/api/v1/agents",
    "/api/v1/me/agents",
    "/api/v1/organization/agents",
    "/api/v1/standing-rules",
    "/api/v1/issues/{issue}/agent-work",
  ]);
  void queryClient.invalidateQueries({ queryKey: agentWorkKeys.myAgents() });
  refreshIssues();
}

/** Pause, resume or stop an agent, or give it another name. */
export function useChangeAgent() {
  return api.query.useMutation("patch", "/api/v1/agents/{agent}", { onSuccess: refreshAgents });
}

/** *Stop all my agents*. */
export function useStopAll() {
  return api.query.useMutation("post", "/api/v1/me/agents/stop-all", { onSuccess: refreshAgents });
}

/** *Create an agent*. */
export function useCreateAgent() {
  return api.query.useMutation("post", "/api/v1/agents", { onSuccess: refreshAgents });
}

/** Hand an agent to another person, who has to accept it. */
export function useHandAgent() {
  return api.query.useMutation("post", "/api/v1/agents/{agent}/hand-over", {
    onSuccess: refreshAgents,
  });
}

/** Accept an agent that was handed to the person. */
export function useAcceptAgent() {
  return api.query.useMutation("post", "/api/v1/agents/{agent}/accept", {
    onSuccess: refreshAgents,
  });
}

/** Take an agent key back. */
export function useTakeKeyBack() {
  return api.query.useMutation("delete", "/api/v1/agents/{agent}/keys/{access_key}", {
    onSuccess: refreshAgents,
  });
}

/** Make a standing rule. */
export function useCreateRule() {
  return api.query.useMutation("post", "/api/v1/standing-rules", { onSuccess: refreshAgents });
}

/** Renew a standing rule, or stop it. */
export function useChangeRule() {
  return api.query.useMutation("patch", "/api/v1/standing-rules/{rule}", {
    onSuccess: refreshAgents,
  });
}

/** Whether the agent can still be paused, resumed or stopped by its person. */
export function mayAct(agent: Pick<ListedAgent, "state">): boolean {
  return agent.state !== "stopped";
}

/** The words of the states of an agent. */
const STATE_TEXT = {
  active: "agents.state.active",
  paused: "agents.state.paused",
  stopped: "agents.state.stopped",
} as const satisfies Record<AgentStateWord, MessageKey>;

/** Where an agent stands, in words: *Active*, *Paused*, *Stopped*. */
export function agentStateText(state: AgentStateWord): string {
  return t(STATE_TEXT[state]);
}

/** Whether a standing rule is about to run out: it works, and its end is the warning days away or nearer. */
export function runsOutSoon(rule: Pick<Rule, "state" | "ends_at">, nowMs: number): boolean {
  return rule.state === "open" && daysUntil(rule.ends_at, nowMs) <= agentRuleWarningDays;
}

/** When a rule runs out, in words: *in 4 days*, *today*, or the date when it is further away. */
export function runsOutText(endsAt: string, nowMs: number): string {
  const days = daysUntil(endsAt, nowMs);
  if (days > agentRuleWarningDays) {
    return t("agents.rule-ends-on", { date: formatDate(endsAt) });
  }
  return days <= 0 ? t("agents.rule-ends-today") : t("agents.rule-ends-in", { count: days });
}

/** The agent with this ID among the person's agents. */
export function agentNamed(agents: readonly ListedAgent[], id: string): ListedAgent | undefined {
  return agents.find((agent) => agent.id === id);
}

/** The names a standing rule is told by: its saved search and its project. */
export interface RuleNames {
  readonly search: (id: string) => string;
  readonly project: (id: string) => string;
}

/** The projects the person can give agents work in, by ID: the one list of projects a person can name (shared with access keys). */
export function useProjectNames(): ReadonlyMap<string, string> {
  const choices = useProjectChoices();
  return new Map(
    (choices.data?.items ?? []).flatMap((project) =>
      project.id === undefined || project.id === null
        ? []
        : [[project.id, project.name ?? project.short_name ?? project.id] as const],
    ),
  );
}

/** The names of the saved searches and projects of these rules, read once each. */
export function useRuleNames(rules: readonly Rule[]): RuleNames {
  const ids = [...new Set(rules.map((rule) => rule.search))];
  const lists = useQueries({
    queries: ids.map((search) =>
      api.query.queryOptions("get", "/api/v1/saved-searches/{search}", {
        params: { path: { search } },
      }),
    ),
  });
  const searches = new Map(ids.map((id, at) => [id, lists[at]?.data?.name] as const));
  const projects = useProjectNames();
  return {
    search: (id) => searches.get(id) ?? t("agents.rule-search-gone"),
    project: (id) => projects.get(id) ?? t("agents.rule-project-gone"),
  };
}

/** The state of an agent and whose it is: *Active, agent of Ada Brandt*. */
export function agentLine(agent: Pick<ListedAgent, "state" | "person">): string {
  const state = agentStateText(agent.state);
  return agent.person === undefined || agent.person === null
    ? state
    : t("agents.state-of", { state, person: personName(agent.person) });
}
