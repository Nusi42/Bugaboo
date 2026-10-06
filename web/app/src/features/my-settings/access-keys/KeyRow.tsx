import { t } from "@bugaboo/messages";
import { ConfirmDialog } from "@bugaboo/ui";

import { formatDate, formatDateTime } from "../../../shared/format.ts";
import styles from "./AccessKeys.module.css";
import type { AccessKey } from "./access-keys.ts";
import { choiceName } from "./choice.ts";

export interface KeyRowProps {
  readonly item: AccessKey;
  /** Delete this key: it leaves the list at once and the server is asked in the background. */
  readonly onDelete: () => void;
}

/** What the key may do, in words: the choice, or how many single permissions. */
function mayText(item: AccessKey): string {
  const { choice, permissions } = item;
  if (choice === undefined || choice === null) {
    return "";
  }
  return choice === "permissions"
    ? t("access-keys.row.permissions", { count: permissions?.length ?? 0 })
    : t(choiceName(choice));
}

/** Where the key works, in words. */
function whereText(item: AccessKey): string {
  const projects = item.projects ?? [];
  if (projects.length === 0) {
    return t("access-keys.row.projects-all");
  }
  const names = projects.map((project) =>
    project.shown === "visible"
      ? t("access-keys.project-label", {
          short_name: project.short_name ?? "",
          name: project.name ?? "",
        })
      : t("access-keys.row.hidden-project"),
  );
  return t("access-keys.row.projects-some", { names: names.join(", ") });
}

/** When the key runs out, or ran out. */
function RunsOut({ item }: { readonly item: AccessKey }) {
  const day = item.expires_at;
  if (day === undefined || day === null) {
    return null;
  }
  const when = formatDate(day);
  return item.expired === true ? (
    <span className={styles.expired}>{t("access-keys.row.expired", { when })}</span>
  ) : (
    <span className={styles.muted}>{t("access-keys.row.expires", { when })}</span>
  );
}

/** One key of the person: its name, what it may do and where, when it runs out, and *Delete*. */
export function KeyRow({ item, onDelete }: KeyRowProps) {
  const name = item.name ?? "";
  const used = item.last_used_at;
  return (
    <li className={styles.item}>
      <span className={styles.itemText}>
        <span className={styles.itemTitle}>{name}</span>
        <span className={styles.prefix}>
          {t("access-keys.row.prefix", { prefix: `bb_${item.prefix ?? ""}` })}
        </span>
        <span className={styles.muted}>{mayText(item)}</span>
        <span className={styles.muted}>{whereText(item)}</span>
        <RunsOut item={item} />
        <span className={styles.muted}>
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
        danger
        onConfirm={onDelete}
      />
    </li>
  );
}
