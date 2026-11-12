import { t } from "@bugaboo/messages";
import type { MenuAction } from "@bugaboo/ui";

import { mayAct, useChangeAgent, type AgentActionWord, type ListedAgent } from "./agents.ts";

/** What can be done to an agent's state, and how the request of it is doing. */
export interface AgentStateChange {
  /** Sends *Pause*, *Resume* or *Stop*. */
  readonly send: (action: AgentActionWord) => void;
  readonly isPending: boolean;
  readonly error: unknown;
  /** Its person paused it, or a limit did: *Resume* is offered instead of *Pause*. */
  readonly paused: boolean;
  /** *Pause* or *Resume* for the agent's own person while it works or rests; none otherwise. */
  readonly pauseOrResume: MenuAction | undefined;
}

/**
 * The change of an agent's state, on the version of it the screen read: the one place that sends it, for the
 * buttons of an agent and the swipe of its row (UX §4.17, §3.3).
 */
export function useAgentState(agent: ListedAgent): AgentStateChange {
  const change = useChangeAgent();
  const send = (action: AgentActionWord): void => {
    change.mutate({
      params: { path: { agent: agent.id } },
      body: { action, version: agent.version },
    });
  };
  const paused = agent.state === "paused";
  return {
    send,
    isPending: change.isPending,
    error: change.error,
    paused,
    pauseOrResume:
      agent.yours && mayAct(agent)
        ? {
            id: paused ? "resume" : "pause",
            label: t(paused ? "agents.resume" : "agents.pause"),
            onAction: () => {
              send(paused ? "resume" : "pause");
            },
          }
        : undefined,
  };
}
