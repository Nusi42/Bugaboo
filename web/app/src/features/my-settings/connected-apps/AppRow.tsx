import { t } from "@bugaboo/messages";
import { buttonLook, ConfirmDialog, partOf, part } from "@bugaboo/ui";

import { formatDateTime } from "../../../shared/format.ts";
import { listText } from "../../../shared/list-text.ts";
import { choiceName } from "../access-keys/choice.ts";
import type { ConnectedApp } from "./connected-apps.ts";

export interface AppRowProps {
  readonly item: ConnectedApp;
  /** Take this app back: it leaves the list at once and the server is asked in the background. */
  readonly onTakeBack: () => void;
}

/** What the app may do, in words. */
function mayText(item: ConnectedApp): string {
  const { scope } = item;
  return scope === "see-only" || scope === "see-and-change" ? t(choiceName(scope)) : "";
}

/** Where the app works, in words: the projects it is limited to, or every project. */
function whereText(item: ConnectedApp): string {
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
  return t("access-keys.row.projects-some", { names: listText(names) });
}

/** One app the person allowed: its name, what it may do and where, when, and *Take back*. */
export function AppRow({ item, onTakeBack }: AppRowProps) {
  const name = item.name ?? "";
  const allowed = item.allowed_at;
  const used = item.last_used_at;
  return (
    <li {...partOf(part.item)}>
      <span {...partOf(part.accessKeysItemText)}>
        <span {...partOf(part.itemTitle)}>{name}</span>
        <span {...partOf(part.muted)}>
          {item.on_this_computer === true
            ? t("connected-apps.row.where-computer")
            : t("connected-apps.row.where-web")}
        </span>
        <span {...partOf(part.muted)}>{mayText(item)}</span>
        <span {...partOf(part.muted)}>{whereText(item)}</span>
        {allowed === undefined || allowed === null ? null : (
          <span {...partOf(part.muted)}>
            {t("connected-apps.row.allowed", { when: formatDateTime(allowed) })}
          </span>
        )}
        <span {...partOf(part.muted)}>
          {used === undefined || used === null
            ? t("connected-apps.row.never-used")
            : t("connected-apps.row.last-used", { when: formatDateTime(used) })}
        </span>
      </span>
      <ConfirmDialog
        triggerLabel={t("connected-apps.take-back")}
        triggerName={t("connected-apps.take-back-name", { name })}
        title={t("connected-apps.take-back-title")}
        text={t("connected-apps.take-back-text")}
        confirmLabel={t("connected-apps.take-back-confirm")}
        cancelLabel={t("common.cancel")}
        confirmLook={buttonLook.danger}
        onConfirm={onTakeBack}
      />
    </li>
  );
}
