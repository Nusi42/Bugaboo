import { t } from "@bugaboo/messages";
import { Button, buttonLook, ConfirmDialog, partOf, part } from "@bugaboo/ui";
import type { ReactNode } from "react";

import { ActionError } from "./ActionError.tsx";
import { mayAct, type ListedAgent } from "./agents.ts";
import { useAgentState, type AgentStateChange } from "./use-agent-state.ts";

export interface StateButtonsProps {
  readonly agent: ListedAgent;
  /** The change of the agent's state the screen keeps (`useAgentState`): its row's swipe sends the same one. */
  readonly change: AgentStateChange;
  /** Only *Stop*: the list of every agent of the organization, where an administrator stops any agent. */
  readonly stopOnly?: boolean;
  /** The row's *More* on a phone, at the end of the buttons (*My agents*). */
  readonly more?: ReactNode;
}

/**
 * *Pause* or *Resume* for the agent's own person, and *Stop* in the color of danger with its
 * question first: the one set of buttons of an agent, on *My agents*, its page and the list of agents.
 */
export function StateButtons({ agent, change, stopOnly = false, more }: StateButtonsProps) {
  if (!mayAct(agent)) {
    return null;
  }
  const { pauseOrResume, send } = change;
  return (
    <div {...partOf(part.agentsActions)}>
      {pauseOrResume !== undefined && !stopOnly ? (
        <Button
          isPending={change.isPending}
          aria-label={t(change.paused ? "agents.resume-name" : "agents.pause-name", {
            agent: agent.name,
          })}
          onPress={pauseOrResume.onAction}
        >
          {pauseOrResume.label}
        </Button>
      ) : null}
      <ConfirmDialog
        triggerLabel={t("agents.stop")}
        triggerName={t("agents.stop-name", { agent: agent.name })}
        triggerLook={buttonLook.danger}
        title={t("agents.stop-title", { agent: agent.name })}
        text={t(agent.yours ? "agents.stop-text" : "agents.stop-others-text")}
        confirmLabel={t("agents.stop")}
        cancelLabel={t("common.cancel")}
        confirmLook={buttonLook.danger}
        onConfirm={() => {
          send("stop");
        }}
      />
      {more}
      <ActionError error={change.error} />
    </div>
  );
}

/** The buttons of an agent that keep the change of its state themselves: the page of an agent, the list of every agent. */
export function OwnStateButtons(props: Omit<StateButtonsProps, "change">) {
  const change = useAgentState(props.agent);
  return <StateButtons {...props} change={change} />;
}
