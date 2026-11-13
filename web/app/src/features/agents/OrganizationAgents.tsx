import { t } from "@bugaboo/messages";
import { partOf, part, ShowMoreOf } from "@bugaboo/ui";

import { itemsOf } from "../../shared/paged.ts";
import { QueryGate } from "../shell/index.ts";
import { LastUsed } from "../my-settings/index.ts";
import {
  listedAgents,
  mayAct,
  useOrganizationAgents,
  type AgentEntry,
  type ListedAgent,
} from "./agents.ts";
import { AgentRow } from "./AgentRow.tsx";
import { AgentStateLine } from "./AgentState.tsx";
import { StateButtons } from "./StateButtons.tsx";

/** One agent of the organization: its name, its state and person, its last use, and *Stop*. */
function Row({
  agent,
  lastUsed,
}: {
  readonly agent: ListedAgent;
  readonly lastUsed: string | null | undefined;
}) {
  return (
    <AgentRow stopped={!mayAct(agent)}>
      <span {...partOf(part.startColumn)}>
        <span {...partOf(part.itemTitle)}>{agent.name}</span>
        <AgentStateLine agent={agent} withPerson />
        <LastUsed at={lastUsed} />
      </span>
      <StateButtons agent={agent} stopOnly />
    </AgentRow>
  );
}

/**
 * The agents in the order the server pages them. An agent that changed its state between two
 * pages can come twice (once under its old state); it is listed once, where it stands now.
 */
function oncePerAgent(items: readonly AgentEntry[]): readonly AgentEntry[] {
  const last = new Map(items.map((item, at) => [item.id, at] as const));
  return items.filter((item, at) => last.get(item.id) === at);
}

/**
 * Every agent of the organization (*Organization settings › Connections › Agents*, AI-30): its
 * person (by *Who sees people*; administrators see names), its state and last use, and *Stop*.
 * The server sorts it, working agents first, then paused, then stopped, by name inside each
 * (M7-F224), so a working agent is never behind the stopped ones.
 */
export function OrganizationAgents() {
  const found = useOrganizationAgents();
  const entries = oncePerAgent(itemsOf(found.data?.pages));
  const lastUsed = new Map(entries.map((entry) => [entry.id, entry.last_used_at] as const));
  const agents = listedAgents(entries);
  return (
    <QueryGate error={found.error} data={found.data === undefined ? undefined : agents}>
      {(all) =>
        all.length === 0 ? (
          <p {...partOf(part.muted)}>{t("agents.org-none")}</p>
        ) : (
          <>
            <ul {...partOf(part.list)} aria-label={t("agents.org-list")}>
              {all.map((agent) => (
                <Row key={agent.id} agent={agent} lastUsed={lastUsed.get(agent.id)} />
              ))}
            </ul>
            <ShowMoreOf label={t("agents.org-more")} list={found} />
          </>
        )
      }
    </QueryGate>
  );
}
