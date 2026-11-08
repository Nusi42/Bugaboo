import { t } from "@bugaboo/messages";
import { Button, buttonLook, ConfirmDialog } from "@bugaboo/ui";

import { ActionError } from "./ActionError.tsx";
import { mayAct, useChangeAgent, type ListedAgent } from "./agents.ts";

/** *Pause* or *Resume*, and *Stop* with its question. */
export function StateButtons({ agent }: { readonly agent: ListedAgent }) {
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
    <>
      <Button
        isPending={change.isPending}
        onPress={() => {
          send(agent.state === "paused" ? "resume" : "pause");
        }}
      >
        {t(agent.state === "paused" ? "agents.resume" : "agents.pause")}
      </Button>
      <ConfirmDialog
        triggerLabel={t("agents.stop")}
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
    </>
  );
}
