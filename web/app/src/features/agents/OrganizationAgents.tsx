import { t } from "@bugaboo/messages";
import { buttonLook, ConfirmDialog, partOf, part } from "@bugaboo/ui";

import { formatDateTime } from "../../shared/format.ts";
import { personName } from "../../shared/person.ts";
import { QueryGate } from "../shell/index.ts";
import {
  agentStateText,
  listedAgents,
  useChangeAgent,
  useOrganizationAgents,
  type ListedAgent,
} from "./agents.ts";

interface RowProps {
  readonly agent: ListedAgent;
  /** When it last signed in with a key; none while it never did. */
  readonly lastUsed: string | undefined;
}

/** One agent of the organization: its name, its person, its state, its last use, and *Stop*. */
function Row({ agent, lastUsed }: RowProps) {
  const change = useChangeAgent();
  return (
    <li {...partOf(part.item)}>
      <span {...partOf(part.startColumn)}>
        <span {...partOf(part.itemTitle)}>{agent.name}</span>
        <span {...partOf(part.muted)}>
          {agent.person === undefined || agent.person === null
            ? t("agents.person-unknown")
            : t("agents.of", { person: personName(agent.person) })}
        </span>
        <span {...partOf(part.muted)}>{agentStateText(agent.state)}</span>
        <span {...partOf(part.muted)}>
          {lastUsed === undefined
            ? t("agents.never-used")
            : t("agents.last-used", { when: formatDateTime(lastUsed) })}
        </span>
      </span>
      {agent.state === "stopped" ? null : (
        <ConfirmDialog
          triggerLabel={t("agents.stop")}
          triggerName={t("agents.stop-name", { agent: agent.name })}
          title={t("agents.stop-title", { agent: agent.name })}
          text={t("agents.stop-others-text")}
          confirmLabel={t("agents.stop")}
          cancelLabel={t("common.cancel")}
          confirmLook={buttonLook.danger}
          onConfirm={() => {
            change.mutate({
              params: { path: { agent: agent.id } },
              body: { action: "stop", version: agent.version },
            });
          }}
        />
      )}
    </li>
  );
}

/**
 * Every agent of the organization (*Organization settings › Connections › Agents*, AI-30): its
 * person (by *Who sees people*; administrators see names), its state and last use, and *Stop*.
 */
export function OrganizationAgents() {
  const found = useOrganizationAgents();
  const items = found.data?.items;
  const lastUsed = new Map(
    (items ?? []).flatMap((item) =>
      item.id === undefined ||
      item.id === null ||
      item.last_used_at === undefined ||
      item.last_used_at === null
        ? []
        : [[item.id, item.last_used_at] as const],
    ),
  );
  return (
    <QueryGate error={found.error} data={items}>
      {(all) => {
        const agents = listedAgents(all);
        return agents.length === 0 ? (
          <p {...partOf(part.muted)}>{t("agents.org-none")}</p>
        ) : (
          <ul {...partOf(part.list)} aria-label={t("agents.org-list")}>
            {agents.map((agent) => (
              <Row key={agent.id} agent={agent} lastUsed={lastUsed.get(agent.id)} />
            ))}
          </ul>
        );
      }}
    </QueryGate>
  );
}
