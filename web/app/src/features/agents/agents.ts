import { agentRuleWarningDays, type Schema } from "@bugaboo/api-client";
import { t } from "@bugaboo/messages";

import { agentWorkKeys } from "../../shared/agent-work-keys.ts";
import { api } from "../../shared/api.ts";
import { daysUntil, formatDate } from "../../shared/format.ts";
import { personName } from "../../shared/person.ts";
import { pageSize, usePaged } from "../../shared/paged.ts";
import { agentKeys } from "./agent-keys.ts";
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
/** The limit an agent reached when a limit paused it. */
export type LimitWord = Schema<"LimitKind">;
/** What an action on an agent does. */
export type AgentActionWord = "pause" | "resume" | "stop";

/** An agent whose list entry holds everything the screens need. */
export interface ListedAgent {
  readonly id: string;
  readonly name: string;
  readonly state: AgentStateWord;
  /** The limit that paused it; none when it is not paused or its person paused it. */
  readonly pausedFor: LimitWord | undefined;
  readonly version: number;
  readonly offered: boolean;
  /** The person who reads is the person responsible for it. */
  readonly yours: boolean;
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
            pausedFor: item.paused_for ?? undefined,
            version: item.version,
            offered: item.offered === true,
            yours: item.yours === true,
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

/** The standing rules of one agent, a page at a time: those that work first, then those that ended. */
export function useAgentRules(agent: string) {
  return usePaged({
    queryKey: agentKeys.rules(agent),
    read: (after) =>
      api.client.GET("/api/v1/agents/{agent}/standing-rules", {
        params: { path: { agent }, query: { limit: pageSize.short, ...after } },
      }),
  });
}

/** The projects the person can name, flat: the list access keys use, so the answer is read once for both. */
export function useProjectChoices() {
  return api.query.useQuery("get", "/api/v1/access-keys/project-choices", {
    params: { query: { limit: pageSize.largest } },
  });
}

/** Every agent of the organization, for administrators, a page at a time. */
export function useOrganizationAgents() {
  return usePaged({
    queryKey: agentKeys.organization(),
    read: (after) =>
      api.client.GET("/api/v1/organization/agents", {
        params: {
          query: {
            limit: pageSize.short,
            fields: "id,name,state,person,yours,last_used_at,version",
            ...after,
          },
        },
      }),
  });
}

/**
 * The list of every agent is sorted by state, so an agent that changed its state is not where the
 * cursors of the pages read so far would find it: only its first page is kept and read again, and
 * *Show more agents* goes on from there.
 */
function keepFirstPageOfEveryAgent(): void {
  queryClient.setQueryData(agentKeys.organization(), (kept: unknown) =>
    isInfinite(kept)
      ? { pages: kept.pages.slice(0, 1), pageParams: kept.pageParams.slice(0, 1) }
      : kept,
  );
}

/** Whether what the cache holds is a list read a page at a time. */
function isInfinite(
  value: unknown,
): value is { readonly pages: readonly unknown[]; readonly pageParams: readonly unknown[] } {
  return typeof value === "object" && value !== null && "pages" in value && "pageParams" in value;
}

/** Agents, their work or their rules changed: every list that shows them is read again. */
export function refreshAgents(): void {
  keepFirstPageOfEveryAgent();
  refreshRoutes([
    "/api/v1/agents",
    "/api/v1/me/agents",
    "/api/v1/organization/agents",
    "/api/v1/standing-rules",
    "/api/v1/issues/{issue}/agent-work",
  ]);
  void queryClient.invalidateQueries({ queryKey: agentWorkKeys.myAgents() });
  void queryClient.invalidateQueries({ queryKey: agentKeys.all });
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

/** Take back the offer of an agent, or decline it. */
export function useTakeBackOffer() {
  return api.query.useMutation("delete", "/api/v1/agents/{agent}/hand-over", {
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

/** Whether a standing rule is about to run out: it works, and its end is the warning days away or nearer. */
export function runsOutSoon(rule: Pick<Rule, "state" | "ends_at">, nowMs: number): boolean {
  return (
    rule.state === "open" &&
    rule.ends_at !== undefined &&
    rule.ends_at !== null &&
    daysUntil(rule.ends_at, nowMs) <= agentRuleWarningDays
  );
}

/** The whole days until a rule runs out, none below zero. */
export function ruleDaysLeft(rule: Pick<Rule, "ends_at">, nowMs: number): number {
  return rule.ends_at === undefined || rule.ends_at === null
    ? Number.POSITIVE_INFINITY
    : Math.max(daysUntil(rule.ends_at, nowMs), 0);
}

/** When a rule runs out, in words: *in 4 days*, *today*, the date when it is further away, or that it never does. */
export function runsOutText(endsAt: string | null | undefined, nowMs: number): string {
  if (endsAt === undefined || endsAt === null) {
    return t("agents.rule-never-ends");
  }
  const days = daysUntil(endsAt, nowMs);
  if (days > agentRuleWarningDays) {
    return t("agents.rule-ends-on", { date: formatDate(endsAt) });
  }
  return days <= 0 ? t("agents.rule-ends-today") : t("agents.rule-ends-in", { count: days });
}

/** The agent of the person with this ID, once the list is read; none for an empty ID. */
export function useAgentNamed(id: string): ListedAgent | undefined {
  return agentNamed(listedAgents(useMyAgentList().data?.items), id);
}

/** The agent with this ID among the person's agents. */
export function agentNamed(agents: readonly ListedAgent[], id: string): ListedAgent | undefined {
  return agents.find((agent) => agent.id === id);
}

/** Whose an agent is: *Agent of Ada Brandt*; none when the reader may not see the person. */
export function agentLine(agent: Pick<ListedAgent, "person">): string | undefined {
  return agent.person === undefined || agent.person === null
    ? undefined
    : t("agents.of", { person: personName(agent.person) });
}
