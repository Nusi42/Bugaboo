import { t } from "@bugaboo/messages";
import {
  buttonLook,
  ConfirmDialog,
  DataCell,
  DataRow,
  DataRowHead,
  dataCellVariant,
  partOf,
  part,
} from "@bugaboo/ui";

import { formatDate, formatDateTime } from "../../../shared/format.ts";
import { mayText, whereText } from "../access-keys/key-words.ts";
import type { ConnectedApp } from "./connected-apps.ts";

export interface AppRowProps {
  readonly item: ConnectedApp;
  /** Take this app back: it leaves the list at once and the server is asked in the background. */
  readonly onTakeBack: () => void;
}

/** One app the person allowed, as a row: its name, what it may do and where, when, and *Take back*. */
export function AppRow({ item, onTakeBack }: AppRowProps) {
  const name = item.name ?? "";
  const allowed = item.allowed_at;
  const used = item.last_used_at;
  return (
    <DataRow>
      <DataRowHead>
        <span {...partOf(part.startColumn)}>
          <span {...partOf(part.itemTitle)}>{name}</span>
          <span {...partOf(part.muted)}>
            {item.on_this_computer === true
              ? t("connected-apps.row.where-computer")
              : t("connected-apps.row.where-web")}
          </span>
        </span>
      </DataRowHead>
      <DataCell label={t("connected-apps.col.may")}>
        {mayText(
          item.scope === "see-only" || item.scope === "see-and-change" ? item.scope : null,
          0,
        )}
      </DataCell>
      <DataCell label={t("connected-apps.col.projects")}>{whereText(item.projects)}</DataCell>
      <DataCell label={t("connected-apps.col.allowed")} variant={dataCellVariant.date}>
        {allowed === undefined || allowed === null ? "" : formatDate(allowed)}
      </DataCell>
      <DataCell label={t("connected-apps.col.last-used")} variant={dataCellVariant.date}>
        {used === undefined || used === null
          ? t("connected-apps.row.never-used")
          : formatDateTime(used)}
      </DataCell>
      <DataCell variant={dataCellVariant.action}>
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
      </DataCell>
    </DataRow>
  );
}
