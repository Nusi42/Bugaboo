import { t } from "@bugaboo/messages";
import { Button, buttonLook, ConfirmDialog, Link, partOf, part } from "@bugaboo/ui";

import { agentAddress } from "./agent-address.ts";
import { agentLine, mayAct, useChangeAgent, type ListedAgent, type MyWorkItem } from "./agents.ts";
import { ActionError } from "./ActionError.tsx";
import { WorkLines } from "./WorkLines.tsx";

/** *Pause* or *Resume*, and *Stop* with its question, for one agent. */
function AgentButtons({ agent }: { readonly agent: ListedAgent }) {
  const change = useChangeAgent();
  const send = (action: "pause" | "resume" | "stop"): void => {
    change.mutate({
      params: { path: { agent: agent.id } },
      body: { action, version: agent.version },
    });
  };
  if (!mayAct(agent)) {
    return null;
  }
  return (
    <div {...partOf(part.agentsActions)}>
      <Button
        isPending={change.isPending}
        aria-label={t(agent.state === "paused" ? "agents.resume-name" : "agents.pause-name", {
          agent: agent.name,
        })}
        onPress={() => {
          send(agent.state === "paused" ? "resume" : "pause");
        }}
      >
        {t(agent.state === "paused" ? "agents.resume" : "agents.pause")}
      </Button>
      <ConfirmDialog
        triggerLabel={t("agents.stop")}
        triggerName={t("agents.stop-name", { agent: agent.name })}
        title={t("agents.stop-title", { agent: agent.name })}
        text={t("agents.stop-text")}
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

export interface AgentSectionProps {
  readonly agent: ListedAgent;
  /** The work of this agent. */
  readonly work: readonly MyWorkItem[];
}

/**
 * One agent on *My agents* (UX §4.17): its name and state, what it works on with the state of each piece
 * of work, and *Pause* or *Resume* and *Stop*.
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
          <span {...partOf(part.muted)}>{agentLine(agent)}</span>
        </span>
        <AgentButtons agent={agent} />
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
