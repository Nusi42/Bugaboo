import { t } from "@bugaboo/messages";
import { Button, buttonLook, ConfirmDialog, partOf, part } from "@bugaboo/ui";

import { KeyFacts } from "../my-settings/index.ts";
import { ActionError } from "./ActionError.tsx";
import { useTakeKeyBack, type AgentKey } from "./agents.ts";

export interface KeyRowProps {
  readonly agent: string;
  readonly item: AgentKey;
  readonly stopped: boolean;
  readonly onRenew: () => void;
}

/** One key of the agent: its name, the start of the key, when it runs out, *Renew* and *Take back*. */
export function KeyRow({ agent, item, stopped, onRenew }: KeyRowProps) {
  const takeBack = useTakeKeyBack();
  return (
    <li {...partOf(part.item)}>
      <KeyFacts item={item} />
      <div {...partOf(part.agentsActions)}>
        {stopped ? null : (
          <Button aria-label={t("agents.key-renew-name", { name: item.name })} onPress={onRenew}>
            {t("agents.renew")}
          </Button>
        )}
        <ConfirmDialog
          triggerLabel={t("agents.key-take-back")}
          triggerName={t("agents.key-take-back-name", { name: item.name })}
          title={t("agents.key-take-back-title")}
          text={t("agents.key-take-back-text")}
          confirmLabel={t("agents.key-take-back")}
          cancelLabel={t("common.cancel")}
          confirmLook={buttonLook.danger}
          onConfirm={() => {
            takeBack.mutate({ params: { path: { agent, access_key: item.id } } });
          }}
        />
        <ActionError error={takeBack.error} />
      </div>
    </li>
  );
}
