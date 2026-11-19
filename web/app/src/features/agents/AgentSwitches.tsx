import { t, type MessageKey } from "@bugaboo/messages";
import { Card, Switch, partOf, part } from "@bugaboo/ui";

import { useChangeAgent, type ListedAgent } from "./agents.ts";
import { ActionError } from "./ActionError.tsx";

export interface AgentSwitchesProps {
  readonly agent: ListedAgent;
}

/**
 * What a person lets their agent do beyond its work (ADR-0029, amendment M9): hand issues of its own
 * work over to the person's other agents, and undo its own runs. Both are off when the agent is
 * made; only its person changes them, and a stopped agent shows them without a way to change them.
 */
export function AgentSwitches({ agent }: AgentSwitchesProps) {
  const change = useChangeAgent();
  const editable = agent.yours && agent.state !== "stopped";
  const save = (mayHandOver: boolean, mayUndo: boolean) => {
    change.mutate({
      params: { path: { agent: agent.id } },
      body: {
        switches: { may_hand_over: mayHandOver, may_undo: mayUndo },
        version: agent.version,
      },
    });
  };
  const rows = [
    [
      "agents.may-hand-over",
      agent.mayHandOver,
      (on: boolean) => {
        save(on, agent.mayUndo);
      },
    ],
    [
      "agents.may-undo",
      agent.mayUndo,
      (on: boolean) => {
        save(agent.mayHandOver, on);
      },
    ],
  ] as const;
  return (
    <Card heading={t("agents.switches")} intro={t("agents.switches-intro")}>
      {editable ? (
        rows.map(([name, on, set]) => (
          <Switch
            key={name}
            label={t(name)}
            isSelected={on}
            isDisabled={change.isPending}
            onChange={set}
          />
        ))
      ) : (
        <SwitchFacts rows={rows} />
      )}
      <ActionError error={change.error} />
    </Card>
  );
}

/** The switches as facts, for an agent that is not the reader's or that is stopped. */
function SwitchFacts({
  rows,
}: {
  readonly rows: readonly (readonly [MessageKey, boolean, unknown])[];
}) {
  return (
    <dl {...partOf(part.agentsFacts)}>
      {rows.map(([name, on]) => (
        <div key={name} {...partOf(part.agentsFact)}>
          <dt {...partOf(part.muted)}>{t(name)}</dt>
          <dd {...partOf(part.agentsFactValue)}>{t(on ? "common.on" : "common.off")}</dd>
        </div>
      ))}
    </dl>
  );
}
