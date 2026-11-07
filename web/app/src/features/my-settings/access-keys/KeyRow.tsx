import { t } from "@bugaboo/messages";
import { buttonLook, ConfirmDialog, partOf, part, SentenceWithValue } from "@bugaboo/ui";

import { formatDate, formatDateTime } from "../../../shared/format.ts";
import type { AccessKey } from "./access-keys.ts";
import { mayText, whereText } from "./key-words.ts";

export interface KeyRowProps {
  readonly item: AccessKey;
  /** Delete this key: it leaves the list at once and the server is asked in the background. */
  readonly onDelete: () => void;
}

/** When the key runs out, or ran out. */
function RunsOut({ item }: { readonly item: AccessKey }) {
  const day = item.expires_at;
  if (day === undefined || day === null) {
    return null;
  }
  // The day on the clock of the region, the clock the box of the form counts the days by.
  const when = formatDate(day);
  return item.expired === true ? (
    <span {...partOf(part.accessKeysExpired)}>{t("access-keys.row.expired", { when })}</span>
  ) : (
    <span {...partOf(part.muted)}>{t("access-keys.row.expires", { when })}</span>
  );
}

/** One key of the person: its name, what it may do and where, when it runs out, and *Delete*. */
export function KeyRow({ item, onDelete }: KeyRowProps) {
  const name = item.name ?? "";
  const used = item.last_used_at;
  const start = `bb_${item.prefix ?? ""}`;
  return (
    <li {...partOf(part.item)}>
      <span {...partOf(part.accessKeysItemText)}>
        <span {...partOf(part.itemTitle)}>{name}</span>
        <SentenceWithValue sentence={t("access-keys.row.prefix", { prefix: start })} value={start}>
          {(value) => <span {...partOf(part.accessKeysPrefix)}>{value}</span>}
        </SentenceWithValue>
        <span {...partOf(part.muted)}>{mayText(item.choice, item.permissions?.length ?? 0)}</span>
        <span {...partOf(part.muted)}>{whereText(item.projects)}</span>
        <RunsOut item={item} />
        <span {...partOf(part.muted)}>
          {used === undefined || used === null
            ? t("access-keys.row.never-used")
            : t("access-keys.row.last-used", { when: formatDateTime(used) })}
        </span>
      </span>
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
