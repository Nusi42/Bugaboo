import { t } from "@bugaboo/messages";
import { Icon, iconName, iconSize, partOf, part } from "@bugaboo/ui";

import { agentStateText, type WorkState } from "./agent-state.ts";

export interface AgentMarkProps {
  /** The name of the agent. */
  readonly name: string;
  /** Where its work is. */
  readonly state: WorkState;
}

/**
 * The small mark that says an agent works on something (UX §4.17, §4.2): one icon whose words are
 * the agent and the state of its work, so the color never says it alone. One component for a row of
 * a list, the Details, an agent's own page and the lists of agents.
 */
export function AgentMark({ name, state }: AgentMarkProps) {
  const label = t("agent-work.mark", { agent: name, state: agentStateText(state) });
  return (
    <span title={label} {...partOf(part.agentMark)}>
      <Icon name={iconName.agent} size={iconSize.small} />
      <span {...partOf(part.visuallyHidden)}>{label}</span>
    </span>
  );
}
