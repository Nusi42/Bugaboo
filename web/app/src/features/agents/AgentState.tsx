import { partOf, part } from "@bugaboo/ui";

import { agentPill, pausedSentence } from "./agent-state.ts";
import { agentLine, type ListedAgent } from "./agents.ts";

/** Where an agent stands, as the pill of agent work (UX §4.17): *Agent working*, *Paused* or *Stopped*. */
export function AgentStatePill({ agent }: { readonly agent: Pick<ListedAgent, "state"> }) {
  const pill = agentPill(agent.state);
  return <span {...partOf(part.agentWorkPill, { variants: [pill.variant] })}>{pill.text}</span>;
}

export interface AgentStateLineProps {
  readonly agent: ListedAgent;
  /** Also say whose it is (*Agent of Ada Brandt*): on its page, not on *My agents*, where all are the reader's. */
  readonly withPerson?: boolean;
}

/** The line under the name of an agent: its state pill, whose it is, and the limit that paused it. */
export function AgentStateLine({ agent, withPerson = false }: AgentStateLineProps) {
  const whose = withPerson ? agentLine(agent) : undefined;
  const paused = pausedSentence(agent);
  return (
    <span {...partOf(part.agentsStateLine)}>
      <AgentStatePill agent={agent} />
      {whose === undefined ? null : <span {...partOf(part.muted)}>{whose}</span>}
      {paused === undefined ? null : <span {...partOf(part.agentsLimitSentence)}>{paused}</span>}
    </span>
  );
}
