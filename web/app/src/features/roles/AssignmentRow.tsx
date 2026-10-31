import { t } from "@bugaboo/messages";
import { Button, Icon, iconName, partOf, part } from "@bugaboo/ui";

import { appliesToText, placeText, roleText } from "../../shared/access-words.ts";
import { detailsText } from "../../shared/list-text.ts";
import type { RowLine } from "./assignments.ts";

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

/**
 * What the row says, in one line of facts that each start a sentence: the role (or *No access*), where it counts, and
 * for a row from above where it comes from.
 */
function whatText(row: RowLine): string {
  // A row that comes from the whole organization counts everywhere: saying so twice says nothing more.
  const fromEverywhere = row.inherited && row.place.kind === "organization";
  return detailsText([
    roleText(row.role),
    ...(fromEverywhere ? [] : [appliesToText(row.appliesTo)]),
    ...(row.inherited ? [fromText(row)] : []),
  ]);
}

/** The way to take a row away; the page asks first when it would take the person's own way to manage roles. */
function TakeAway({ row, onRemove }: Pick<AssignmentRowProps, "row" | "onRemove">) {
  return (
    <Button
      aria-label={t("assignments.remove-name", { who: row.who.name, what: roleText(row.role) })}
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
    <li {...partOf(part.item)}>
      <span {...partOf(part.itemText)}>
        <span {...partOf(part.itemTitle)}>
          {/* A group is a set of people, and does not look like one person (M5-F135). */}
          {row.who.kind === "group" ? <Icon name={iconName.groups} /> : null}
          {who}
        </span>
        <span {...partOf(row.role === null ? part.rolesNoAccess : part.muted)}>
          {whatText(row)}
        </span>
        {row.place.kind === "hidden" && !row.inherited ? (
          <span {...partOf(part.muted)}>{placeText(row.place)}</span>
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
