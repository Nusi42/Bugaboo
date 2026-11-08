import { t } from "@bugaboo/messages";
import { buttonLook, ConfirmDialog, partOf, part } from "@bugaboo/ui";

import type { AccessKey } from "./access-keys.ts";
import { KeyFacts } from "./KeyFacts.tsx";
import { mayText, whereText } from "./key-words.ts";

export interface KeyRowProps {
  readonly item: AccessKey;
  /** Delete this key: it leaves the list at once and the server is asked in the background. */
  readonly onDelete: () => void;
}

/** One key of the person: its name, what it may do and where, when it runs out, and *Delete*. */
export function KeyRow({ item, onDelete }: KeyRowProps) {
  const name = item.name ?? "";
  return (
    <li {...partOf(part.item)}>
      <KeyFacts item={item}>
        <span {...partOf(part.muted)}>{mayText(item.choice, item.permissions?.length ?? 0)}</span>
        <span {...partOf(part.muted)}>{whereText(item.projects)}</span>
      </KeyFacts>
      <ConfirmDialog
        triggerLabel={t("access-keys.delete")}
        triggerName={t("access-keys.delete-name", { name })}
        title={t("access-keys.delete-title")}
        text={t("access-keys.delete-text")}
        confirmLabel={t("access-keys.delete-confirm")}
        cancelLabel={t("common.cancel")}
        confirmLook={buttonLook.danger}
        onConfirm={onDelete}
      />
    </li>
  );
}
