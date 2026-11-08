import { t } from "@bugaboo/messages";
import { Link, partOf, part } from "@bugaboo/ui";

import { agentAddress } from "./agent-address.ts";
import { type ListedAgent, type MyWorkItem } from "./agents.ts";
import { AgentStateLine } from "./AgentState.tsx";
import { StateButtons } from "./StateButtons.tsx";
import { WorkLines } from "./WorkLines.tsx";

export interface AgentSectionProps {
  readonly agent: ListedAgent;
  /** The work of this agent. */
  readonly work: readonly MyWorkItem[];
}

/**
 * One agent on *My agents* (UX §4.17): its name and state pill (and the limit that paused it), what
 * it works on with the state of each piece of work, and *Pause* or *Resume* and *Stop*.
 */
export function AgentSection({ agent, work }: AgentSectionProps) {
  const open = work.filter((item) => item.work.open);
  return (
    <li {...partOf(part.agentsAgent)}>
      <div {...partOf(part.agentsAgentHead)}>
        <span {...partOf(part.startColumn)}>
          <span {...partOf(part.itemTitle)}>
            <Link href={agentAddress(agent.id)}>{agent.name}</Link>
          </span>
          <AgentStateLine agent={agent} />
        </span>
        <StateButtons agent={agent} />
      </div>
      {open.length === 0 ? (
        <p {...partOf(part.muted)}>
          {agent.state === "stopped" ? t("agents.work-none-stopped") : t("agents.work-none")}
        </p>
      ) : (
        <WorkLines work={open} />
      )}
    </li>
  );
}
