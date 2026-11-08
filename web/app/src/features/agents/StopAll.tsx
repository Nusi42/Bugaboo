import { t } from "@bugaboo/messages";
import { buttonLook, ConfirmDialog, partOf, part } from "@bugaboo/ui";

import { ActionError } from "./ActionError.tsx";
import { useStopAll, type ListedAgent } from "./agents.ts";

/** *Stop all my agents*, with a question first that names the agents (UX §4.17). */
export function StopAll({ agents }: { readonly agents: readonly ListedAgent[] }) {
  const stop = useStopAll();
  return (
    <>
      <ConfirmDialog
        triggerLabel={t("agents.stop-all")}
        triggerLook={buttonLook.danger}
        title={t("agents.stop-all-title")}
        text={t("agents.stop-all-text")}
        confirmLabel={t("agents.stop-all-confirm")}
        cancelLabel={t("common.cancel")}
        confirmLook={buttonLook.danger}
        onConfirm={() => {
          stop.mutate({});
        }}
      >
        <ul {...partOf(part.list)}>
          {agents.map((agent) => (
            <li key={agent.id}>{agent.name}</li>
          ))}
        </ul>
      </ConfirmDialog>
      <ActionError error={stop.error} />
    </>
  );
}
