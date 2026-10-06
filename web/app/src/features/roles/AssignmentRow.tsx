import { t } from "@bugaboo/messages";
import { Button, ConfirmDialog } from "@bugaboo/ui";

import { appliesToText, placeText, roleText } from "../../shared/access-words.ts";
import { useMe } from "../../shared/use-me.ts";
import type { RowLine } from "./assignments.ts";
import styles from "./Roles.module.css";

export interface AssignmentRowProps {
  readonly row: RowLine;
  /** Takes the row away (the screen offers *Undo*). */
  readonly onRemove: (row: RowLine) => void;
  /** For a row that comes from above, on a project: starts a row for this project for the same person or group. */
  readonly onChangeHere?: ((row: RowLine) => void) | undefined;
}

/** Where a row that comes from above comes from: *from ACME*, *from the whole organization*. */
function fromText(row: RowLine): string {
  if (row.place.kind === "project") {
    return t("roles.from-project", { name: row.place.name });
  }
  return t(row.place.kind === "organization" ? "roles.from-organization" : "roles.from-hidden");
}

/** What the row says, in one line: the role (or *No access*) and where it counts. */
function whatText(row: RowLine): string {
  const where = row.inherited
    ? `${appliesToText(row.appliesTo)}, ${fromText(row)}`
    : appliesToText(row.appliesTo);
  return `${roleText(row.role)} · ${where}`;
}

/** The way to take a row away: a question first when the row is the person's own. */
function TakeAway({ row, onRemove }: Pick<AssignmentRowProps, "row" | "onRemove">) {
  const me = useMe();
  const name = t("assignments.remove-name", { who: row.who.name, what: roleText(row.role) });
  if (row.who.kind === "person" && me !== undefined && row.who.id === me.id) {
    return (
      <ConfirmDialog
        triggerLabel={t("assignments.remove")}
        triggerName={name}
        title={t("assignments.own-title")}
        text={t("assignments.own-text")}
        confirmLabel={t("assignments.remove")}
        cancelLabel={t("common.cancel")}
        danger
        onConfirm={() => {
          onRemove(row);
        }}
      />
    );
  }
  return (
    <Button
      aria-label={name}
      onPress={() => {
        onRemove(row);
      }}
    >
      {t("assignments.remove")}
    </Button>
  );
}

/**
 * One row of *Who has which role*: whom it is for, the role (or *No access*), where it counts, and
 * what can be done with it. A row from a parent project says where it comes from and is changed for
 * this project by making a row here; a row of this place is taken away.
 */
export function AssignmentRow({ row, onRemove, onChangeHere }: AssignmentRowProps) {
  const who =
    row.who.kind === "group" ? t("assignments.who-group", { name: row.who.name }) : row.who.name;
  return (
    <li className={styles.item}>
      <span className={styles.itemText}>
        <span className={styles.itemTitle}>{who}</span>
        <span className={row.role === null ? styles.noAccess : styles.muted}>{whatText(row)}</span>
        {row.place.kind === "hidden" ? (
          <span className={styles.muted}>{placeText(row.place)}</span>
        ) : null}
      </span>
      {row.inherited ? (
        onChangeHere === undefined ? null : (
          <Button
            aria-label={t("assignments.change-here-name", { who: row.who.name })}
            onPress={() => {
              onChangeHere(row);
            }}
          >
            {t("assignments.change-here")}
          </Button>
        )
      ) : (
        <TakeAway row={row} onRemove={onRemove} />
      )}
    </li>
  );
}
