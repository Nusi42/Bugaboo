import { t } from "@bugaboo/messages";
import { Link, partOf, part } from "@bugaboo/ui";

import { agentAddress } from "./agent-address.ts";
import { mayAct, type ListedAgent, type MyWorkItem } from "./agents.ts";
import { AgentStateLine } from "./AgentState.tsx";
import { StateButtons } from "./StateButtons.tsx";
import { WorkLines } from "./WorkLines.tsx";

export interface AgentSectionProps {
  readonly agent: ListedAgent;
  /** The work of this agent. */
  readonly work: readonly MyWorkItem[];
}

/**
 * One agent on *My agents* (UX §4.17): its name and state pill (and the limit that paused it), the
 * switches its person turned on, what it works on with the state of each piece of work, and *Pause*
 * or *Resume* and *Stop*.
 */
export function AgentSection({ agent, work }: AgentSectionProps) {
  const open = work.filter((item) => item.work.open);
  return (
    <li {...partOf(part.agentsAgent, { variants: [!mayAct(agent) && "stopped"] })}>
      <div {...partOf(part.agentsAgentHead)}>
        <span {...partOf(part.startColumn)}>
          <span {...partOf(part.itemTitle)}>
            <Link href={agentAddress(agent.id)}>{agent.name}</Link>
          </span>
          <AgentStateLine agent={agent} />
        </span>
        <StateButtons agent={agent} />
      </div>
      {/* What its person lets it do beyond its work, when anything: both start off. */}
      {agent.mayHandOver ? <p {...partOf(part.muted)}>{t("agents.may-hand-over")}</p> : null}
      {agent.mayUndo ? <p {...partOf(part.muted)}>{t("agents.may-undo")}</p> : null}
      {open.length > 0 ? <WorkLines work={open} /> : null}
      {/* A stopped agent says nothing more than its state: the card is quiet. */}
      {open.length === 0 && mayAct(agent) ? (
        <p {...partOf(part.muted)}>{t("agents.work-none")}</p>
      ) : null}
    </li>
  );
}
