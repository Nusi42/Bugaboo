import { t } from "@bugaboo/messages";
import { Button, buttonLook, ConfirmDialog, partOf, part } from "@bugaboo/ui";

import { ActionError } from "./ActionError.tsx";
import { mayAct, useChangeAgent, type AgentActionWord, type ListedAgent } from "./agents.ts";

export interface StateButtonsProps {
  readonly agent: ListedAgent;
  /** Only *Stop*: the list of every agent of the organization, where an administrator stops any agent. */
  readonly stopOnly?: boolean;
}

/**
 * *Pause* or *Resume* for the agent's own person, and *Stop* in the color of danger with its
 * question first: the one set of buttons of an agent, on *My agents*, its page and the list of agents.
 */
export function StateButtons({ agent, stopOnly = false }: StateButtonsProps) {
  const change = useChangeAgent();
  const send = (action: AgentActionWord): void => {
    change.mutate({
      params: { path: { agent: agent.id } },
      body: { action, version: agent.version },
    });
  };
  if (!mayAct(agent)) {
    return null;
  }
  const paused = agent.state === "paused";
  return (
    <div {...partOf(part.agentsActions)}>
      {agent.yours && !stopOnly ? (
        <Button
          isPending={change.isPending}
          aria-label={t(paused ? "agents.resume-name" : "agents.pause-name", { agent: agent.name })}
          onPress={() => {
            send(paused ? "resume" : "pause");
          }}
        >
          {t(paused ? "agents.resume" : "agents.pause")}
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
      <ActionError error={change.error} />
    </div>
  );
}
