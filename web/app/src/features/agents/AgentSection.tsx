import { t } from "@bugaboo/messages";
import { Link, partOf, part, sidesOf, type RowActions } from "@bugaboo/ui";

import { useRowSwipe } from "../../shared/use-row-swipe.tsx";
import { agentAddress } from "./agent-address.ts";
import { mayAct, type ListedAgent, type MyWorkItem } from "./agents.ts";
import { AgentStateLine } from "./AgentState.tsx";
import { StateButtons } from "./StateButtons.tsx";
import { useAgentState } from "./use-agent-state.ts";
import { WorkLines } from "./WorkLines.tsx";

export interface AgentSectionProps {
  readonly agent: ListedAgent;
  /** The work of this agent. */
  readonly work: readonly MyWorkItem[];
}

/**
 * One agent on *My agents* (UX §4.17): its name and state pill (and the limit that paused it), what
 * it works on with the state of each piece of work, and *Pause* or *Resume* and *Stop*. On a phone a swipe brings in
 * *Pause* or *Resume*, which the row's *More* holds too (UX §3.3); *Stop* asks first, so it stays a button.
 */
export function AgentSection({ agent, work }: AgentSectionProps) {
  const open = work.filter((item) => item.work.open);
  const change = useAgentState(agent);
  const actions: RowActions =
    change.pauseOrResume === undefined ? {} : { end: [change.pauseOrResume] };
  const swipe = useRowSwipe(sidesOf(actions));
  return (
    <li
      {...partOf(part.agentsAgent, { variants: [!mayAct(agent) && "stopped"] })}
      {...swipe.rowProps}
    >
      <div {...partOf(part.agentsAgentHead)}>
        <span {...partOf(part.startColumn)}>
          <span {...partOf(part.itemTitle)}>
            <Link href={agentAddress(agent.id)}>{agent.name}</Link>
          </span>
          <AgentStateLine agent={agent} />
        </span>
        <StateButtons agent={agent} change={change} more={swipe.actions(actions, agent.name)} />
      </div>
      {open.length > 0 ? <WorkLines work={open} /> : null}
      {/* A stopped agent says nothing more than its state: the card is quiet. */}
      {open.length === 0 && mayAct(agent) ? (
        <p {...partOf(part.muted)}>{t("agents.work-none")}</p>
      ) : null}
    </li>
  );
}
