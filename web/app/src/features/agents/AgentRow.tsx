import { partOf, part } from "@bugaboo/ui";
import type { ReactNode } from "react";

export interface AgentRowProps {
  /** The row is of a stopped agent: quieter than the rows of agents that work or rest. */
  readonly stopped?: boolean;
  readonly children: ReactNode;
}

/**
 * One row of a list of *My agents*, an agent's page or the list of every agent: the words, and
 * under them (on a phone, always) the buttons of the row.
 */
export function AgentRow({ stopped = false, children }: AgentRowProps) {
  return (
    <li {...partOf(part.item, { variants: ["buttons-below", stopped && "stopped"] })}>
      {children}
    </li>
  );
}
