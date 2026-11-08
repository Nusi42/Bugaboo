import { t } from "@bugaboo/messages";
import { partOf, part, ShowMoreOf } from "@bugaboo/ui";

import { formatDateTime } from "../../shared/format.ts";
import { itemsOf } from "../../shared/paged.ts";
import { personName } from "../../shared/person.ts";
import { QueryGate } from "../shell/index.ts";
import { agentsInOrder } from "./agent-state.ts";
import { listedAgents, useOrganizationAgents, type ListedAgent } from "./agents.ts";
import { AgentStatePill } from "./AgentState.tsx";
import { StateButtons } from "./StateButtons.tsx";

interface RowProps {
  readonly agent: ListedAgent;
  /** When it last signed in with a key; none while it never did. */
  readonly lastUsed: string | undefined;
}

/** One agent of the organization: its name, its person, its state pill, its last use, and *Stop*. */
function Row({ agent, lastUsed }: RowProps) {
  return (
    <li {...partOf(part.item)}>
      <span {...partOf(part.startColumn)}>
        <span {...partOf(part.itemTitle)}>{agent.name}</span>
        <span {...partOf(part.muted)}>
          {agent.person === undefined || agent.person === null
            ? t("agents.person-unknown")
            : t("agents.of", { person: personName(agent.person) })}
        </span>
        <span {...partOf(part.agentsStateLine)}>
          <AgentStatePill agent={agent} />
        </span>
        <span {...partOf(part.muted)}>
          {lastUsed === undefined
            ? t("agents.never-used")
            : t("agents.last-used", { when: formatDateTime(lastUsed) })}
        </span>
      </span>
      <StateButtons agent={agent} stopOnly />
    </li>
  );
}

/**
 * Every agent of the organization (*Organization settings › Connections › Agents*, AI-30): its
 * person (by *Who sees people*; administrators see names), its state and last use, and *Stop*.
 */
export function OrganizationAgents() {
  const found = useOrganizationAgents();
  const items = itemsOf(found.data?.pages);
  const lastUsed = new Map<string, string>();
  for (const item of items) {
    if (
      item.id !== undefined &&
      item.id !== null &&
      item.last_used_at !== undefined &&
      item.last_used_at !== null
    ) {
      lastUsed.set(item.id, item.last_used_at);
    }
  }
  // The one order of agents, over the pages read so far (M7-F224).
  const agents = agentsInOrder(listedAgents(items));
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
