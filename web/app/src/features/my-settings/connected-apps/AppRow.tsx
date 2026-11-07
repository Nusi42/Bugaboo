import { t } from "@bugaboo/messages";
import { buttonLook, ConfirmDialog, partOf, part } from "@bugaboo/ui";

import { formatDate, formatDateTime } from "../../../shared/format.ts";
import { mayText, whereText } from "../access-keys/key-words.ts";
import type { ConnectedApp } from "./connected-apps.ts";

/** A word of the HTML standard, not text for people. */
const rowHeader = "row";

export interface AppRowProps {
  readonly item: ConnectedApp;
  /** Take this app back: it leaves the list at once and the server is asked in the background. */
  readonly onTakeBack: () => void;
}

interface CellProps {
  /** A cell of a date stays on one line. */
  readonly date?: boolean;
  /** What the cell is, said beside its words on a phone, where the head of the table is not shown. */
  readonly label: string;
  readonly children: string;
}

/** A cell of the table. */
function Cell({ label, date = false, children }: CellProps) {
  return (
    <td {...partOf(part.appsCell, { variants: [date && "date"] })}>
      <span {...partOf(part.appsLabel)}>{label} </span>
      {children}
    </td>
  );
}

/** One app the person allowed, as a row: its name, what it may do and where, when, and *Take back*. */
export function AppRow({ item, onTakeBack }: AppRowProps) {
  const name = item.name ?? "";
  const allowed = item.allowed_at;
  const used = item.last_used_at;
  return (
    <tr {...partOf(part.appsRow)}>
      <th scope={rowHeader} {...partOf(part.appsCell, { variants: ["name"] })}>
        <span {...partOf(part.startColumn)}>
          <span {...partOf(part.itemTitle)}>{name}</span>
          <span {...partOf(part.muted)}>
            {item.on_this_computer === true
              ? t("connected-apps.row.where-computer")
              : t("connected-apps.row.where-web")}
          </span>
        </span>
      </th>
      <Cell label={t("connected-apps.col.may")}>
        {mayText(
          item.scope === "see-only" || item.scope === "see-and-change" ? item.scope : null,
          0,
        )}
      </Cell>
      <Cell label={t("connected-apps.col.projects")}>{whereText(item.projects)}</Cell>
      <Cell label={t("connected-apps.col.allowed")} date>
        {allowed === undefined || allowed === null ? "" : formatDate(allowed)}
      </Cell>
      <Cell label={t("connected-apps.col.last-used")} date>
        {used === undefined || used === null
          ? t("connected-apps.row.never-used")
          : formatDateTime(used)}
      </Cell>
      <td {...partOf(part.appsCell, { variants: ["action"] })}>
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
      </td>
    </tr>
  );
}
