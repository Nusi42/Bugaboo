import { t } from "@bugaboo/messages";
import { partOf, part } from "@bugaboo/ui";

import { QueryGate } from "../shell/index.ts";
import { listedAgents, mayAct, useAgentRecord, type AgentRecord } from "./agents.ts";
import { AgentHead } from "./AgentHead.tsx";
import { AgentKeys } from "./AgentKeys.tsx";
import { AgentLimits } from "./AgentLimits.tsx";
import { AgentRules } from "./AgentRules.tsx";
import { AgentWorkCard } from "./AgentWorkCard.tsx";
import { HandToPerson } from "./HandToPerson.tsx";

/**
 * The page of an agent (UX §4.17): its person and state, its keys (each runs out), its limits, its work and its standing rules,
 * *Pause* and *Stop*, and *Hand to another person*.
 */
export function AgentPage({ id }: { readonly id: string }) {
  const found = useAgentRecord(id);
  const record: AgentRecord | undefined = found.data;
  return (
    <QueryGate
      error={found.error}
      data={record}
      notFound={<p {...partOf(part.muted)}>{t("agents.not-found")}</p>}
    >
      {(loaded) => {
        const agent = listedAgents([loaded])[0];
        return agent === undefined ? null : (
          <div {...partOf(part.stack)}>
            <AgentHead agent={agent} />
            <AgentKeys agent={agent.id} keys={loaded.keys} stopped={!mayAct(agent)} />
            <AgentLimits agent={agent} maximums={loaded.maximums} />
            <AgentWorkCard agent={agent} />
            <AgentRules agent={agent} />
            <HandToPerson agent={agent} />
          </div>
        );
      }}
    </QueryGate>
  );
}
